use alloc::{
    string::{String, ToString},
    vec::Vec,
    vec,
    sync::Arc,
};
use crate::socket::*;

pub trait FileDescriptor: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> isize;
    fn write(&self, buf: &[u8]) -> isize;
    fn close(&self);
}

#[derive(Clone, PartialEq)]
pub struct Path {
    pub path: Vec<String>
}

impl Path {
    pub fn new(full_path: String) -> Self {
        let path: Vec<String> = full_path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Self { path }
    }
    pub fn path_iter(&self) -> Vec<String> {
        self.path.clone()
    }
}

pub struct FDTable {
    max_fds: usize,
    fd_array: Vec<Option<Arc<dyn FileDescriptor>>>,
    socket_array: Vec<Option<Arc<Socket>>>,
    empty_idx: usize,
}

impl FDTable {
    const MAX_FD: usize = 1024;
    pub const DEFAULT_TABLE: Self = Self {
        max_fds: 0,
        fd_array: Vec::new(),
        socket_array: Vec::new(),
        empty_idx: 0,
    };
    pub fn initialize(&mut self) {
        self.max_fds = Self::MAX_FD;
        self.empty_idx = 0;
        self.fd_array = vec![None; Self::MAX_FD];
        self.socket_array = vec![None; Self::MAX_FD]
    }
    fn update_idx(&mut self) {
        for i in self.empty_idx + 1..self.max_fds {
            if self.fd_array[i].is_none() && self.socket_array[i].is_none() {
                self.empty_idx = i;
                return;
            }
        }
        self.empty_idx = self.max_fds;
    }
    pub fn add(&mut self, file: Arc<dyn FileDescriptor>) -> i32 {
        if self.empty_idx == self.max_fds {
            return -1;
        }
        let idx = self.empty_idx;
        self.fd_array[idx] = Some(file);
        self.update_idx();
        idx as i32
    }
    pub fn add_socket(&mut self, socket: Arc<Socket>) -> i32 {
        if self.empty_idx == self.max_fds {
            return -1;
        }
        let idx = self.empty_idx;
        self.socket_array[idx] = Some(socket);
        self.update_idx();
        idx as i32
    }
    pub fn remove(&mut self, fd: i32) {
        let idx = fd as usize;
        if let Some(entry) = self.fd_array[idx].take() {
            entry.close();
            self.fd_array[idx] = None;
        }
        if idx < self.empty_idx {
            self.empty_idx = idx;
        }
    }
    pub fn remove_socket(&mut self, fd: i32) {
        let idx = fd as usize;
        if let Some(entry) = self.socket_array[idx].take() {
            entry.close();
            self.fd_array[idx] = None;
        }
        if idx < self.empty_idx {
            self.empty_idx = idx;
        }
    }
    pub fn get(&self, fd: i32) -> Option<Arc<dyn FileDescriptor>> {
        self.fd_array.get(fd as usize)?.as_ref().map(Arc::clone)
    }
    pub fn get_socket(&self, fd: i32) -> Option<Arc<Socket>> {
        self.socket_array.get(fd as usize)?.as_ref().map(Arc::clone)
    }
}
