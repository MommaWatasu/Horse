use crate::{lib::storage::Storage, memory_manager::*, drivers::fs::core::StorageController};

use alloc::{vec, vec::Vec};

pub struct VataController {
    data: Vec<u8>,
}

impl VataController {
    pub fn new() -> Self {
        return Self {
            data: vec![0u8; frame_manager_instance().check_free_memory() / 2], //TODO: adjust memory size considering the size of available memory
        };
    }
}

impl Storage for VataController {
    fn read(&mut self, buf: &mut [u8], lba: u32, nbytes: usize) -> u8 {
        let idx = 512 * lba as usize;
        let idx_end = idx + nbytes;
        if idx_end > self.data.len() {
            buf.copy_from_slice(&self.data[idx..idx_end]);
            return 0
        } else {
            return 1
        }
    }
    fn write(&mut self, buf: &[u8], lba: u32, nbytes: usize) -> u8 {
        let idx = 512 * lba as usize;
        let idx_end = idx + nbytes;
        if idx_end as usize > self.data.len() {
            self.data[idx..idx_end].copy_from_slice(buf);
            return 0;
        } else {
            return 1;
        }
    }
}


impl StorageController for VataController {}