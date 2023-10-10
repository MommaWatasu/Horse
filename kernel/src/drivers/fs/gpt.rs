use alloc::{
    vec,
    vec::Vec
};

use crate::horse_lib::bytes::*;

use super::core::STORAGE_CONTROLLERS;

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct PartitionTableHeader {
    signature: [u8; 8],
    revision: u32,
    header_size: u32,
    checksum: u32,
    reserved: u32,
    lba: u64,
    alternate_lba: u64,
    first_block: u64,
    last_block: u64,
    guid: [u8; 16],
    part_entry_lba: u64,
    num_entries: u32,
    entry_size: u32,
    array_checksum: u32,
}

impl PartitionTableHeader {
    pub fn validate(&self) -> bool {
        return self.checksum == crc(self)
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct PartitionEntry {
    type_guid: [u8; 16],
    part_guid: [u8; 16],
    start_lba: u64,
    end_lba: u64,
    attributes: u64,
    name: [u8; 72]
}

pub struct GPT {
    header: PartitionTableHeader,
    entries: Vec<PartitionEntry>,
}

impl GPT {
    // TODO: I have to implement process fpr the recovery field
    pub fn new(id: usize) -> Option<Self> {
        let mut header_buf = [0; 512];
        STORAGE_CONTROLLERS.lock()[id].read(&mut header_buf, 1, 512);
        let header = unsafe { *(header_buf.as_mut_ptr() as *mut PartitionTableHeader) };
        if !header.validate() {
            return None
        }

        let mut array_buf = [0; 128*128];
        STORAGE_CONTROLLERS.lock()[id].read(&mut array_buf, 2, header.num_entries as usize * 128);
        let mut entries = vec![];
        for i in 0..header.num_entries {
            entries.push(unsafe { *(array_buf[128*i as usize..128*(i as usize +1)].as_mut_ptr() as *mut PartitionEntry) })
        }
        return Some(Self {
            header,
            entries
        })
    }
}