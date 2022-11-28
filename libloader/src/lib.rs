#![no_std]

use uefi::table::boot::{
    MemoryDescriptor,
    MemoryMapSize
};
use core::slice::from_raw_parts;

//MemoryMap
pub struct MemoryMap {
    buf: *mut MemoryDescriptor,
    buf_size: usize,
    entry_size: usize
}

impl MemoryMap {
    pub fn new(ptr: *mut MemoryDescriptor, mmap_size: MemoryMapSize) -> Self {
        Self {
            buf: ptr as *mut MemoryDescriptor,
            buf_size: mmap_size.map_size,
            entry_size: mmap_size.entry_size
        }
    }

    pub fn descriptors(&self) -> &[MemoryDescriptor] {
        unsafe { from_raw_parts(self.buf, self.buf_size) }
    }
}

//Graphics
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FrameBufferInfo {
    pub fb: *mut u8,
    pub size: usize,
}

impl FrameBufferInfo {
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.fb
    }

    pub fn size(&self) -> usize {
        self.size
    }
    /// Write to th index-th byte of the framebuffer
    ///
    /// # Safety
    /// This is unsafe : no bound check.
    pub unsafe fn write_byte(&mut self, index: usize, val: u8) {
        self.fb.add(index).write_volatile(val);
    }

    /// Write to th index-th byte of the framebuffer
    ///
    /// # Safety
    /// This is unsafe : no bound check.
    pub unsafe fn write_value(&mut self, index: usize, value: [u8; 3]) {
        (self.fb.add(index) as *mut [u8; 3]).write_volatile(value)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum PixelFormat {
    Rgb = 0,
    Bgr,
    Bitmask,
    BltOnly,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct PixelBitmask {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub reserved: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct ModeInfo {
    pub version: u32,
    pub hor_res: u32,
    pub ver_res: u32,
    pub format: PixelFormat,
    pub mask: PixelBitmask,
    pub stride: u32,
}

impl ModeInfo {
    pub fn resolution(&self) -> (usize, usize) {
        (self.hor_res as usize, self.ver_res as usize)
    }
}