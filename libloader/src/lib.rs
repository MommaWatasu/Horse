#![no_std]

use uefi::table::boot::{
    MemoryDescriptor,
    MemoryMapSize,
    MemoryType
};
use core::{
    iter::Iterator,
    slice::from_raw_parts
};

//MemoryMap
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryMap {
    pub buf: *mut MemoryDescriptor,
    pub buf_size: usize,
    pub entry_size: usize,
    count: usize,
}

impl MemoryMap {
    pub fn new(ptr: *mut MemoryDescriptor, mmap_size: MemoryMapSize) -> Self {
        Self {
            buf: ptr as *mut MemoryDescriptor,
            buf_size: mmap_size.map_size,
            entry_size: mmap_size.entry_size,
            count: 0
        }
    }

    pub fn descriptors(&self) -> &[MemoryDescriptor] {
        unsafe { from_raw_parts(self.buf, (self.buf_size/self.entry_size)-1) }
    }
}

impl Iterator for MemoryMap {
    type Item = *mut MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count > self.buf_size / self.entry_size {
            return None;
        }
        let descriptor = (self.buf as usize + self.entry_size * self.count) as *mut MemoryDescriptor;
        self.count += 1;
        return Some(descriptor);
    }
}

pub fn is_available(ty: MemoryType) -> bool {
    ty == MemoryType::BOOT_SERVICES_CODE
    || ty == MemoryType::BOOT_SERVICES_DATA
    || ty == MemoryType::CONVENTIONAL
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
        (self.fb.add(index) as *mut [u8; 3]).write_volatile(value);
    }
}

/// thread-safe FrameBuffer used for Layer Manager
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TSFrameBuffer {
    fb: usize
}

impl TSFrameBuffer {
    pub unsafe fn new(ptr: *mut u8) -> Self {
        return Self {fb: ptr as usize}
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut u8 {
        self.fb as *mut u8
    }

    pub unsafe fn write_byte(&mut self, index: usize, val: u8) {
        self.as_mut_ptr().add(index).write_volatile(val);
    }

    pub unsafe fn write_value(&mut self, index: usize, value: [u8; 3]) {
        (self.as_mut_ptr().add(index) as *mut [u8; 3]).write_volatile(value);
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum PixelFormat {
    Rgb = 0,
    Bgr,
    Bitmask,
    BltOnly,
}

impl Default for PixelFormat {
    fn default() -> Self { Self::Rgb }
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