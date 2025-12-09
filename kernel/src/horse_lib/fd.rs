use alloc::{
    string::{
        String,
        ToString
    },
    vec::Vec,
    vec
};

pub enum OpenFlags {
    RDOnly = 0x00000000,
    WROnly = 0x00000001,
    RDWR = 0x00000002,
    Create = 0x00000100,
}

#[derive(Clone, PartialEq)]
pub struct Path {
    pub path: Vec<String>
}

impl Path {
    pub fn new(full_path: String)  -> Self {
        return Self { path: full_path.split('/').map(|s| s.to_string()).collect() }
    }
    pub fn path_iter(&self) -> Vec<String> {
        return self.path.clone()
    }
}

#[derive(Clone, PartialEq)]
pub struct File {
    pub f_mode: u32,
    pub path: Path
}

impl File {
    pub fn new(f_mode: u32, path: &str) -> Self {
        return Self { f_mode, path: Path::new(String::from(path)) }
    }
}

pub struct FDTable {
    max_fds: usize,
    fd_array: Vec<Option<File>>,
    empty_idx: usize
}

impl FDTable {
    pub const DEFAULT_TABLE: Self = Self {
        max_fds: 0,
        fd_array: Vec::new(),
        empty_idx: 0
    };
    pub fn initialize(&mut self) {
        self.max_fds = 1024;
        self.empty_idx = 3;
        self.fd_array = vec![None; 1024];
        self.fd_array[0] = Some(File::new(OpenFlags::RDOnly as u32, "/dev/stdin"));
        self.fd_array[1] = Some(File::new(OpenFlags::WROnly as u32, "/dev/stdout"));
        self.fd_array[2] = Some(File::new(OpenFlags::WROnly as u32, "/dev/stderr"));
    }
    pub fn new() -> Self {
        let mut fd_array = vec![None; 1024];
        fd_array[0] = Some(File::new(OpenFlags::RDOnly as u32, "/dev/stdin"));
        fd_array[1] = Some(File::new(OpenFlags::WROnly as u32, "/dev/stdout"));
        fd_array[2] = Some(File::new(OpenFlags::WROnly as u32, "/dev/stderr"));
        return Self {
            max_fds: 1024,
            fd_array,
            empty_idx: 3
        }
    }
    fn update_idx(&mut self) {
        for i in self.empty_idx+1..self.max_fds {
            if self.fd_array[i] == None {
                self.empty_idx = i;
                break
            }
        }
        self.empty_idx = self.max_fds;
    }
    pub fn add(&mut self, file: File) -> i32 {
        if self.empty_idx == self.max_fds {
            return -1
        }
        let idx = self.empty_idx;
        self.fd_array[idx] = Some(file);
        self.update_idx();
        return idx as i32
    }
    pub fn remove(&mut self, fd: i32) {
        let idx = fd as usize;
        self.fd_array[idx] = None;
        if idx < self.empty_idx {
            self.empty_idx = idx;
        }
    }
    pub fn get(&self, fd: i32) -> File {
        return self.fd_array[fd as usize].clone().unwrap()
    }
}