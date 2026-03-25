use alloc::{
    boxed::Box,
    vec::Vec
};
use spin::Mutex;

use crate::{
    horse_lib::{
        fd::{FDTable, Path},
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

pub trait FileSystem: Send + Sync {
    fn exists(&self, path: &Path) -> bool;
    fn read_file(&self, path: &Path, buf: &mut [u8], nbytes: usize) -> isize;
}
