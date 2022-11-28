use uefi::{
    prelude::*,
    data_types::CStr16,
    proto::{
        media::{
            file::{
                Directory, File, FileAttribute, FileInfo, FileMode, FileType, RegularFile
            }
        }
    }
};
use alloc::vec::Vec;
use core::ops::DerefMut;

pub fn open_root(bt: &BootServices, handler: Handle) -> Directory {
    let mut sfs = bt.get_image_file_system(handler).expect("can't open filesystem protocol");
    // NOTE: Is it safe? Internally BootServices::get_image_file_system does something similar.
    sfs.deref_mut()
        .open_volume()
        .expect("no simple filesystem protocol")
}

fn create(dir: &mut Directory, filename: &CStr16, is_dir: bool) -> FileType {
    let attr = if is_dir {FileAttribute::DIRECTORY} else {FileAttribute::empty()};
    dir.open(filename, FileMode::CreateReadWrite, attr)
        .expect("failed to open file")
        .into_type()
        .unwrap()
}

fn open(dir: &mut Directory, filename: &CStr16) -> FileType {
    dir.open(filename, FileMode::Read, FileAttribute::READ_ONLY)
        .expect("failed to open file")
        .into_type()
        .unwrap()
}

pub fn create_file(dir: &mut Directory, filename: &CStr16) -> RegularFile {
    match create(dir, filename, false) {
        FileType::Regular(file) => file,
        FileType::Dir(_)=>panic!("{} isn't a regular file!", filename)
    }
}

pub fn open_file(dir: &mut Directory, filename: &CStr16) -> RegularFile {
    match open(dir, filename) {
        FileType::Regular(file) => file,
        FileType::Dir(_)=>panic!("{} isn't a regular file!", filename)
    }
}

pub fn read_file_to_vec(file: &mut RegularFile) -> Vec<u8> {
    let size = file.get_boxed_info::<FileInfo>().unwrap().file_size() as usize;
    let mut buf = vec![0; size];
    file.read(&mut buf).unwrap();
    buf
}

#[macro_export]
macro_rules! fwrite {
    ($file:expr, $format:tt $( $rest:tt )*) => {
        $file.write(format!($format $( $rest )*).as_bytes()).unwrap()
    };
}

#[macro_export]
macro_rules! fwriteln {
    ($file:expr, $format:tt $( $rest:tt )*) => {
        fwrite!($file, concat!($format, "\n") $( $rest )*)
    };
}