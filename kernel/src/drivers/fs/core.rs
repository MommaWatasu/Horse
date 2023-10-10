use alloc::{
    boxed::Box,
    vec::Vec
};
use spin::Mutex;

use crate::{
    horse_lib::{
        fd::FDTable,
        storage::Storage,
    },
    drivers::ata::{
        pata::IdeController,
        vata::VataController
    }
};

pub enum DiskType {
    Ide { controller: IdeController},
    Vata { controller: VataController },
}

pub trait StorageController: Storage + Send + Sync {}
pub static STORAGE_CONTROLLERS: Mutex<Vec<Box<dyn StorageController>>> = Mutex::new(Vec::new());
pub static FILE_DESCRIPTOR_TABLE: Mutex<FDTable> = Mutex::new(FDTable::DEFAULT_TABLE);

pub trait FileSystem {
    //fn create();
    //fn remove();
    fn open(&self, path: &str, flags: u32) -> i32;
    fn close(&self, fd: i32);
    fn read(&self, fd: i32, buf: &mut [u8], nbytes: usize) -> isize;
}