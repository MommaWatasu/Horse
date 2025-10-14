use crate::{MemoryMap, StatusCode};
use core::{marker::Sync, mem::size_of};
use libloader::is_available;
use spin::mutex::{Mutex, MutexGuard};

type MapLineType = usize;
const UEFI_PAGE_SIZE: u64 = 4096;
pub const BYTES_PER_FRAME: usize = 4096; //4KiB
                                         //TODO: find the cause of crash when the MAX_PHYSICS_MEMORY_BYTES set to more than 16GB
const MAX_PHYSICS_MEMORY_BYTES: usize = 128 * 1024 * 1024 * 1024; //16GiB
const FRAME_COUNT: usize = MAX_PHYSICS_MEMORY_BYTES / BYTES_PER_FRAME;
const BITS_PER_MAP_LINE: usize = 8 * size_of::<MapLineType>(); //8 * sizeof::<MapLineType>
const MAP_LINE_COUNT: usize = FRAME_COUNT / BITS_PER_MAP_LINE;

#[derive(Clone, Copy, PartialEq)]
pub struct FrameID(usize);

impl FrameID {
    const MIN: Self = Self(0);
    const MAX: Self = Self(FRAME_COUNT);
    pub fn new(id: usize) -> Self {
        Self(id)
    }
    pub fn phys_addr(&self) -> *mut u8 {
        (self.id() * BYTES_PER_FRAME) as *mut u8
    }
    pub fn from_phys_addr(ptr: *mut u8) -> Self {
        Self(ptr as usize / BYTES_PER_FRAME)
    }
    fn id(&self) -> usize {
        self.0
    }
}

static MEMORY_MANAGER: Mutex<BitmapMemoryManager> = Mutex::new(BitmapMemoryManager::new());
pub fn frame_manager_instance() -> MutexGuard<'static, BitmapMemoryManager> {
    MEMORY_MANAGER.lock()
}

pub struct BitmapMemoryManager {
    alloc_map: [MapLineType; MAP_LINE_COUNT],
    range_begin: FrameID,
    range_end: FrameID,
}

unsafe impl Sync for BitmapMemoryManager {}

impl BitmapMemoryManager {
    pub const fn new() -> Self {
        Self {
            alloc_map: [0; MAP_LINE_COUNT],
            range_begin: FrameID::MIN,
            range_end: FrameID::MAX,
        }
    }

    pub fn initialize(&mut self, memory_map: MemoryMap) {
        let mut available_end: u64 = 0;
        for desc in memory_map.descriptors() {
            if available_end < desc.phys_start {
                self.mark_allocated(
                    FrameID::new(available_end as usize / BYTES_PER_FRAME),
                    (desc.phys_start - available_end) as usize / BYTES_PER_FRAME,
                );
            }

            let phys_end = desc.phys_start + desc.page_count * UEFI_PAGE_SIZE;
            if is_available(desc.ty) {
                available_end = phys_end;
            } else {
                self.mark_allocated(
                    FrameID::new(desc.phys_start as usize / BYTES_PER_FRAME),
                    (desc.page_count * UEFI_PAGE_SIZE) as usize / BYTES_PER_FRAME,
                );
            }
        }
        self.set_memory_range(
            FrameID::new(1),
            FrameID::new(available_end as usize / BYTES_PER_FRAME),
        );
    }

    pub fn allocate(&mut self, n_frames: usize) -> Result<FrameID, StatusCode> {
        let mut start_frame_id = self.range_begin.id();
        let mut i: usize;
        loop {
            i = 0;
            while i < n_frames {
                if start_frame_id + i >= self.range_end.id() {
                    return Err(StatusCode::NoEnoughMemory);
                }
                if self.get_bit(FrameID::new(start_frame_id + i)) {
                    break;
                }
                i += 1;
            }
            if i == n_frames {
                self.mark_allocated(FrameID::new(start_frame_id), n_frames);
                return Ok(FrameID::new(start_frame_id));
            }
            start_frame_id += i + 1;
        }
    }

    pub fn free(&mut self, start_frame: FrameID, n_frames: usize) -> StatusCode {
        self.set_bit_range(start_frame, n_frames, false);
        return StatusCode::Success;
    }

    pub fn mark_allocated(&mut self, start_frame: FrameID, n_frames: usize) {
        self.set_bit_range(start_frame, n_frames, true);
    }

    pub fn set_memory_range(&mut self, range_begin: FrameID, range_end: FrameID) {
        self.range_begin = range_begin;
        self.range_end = range_end;
    }

    fn get_bit(&self, frame: FrameID) -> bool {
        let line_idx = frame.id() / BITS_PER_MAP_LINE;
        let bit_idx = frame.id() % BITS_PER_MAP_LINE;

        return (self.alloc_map[line_idx] & (1 as MapLineType) << bit_idx) != 0;
    }

    /// Efficiently set a range of bits at once using bit masks
    /// This is much faster than calling set_bit in a loop
    fn set_bit_range(&mut self, start_frame: FrameID, n_frames: usize, allocated: bool) {
        if n_frames == 0 {
            return;
        }

        let start_id = start_frame.id();
        let end_id = start_id.checked_add(n_frames)
            .expect("set_bit_range: start_id + n_frames overflowed");
        
        // Bounds checking
        if end_id > FRAME_COUNT {
            panic!(
                "set_bit_range: end_id {} exceeds FRAME_COUNT {}. start_id={}, n_frames={}",
                end_id, FRAME_COUNT, start_id, n_frames
            );
        }
        
        let start_line = start_id / BITS_PER_MAP_LINE;
        let end_line = (end_id - 1) / BITS_PER_MAP_LINE;
        let start_bit = start_id % BITS_PER_MAP_LINE;
        let end_bit = (end_id - 1) % BITS_PER_MAP_LINE;
        
        // Additional bounds checking
        if start_line >= MAP_LINE_COUNT || end_line >= MAP_LINE_COUNT {
            panic!(
                "set_bit_range: line index out of bounds. start_line={}, end_line={}, MAP_LINE_COUNT={}",
                start_line, end_line, MAP_LINE_COUNT
            );
        }

        if start_line == end_line {
            // All bits are in the same map line
            let mask = self.create_mask(start_bit, end_bit + 1);
            if allocated {
                self.alloc_map[start_line] |= mask;
            } else {
                self.alloc_map[start_line] &= !mask;
            }
        } else {
            // Bits span multiple map lines
            
            // Handle the first partial line
            let first_mask = self.create_mask(start_bit, BITS_PER_MAP_LINE);
            if allocated {
                self.alloc_map[start_line] |= first_mask;
            } else {
                self.alloc_map[start_line] &= !first_mask;
            }

            // Handle complete lines in the middle
            for line in (start_line + 1)..end_line {
                self.alloc_map[line] = if allocated { !0 } else { 0 };
            }

            // Handle the last partial line
            let last_mask = self.create_mask(0, end_bit + 1);
            if allocated {
                self.alloc_map[end_line] |= last_mask;
            } else {
                self.alloc_map[end_line] &= !last_mask;
            }
        }
    }

    /// Create a bit mask for bits from start_bit (inclusive) to end_bit (exclusive)
    #[inline]
    fn create_mask(&self, start_bit: usize, end_bit: usize) -> MapLineType {
        debug_assert!(start_bit < BITS_PER_MAP_LINE, "start_bit must be < BITS_PER_MAP_LINE");
        debug_assert!(end_bit <= BITS_PER_MAP_LINE, "end_bit must be <= BITS_PER_MAP_LINE");
        debug_assert!(start_bit < end_bit, "start_bit must be < end_bit");
        
        if end_bit >= BITS_PER_MAP_LINE {
            // All bits from start_bit to the end of the line
            !0 << start_bit
        } else {
            let num_bits = end_bit - start_bit;
            // Prevent shift overflow: if num_bits == BITS_PER_MAP_LINE, use !0
            if num_bits >= BITS_PER_MAP_LINE {
                !0
            } else {
                (((1 as MapLineType) << num_bits) - 1) << start_bit
            }
        }
    }

    pub fn check_free_memory(&self) -> usize {
        let mut count: usize = 0;
        for i in self.range_begin.id()..self.range_end.id() {
            if !self.get_bit(FrameID::new(i)) {
                count += 1
            }
        }
        return count * BYTES_PER_FRAME;
    }
}
