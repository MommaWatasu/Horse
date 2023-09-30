use alloc::{
    boxed::Box,
    vec::Vec
};
use spin::Mutex;

use crate::{
    error,
    lib::{
        bytes::bytes2str,
        storage::Storage
    }
};
use super::{
    core::FileSystem,
    fat::core::{
        BPB,
        FAT,
    },
    gpt::GPT
};

static mut FILESYSTEM_TABLE: Mutex<Vec<&mut dyn FileSystem>> = Mutex::new(Vec::new());

pub fn initialize_storage<T: Storage>(storage: &mut T, id: usize) {
    match GPT::new(storage) {
        Some(gpt) => {
            initialize_gpt(gpt, id)
        }
        None => unsafe {
            match initialize_partition(storage, id) {
                Some(fs) => {
                    FILESYSTEM_TABLE.lock().push(&mut *Box::into_raw(fs))
                },
                None => { error!("this partition has not supported file system") }
            }
        }
    }
}

fn initialize_gpt(gpt: GPT, id: usize) {
}

fn initialize_partition<T: Storage>(storage: &mut T, id: usize) -> Option<Box<dyn FileSystem>> {
    let mut buf = [0; 512];
    storage.read(&mut buf, 0, 512);
    let bpb = unsafe { *(buf.as_mut_ptr() as *mut BPB) };
    if &bytes2str(&bpb.fil_sys_type)[0..3] == "FAT" {
        return Some(Box::new(FAT::new(bpb, id)))
    } else {
        return None
    }
}

