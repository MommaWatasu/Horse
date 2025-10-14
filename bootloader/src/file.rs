use uefi::{
    boot::{
        get_image_file_system,
        image_handle,
    },
    data_types::CStr16, fs::{
        Path,
    },
    fs::FileSystem
};
use alloc::string::String;

pub struct FileBuffer {
    buffer: String
}

impl FileBuffer {
    pub fn new() -> Self {
        return Self {
            buffer: String::new()
        }
    }

    pub fn write(&mut self, content: &str) {
        self.buffer = format!("{}{}", self.buffer, content);
    }

    pub fn writeln(&mut self, content: &str) {
        self.write(&format!("{}{}", content, "\n"))
    }

    pub fn flush(self, fs: &mut FileSystem, path: &CStr16) {
        fs.write(Path::new(path), self.buffer.as_bytes()).expect("can't write file");
    }
}

pub fn open_root() -> FileSystem {
    FileSystem::new(get_image_file_system(image_handle()).expect("can't open filesystem protocol"))
}

#[macro_export]
macro_rules! fwrite {
    ($file:expr, $format:tt $( $rest:tt )*) => {
        $file.write(format!($format $( $rest )*))
    };
}

#[macro_export]
macro_rules! fwriteln {
    ($file:expr, $format:tt $( $rest:tt )*) => {
        fwrite!($file, concat!($format, "\n") $( $rest )*)
    };
}