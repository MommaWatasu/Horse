use crate::{lib::storage::Storage, memory_manager::*};

use alloc::{vec, vec::Vec};

struct VataController {
    data: Vec<u8>,
}

impl VataController {
    fn new() -> Self {
        return Self {
            data: vec![0u8; frame_manager_instance().check_free_memory() / 2], //TODO: adjust memory size considering the size of available memory
        };
    }
}

impl Storage for VataController {
    fn read(&mut self, bytes: u32, lba: u32, buf: &mut [u8]) -> u8 {
        let idx = 512 * lba as usize;
        let idx_end = idx + bytes as usize;
        if idx_end > self.data.len() {
            buf.copy_from_slice(&self.data[idx..idx_end]);
            return 0
        } else {
            return 1
        }
    }
    fn write(&mut self, bytes: u32, lba: u32, buf: &[u8]) -> u8 {
        let idx = 512 * lba as usize;
        let idx_end = idx + bytes as usize;
        if idx_end as usize > self.data.len() {
            self.data[idx..idx_end].copy_from_slice(buf);
            return 0;
        } else {
            return 1;
        }
    }
}
