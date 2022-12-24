use crate::{
    memory_manager::*,
    StatusCode,
    status_log
};

use core::{
    alloc::{
        GlobalAlloc,
        Layout
    },
    ptr
};
use spin::mutex::Mutex;

enum AllocateMode {
    Block(usize),
    Frame(usize)
}

impl From<Layout> for AllocateMode {
    fn from(l: Layout) -> Self {
        let size = l.size().max(l.align());
        match BLOCK_SIZES.iter().position(|s| *s >= size) {
            Some(index) => Self::Block(index),
            None => Self::Frame((size + BYTES_PER_FRAME-1) / BYTES_PER_FRAME)
        }
    }
}

const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

pub struct KernelMemoryAllocator {
    available_blocks: Mutex<[*mut u8; BLOCK_SIZES.len()]>
}

impl KernelMemoryAllocator {
    pub const fn new() -> Self {
        Self{ available_blocks: Mutex::new([ptr::null_mut(); BLOCK_SIZES.len()]) }
    }

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
            let next = if i == n_blocks_per_frame-1 {
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
            },
            AllocateMode::Frame(n_frames) => match frame_manager_instance().allocate(n_frames) {
                Ok(frame) => { frame.phys_addr() },
                Err(status) => {
                    status_log!(status, "KernelAllocator failed to allocate frame");
                    ptr::null_mut()
                }
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match layout.into() {
            AllocateMode::Block(index) => {
                let mut available_blocks = self.available_blocks.lock();
                let next = available_blocks[index];
                (ptr as *mut u64).write(next as u64);
                available_blocks[index] = ptr;
            },
            AllocateMode::Frame(n_frames) => {
                frame_manager_instance().free(FrameID::from_phys_addr(ptr), n_frames);
            }
        }
    }
}