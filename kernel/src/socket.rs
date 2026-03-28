use alloc::{
    string::String,
    vec::Vec,
    sync::Arc,
};
use spin::Mutex;

use crate::horse_lib::{
    fd::FileDescriptor,
    ringbuffer::*,
};
use crate::sync::WaitQueue;

pub const SOCKET_RING_BUFFER_SIZE: usize = 64 * 1024;
pub static mut GLOBAL_SOCKET_TABLE: Mutex<SocketTable> = Mutex::new(SocketTable::new());

pub struct Socket {
    pub state: Mutex<SocketState>,
    pub accept_queue: Mutex<Vec<Socket>>,
    pub wait_queue: Arc<Mutex<WaitQueue>>,
    pub read_buf: Option<Arc<Mutex<RingBuffer>>>,
    pub write_buf: Option<Arc<Mutex<RingBuffer>>>,
    pub peer_wait_queue: Option<Arc<Mutex<WaitQueue>>>
}

impl Socket {
    pub fn new(_domain: i32, _socket_type: i32) -> Self {
        Self {
            state: Mutex::new(SocketState::Created),
            accept_queue: Mutex::new(Vec::new()),
            wait_queue: Arc::new(Mutex::new(WaitQueue::new())),
            read_buf: None,
            write_buf: None,
            peer_wait_queue: None,
        }
    }
    pub fn set_state(&mut self, state: SocketState) {
        *self.state.lock() = state;
    }
    pub fn set_buffer(&mut self, a2b: Arc<Mutex<RingBuffer>>, b2a: Arc<Mutex<RingBuffer>>) {
        self.read_buf = Some(a2b);
        self.write_buf = Some(b2a);
    }
}
impl FileDescriptor for Socket {
    fn read(&self, buf: &mut [u8]) -> isize {
        loop {
            if let Some(read_buf) = &self.read_buf {
                let n = read_buf.lock().pop(buf);
                if n > 0 {
                    return n as isize;
                }
            }

            self.wait_queue.lock().wait();
        }
    }
    fn write(&self, buf: &[u8]) -> isize {
        let mut n: isize = 0;
        if let Some(write_buf) = &self.write_buf {
            n = write_buf.lock().push(buf) as isize;
        }

        if let Some(peer_wq) = &self.peer_wait_queue {
            peer_wq.lock().wake();
        }

        n
    }
    fn close(&self) {}
}


#[derive(PartialEq, Eq)]
pub enum SocketState {
    Created,
    Bound(String),
    Listening,
    Connected
}

pub struct SocketAddrUn {
    pub sun_family: u16,
    // 108 is the MAX UNIX path size
     pub sun_path: [u8; 108]
}

pub struct SocketTable {
    entries: Vec<(String, Arc<Socket>)>,
}

impl SocketTable {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new()
        }
    }

    pub fn insert(&mut self, path: String, socket: Arc<Socket>) {
        let idx = self.entries.partition_point(|(k, _)| k < &path);
        self.entries.insert(idx, (path, socket));
    }

    pub fn get(&self, path: &str) -> Option<Arc<Socket>> {
        let idx = self.entries.partition_point(|(k, _)| k.as_str() < path);
        if self.entries.get(idx).map(|(k, _)| k.as_str()) == Some(path) {
            Some(self.entries[idx].1.clone())
        } else {
            None
        }
    }

    pub fn remove(&mut self, path: &str) {
        if let Some(idx) = self.entries.iter().position(|(k, _)| k == path) {
            self.entries.remove(idx);
        }
    }
}