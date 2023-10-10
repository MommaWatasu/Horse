use alloc::{
    boxed::Box,
    vec::Vec
};
use spin::Mutex;

use crate::{
    error,
    debug,
    horse_lib::{
        bytes::bytes2str,
        storage::Storage
    }
};
use super::{
    core::{FileSystem, STORAGE_CONTROLLERS},
    fat::core::{
        BPB,
        FAT,
    },
    gpt::GPT
};

pub static mut FILESYSTEM_TABLE: Mutex<Vec<Box<dyn FileSystem>>> = Mutex::new(Vec::new());

pub fn initialize_filesystem() {
    let nstorage = STORAGE_CONTROLLERS.lock().len();
    for id in 0..nstorage {
        initialize_storage(id);
    }
}

pub fn initialize_storage(id: usize) {
    match GPT::new(id) {
        Some(gpt) => {
            initialize_gpt(gpt, id)
        }
        None => unsafe {
            match initialize_partition(id) {
                Some(fs) => {
                    FILESYSTEM_TABLE.lock().push(fs)
                },
                None => { error!("this partition has not supported file system") }
            }
        }
    }
}

fn initialize_gpt(gpt: GPT, id: usize) {
}

fn initialize_partition(id: usize) -> Option<Box<dyn FileSystem>> {
    let mut buf = [0; 512];
    STORAGE_CONTROLLERS.lock()[id].read(&mut buf, 0, 512);
    let bpb = unsafe { *(buf.as_mut_ptr() as *mut BPB) };
    let fsys = &bytes2str(&bpb.fil_sys_type);
    if &fsys[0..5] == "FAT32" {
        return Some(Box::new(FAT::new(bpb, id)))
    } else {
        return None
    }
}

