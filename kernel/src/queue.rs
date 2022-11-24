use core::{
    marker::Copy,
    mem::MaybeUninit
};
use crate::{
    debug,
    StatusCode
};

#[derive(Debug)]
pub struct ArrayQueue<T, const N: usize> {
    data: MaybeUninit<[T; N]>,
    read_pos: usize,
    write_pos: usize,
    pub count: usize,
    pub capacity: usize
}

impl<T: Copy, const N: usize> ArrayQueue<T, N> {
    pub const fn new() -> Self {
        return Self{
            data: MaybeUninit::uninit(),
            read_pos: 0,
            write_pos: 0,
            count: 0,
            capacity: N
        }
    }

    pub fn initialize(&mut self, value: T) {
        self.data.write([value; N]);
    }

    pub fn push(&mut self, value: T) -> StatusCode {
        if self.count == self.capacity {
            return StatusCode::Full;
        }
        let data = unsafe { &mut *self.data.as_mut_ptr() };
        data[self.write_pos] = value;
        self.count += 1;
        self.write_pos += 1;
        if self.write_pos == self.capacity {
            self.write_pos = 0;
        }
        return StatusCode::Success;
    }

    pub fn pop(&mut self) -> Result<T, &'static str> {
        if self.count == 0 {
            return Err("The queue is empty")
        }
        let value: T = unsafe { self.data.assume_init()[self.read_pos] };
        self.count-=1;
        self.read_pos+=1;
        if self.read_pos == self.capacity {
            self.read_pos = 0;
        }
        return Ok(value)
    }
}