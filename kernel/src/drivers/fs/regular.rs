use crate::{
    drivers::fs::init::FILESYSTEM_TABLE,
    horse_lib::fd::{FileDescriptor, Path},
};
use alloc::vec;

pub struct RegularFile {
    pub f_mode: u32,
    pub path: Path,
}

impl RegularFile {
    pub fn new(f_mode: u32, path: &str) -> Self {
        Self {
            f_mode,
            path: Path::new(alloc::string::String::from(path)),
        }
    }
}

impl FileDescriptor for RegularFile {
    fn read(&self, buf: &mut [u8]) -> isize {
        let nbytes = buf.len();
        let mut tmp = vec![0u8; nbytes];
        let fs_table = unsafe { FILESYSTEM_TABLE.lock() };
        if fs_table.is_empty() {
            return -5; // EIO
        }
        let bytes_read = fs_table[0].read_file(&self.path, &mut tmp, nbytes);
        if bytes_read > 0 {
            buf[..bytes_read as usize].copy_from_slice(&tmp[..bytes_read as usize]);
        }
        bytes_read
    }
    fn write(&self, _buf: &[u8]) -> isize {
        -5 // EIO - read-only filesystem
    }
    fn close(&self) {}
}
