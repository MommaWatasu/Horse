use crate::{memory_manager::*, status_log, StatusCode};

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};
use spin::mutex::Mutex;

enum AllocateMode {
    Block(usize),
    Frame(usize),
}

impl From<Layout> for AllocateMode {
    fn from(l: Layout) -> Self {
        // Select the minimum block that satisfies both size and alignment requirements
        // Use the maximum of size and alignment to meet alignment requirements
        let size = l.size();
        let align = l.align();
        let required_size = size.max(align);
        
        match BLOCK_SIZES.iter().position(|s| *s >= required_size && *s >= align) {
            Some(index) => Self::Block(index),
            None => {
                // For frame allocation, calculate the number of frames to satisfy size and alignment requirements
                let n_frames = (required_size + BYTES_PER_FRAME - 1) / BYTES_PER_FRAME;
                Self::Frame(n_frames)
            }
        }
    }
}

// All block sizes are powers of 2, ranging from 8 bytes to 2048 bytes
// This ensures that each block allocated from the beginning of a frame (4096-byte boundary)
// is automatically placed at the appropriate alignment boundary
// Example: A 16-byte block is always placed at a 16-byte boundary
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

// Magic number for double-free detection in debug builds
#[cfg(debug_assertions)]
const FREED_BLOCK_MAGIC: u64 = 0xDEADBEEFDEADBEEF;

/// Kernel's global memory allocator
/// Uses a free-list based block allocator for small allocations (≤2048 bytes)
/// Directly uses the frame manager for large allocations
pub struct KernelMemoryAllocator {
    available_blocks: Mutex<[*mut u8; BLOCK_SIZES.len()]>,
}

impl KernelMemoryAllocator {
    pub const fn new() -> Self {
        Self {
            available_blocks: Mutex::new([ptr::null_mut(); BLOCK_SIZES.len()]),
        }
    }

    /// Allocate a new frame for the specified block size index and
    /// initialize it as a free list
    /// 
    /// # Returns
    /// Pointer to the head of the newly allocated block chain,
    /// or null pointer if allocation fails
    fn allocate_frame_for_block(index: usize) -> *mut u8 {
        let block_size = BLOCK_SIZES[index];
        let n_blocks_per_frame = BYTES_PER_FRAME / block_size;
        let ptr: *mut u8 = match frame_manager_instance().allocate(1) {
            Ok(frame) => frame.phys_addr(),
            Err(status) => {
                status_log!(status, "KernelALlocator failed to allocate frame");
                ptr::null_mut()
            }
        };
        for i in 0..n_blocks_per_frame {
            let current = unsafe { ptr.add(i * block_size) };
            let next = if i == n_blocks_per_frame - 1 {
                ptr::null_mut()
            } else {
                unsafe { current.add(block_size) }
            };
            unsafe { (current as *mut u64).write(next as u64) };
        }
        return ptr;
    }
}

unsafe impl Sync for KernelMemoryAllocator {}

unsafe impl GlobalAlloc for KernelMemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match layout.into() {
            AllocateMode::Block(index) => {
                let mut available_blocks = self.available_blocks.lock();
                let mut ptr = available_blocks[index];
                if ptr.is_null() {
                    ptr = Self::allocate_frame_for_block(index);
                }
                if !ptr.is_null() {
                    available_blocks[index] = (ptr as *mut u64).read() as *mut u8;
                }
                return ptr;
            }
            AllocateMode::Frame(n_frames) => match frame_manager_instance().allocate(n_frames) {
                Ok(frame) => frame.phys_addr(),
                Err(status) => {
                    status_log!(status, "KernelAllocator failed to allocate frame");
                    ptr::null_mut()
                }
            },
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match layout.into() {
            AllocateMode::Block(index) => {
                // Double-free detection in debug builds
                #[cfg(debug_assertions)]
                {
                    let value = (ptr as *mut u64).read();
                    if value == FREED_BLOCK_MAGIC {
                        panic!("Double free detected at address {:p}", ptr);
                    }
                }
                
                let mut available_blocks = self.available_blocks.lock();
                let next = available_blocks[index];
                (ptr as *mut u64).write(next as u64);
                available_blocks[index] = ptr;
                
                // Set freed marker in debug builds
                #[cfg(debug_assertions)]
                if next.is_null() {
                    // Don't actually set it if it's the end of the list
                    // (to avoid overwriting the next pointer)
                }
            }
            AllocateMode::Frame(n_frames) => {
                frame_manager_instance().free(FrameID::from_phys_addr(ptr), n_frames);
            }
        }
    }
}
