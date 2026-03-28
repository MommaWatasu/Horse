use alloc::{
    boxed::Box,
    vec
};

pub struct RingBuffer {
    buf: Box<[u8]>,
    head: usize,
    tail: usize,
    count: usize
}

impl RingBuffer {
    pub fn new(mut cap: usize) -> Self {
        cap = cap.next_power_of_two();
        Self {
            buf: vec![0u8; cap].into_boxed_slice(),
            head: 0,
            tail: 0,
            count: 0
        }
    }

    pub fn capacity(&self) -> usize { self.buf.len() } 

    pub fn push(&mut self, data: &[u8]) -> usize {
        let space = self.capacity() - self.count;
        let n = data.len().min(space);
        for &b in &data[..n] {
            self.buf[self.tail & (self.capacity() - 1)] = b;
            self.tail = self.tail.wrapping_add(1);
        }
        self.count += n;
        n
    }

    pub fn pop(&mut self, out: &mut [u8]) -> usize {
        let n = out.len().min(self.count);
        for b in &mut out[..n] {
            *b = self.buf[self.head & (self.capacity() - 1)];
            self.head = self.head.wrapping_add(1);
        }
        self.count -= n;
        n
    }
}