#![no_std]

use core::{
    slice::from_raw_parts_mut,
    ffi::c_void,
};
use uefi::mem::memory_map::{
    MemoryType,
    MemoryMap,
    MemoryMapOwned,
    MemoryDescriptor
};
use uefi_raw::table::{
    configuration::ConfigurationTable,
    system::SystemTable
};

pub struct ConfigTableEntries {
    entries: &'static [ConfigurationTable],
}

impl ConfigTableEntries {
    pub unsafe fn new(st: SystemTable) -> Self {
        return Self {
            entries: from_raw_parts_mut(
                st.configuration_table as *mut ConfigurationTable,
                st.number_of_configuration_table_entries as usize,
            ),
        };
    }

    pub fn get_by_guid(&self, guid: uefi::Guid) -> Option<*mut c_void> {
        for entry in self.entries {
            if entry.vendor_guid == guid {
                return Some(entry.vendor_table as *mut c_void);
            }
        }
        return None;
    }
}

const MAX_MEMORY_MAP_ENTRIES: usize = 256;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BootMemoryMap {
    descriptors: [MemoryDescriptor; MAX_MEMORY_MAP_ENTRIES],
    entry_count: usize,
}

impl BootMemoryMap {
    /// Creates a new BootMemoryMap from a MemoryMapOwned.
    /// This function copies all memory descriptors from the MemoryMapOwned
    /// to create an independent, copyable structure.
    pub fn new(memmap: MemoryMapOwned) -> Self {
        let mut descriptors = [MemoryDescriptor::default(); MAX_MEMORY_MAP_ENTRIES];
        let mut count = 0;
        
        for (i, entry) in memmap.entries().enumerate() {
            if i >= MAX_MEMORY_MAP_ENTRIES {
                break;
            }
            descriptors[i] = *entry;
            count += 1;
        }
        
        Self {
            descriptors,
            entry_count: count,
        }
    }

    /// Returns an iterator over the memory descriptors.
    pub fn iter(&self) -> core::slice::Iter<'_, MemoryDescriptor> {
        self.descriptors[..self.entry_count].iter()
    }

    /// Returns the number of memory descriptors.
    pub fn len(&self) -> usize {
        self.entry_count
    }

    /// Returns a slice of the memory descriptors.
    pub fn entries(&self) -> &[MemoryDescriptor] {
        &self.descriptors[..self.entry_count]
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