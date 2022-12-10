use crate::StatusCode;
use crate::debug;
use core::mem::size_of;

type MapLineType = u64;
const BYTES_PER_FRAME: usize = 4096;//4KiB
//TODO: find the cause of crash when the MAX_PHYSICS_MEMORY_BYTES set to more than 16GB
const MAX_PHYSICS_MEMORY_BYTES: usize = 16 * 1024 * 1024 * 1024;//16GiB
const FRAME_COUNT: usize = MAX_PHYSICS_MEMORY_BYTES / BYTES_PER_FRAME;
const BITS_PER_MAP_LINE: usize = 8 * size_of::<MapLineType>();//8 * sizeof::<MapLineType>

pub struct FrameID{
    id: usize
}

impl FrameID {
    const BEGIN: Self = Self{ id: 0 };
    const END: Self = Self{ id: FRAME_COUNT };
    pub fn new(id: usize) -> Self { Self{id} }
    pub fn from_u64(id: u64) -> Self {Self{ id: id as usize }}
    fn id(&self) -> usize { self.id }
}
//depends on whether unsgined long is 4-bits or not
pub struct BitmapMemoryManager {
    alloc_map: [MapLineType; FRAME_COUNT/BITS_PER_MAP_LINE],
    range_begin: FrameID,
    range_end: FrameID
}

impl BitmapMemoryManager {
    pub const fn new() -> Self {
        Self {
            alloc_map: [0; FRAME_COUNT/BITS_PER_MAP_LINE],
            range_begin: FrameID::BEGIN,
            range_end: FrameID::END
        }
    }

    pub fn allocate(&mut self, n_frames: usize) -> Result<FrameID, StatusCode> {
        let mut start_frame_id = self.range_begin.id();
        let mut i: usize;
        loop {
            i = 0;
            while i < n_frames {
                i += 1;
                if start_frame_id + i >= self.range_end.id() {
                    return Err(StatusCode::NoEnoughMemory);
                }
                if self.get_bit(FrameID::new(start_frame_id + 1)) {
                    break;
                }
            }
            if i == n_frames {
                self.mark_allocated(FrameID::new(start_frame_id), n_frames);
                return Ok(FrameID::new(start_frame_id));
            }
            start_frame_id += i+1;
        }
    }

    pub fn free(&mut self, start_frame: FrameID, n_frames: usize) -> StatusCode {
        for i in 0..n_frames {
            self.set_bit(FrameID::new(start_frame.id() + i), false);
        }
        return StatusCode::Success;
    }

    pub fn mark_allocated(&mut self, start_frame: FrameID, n_frames: usize) {
        //debug!("start_id: {}, frames: {}", start_frame.id(), n_frames);
        for i in 0..n_frames {
            self.set_bit(FrameID::new(start_frame.id() + i), true);
        }
    }
    
    pub fn set_memory_range(&mut self, range_begin: FrameID, range_end: FrameID) {
        self.range_begin = range_begin;
        self.range_end = range_end;
    }

    fn get_bit(&self, frame: FrameID) -> bool {
        let line_idx = frame.id() / BITS_PER_MAP_LINE;
        let bit_idx = frame.id() % BITS_PER_MAP_LINE;

        return (self.alloc_map[line_idx] & ((1 << bit_idx) as MapLineType)) != 0;
    }

    fn set_bit(&mut self, frame: FrameID, allocated: bool) {
        let line_idx = frame.id() / BITS_PER_MAP_LINE;
        let bit_idx = frame.id() % BITS_PER_MAP_LINE;

        if allocated {
            self.alloc_map[line_idx] |= ((1 as MapLineType) << bit_idx) as MapLineType
        } else {
            self.alloc_map[line_idx] &= ((1 as MapLineType) << bit_idx) as MapLineType
        }
    }
}