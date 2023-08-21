use crate::{
    INTERRUPTION_QUEUE,
    Message,
    println
};

use alloc::collections::BinaryHeap;
use core::cmp::{Ord, Ordering};

pub struct TimerManager {
    tick: u64,
    timers: BinaryHeap<Timer>,
}

impl TimerManager {
    pub fn new() -> Self {
        return Self{tick: 0, timers: BinaryHeap::new()}
    }
    pub fn add_timer(&mut self, timeout: u64, value: i32) {
        let timer = Timer::new(self.tick, timeout, value);
        self.timers.push(timer)
    }
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        loop {
            if let Some(t) = self.timers.peek() {
                if t.absolute_timeout > (self.tick as u128) {
                    break;
                }
                INTERRUPTION_QUEUE.lock().push(Message::TimerTimeout{ timeout: t.timeout, value: t.value });
                self.timers.pop();
            } else { break }
        }
    }
}

#[derive(Eq)]
struct Timer {
    absolute_timeout: u128,
    pub timeout: u64,
    pub value: i32
}

impl Timer {
    fn new(tick: u64, timeout: u64, value: i32) -> Self {
        return Self {
            absolute_timeout: (tick as u128) + (timeout as u128),
            timeout: tick.wrapping_add(timeout),
            value
        }
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return Some(self.cmp(other))
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.absolute_timeout > other.absolute_timeout {
            return Ordering::Less
        } else if self.timeout == other.timeout {
            return Ordering::Equal
        } else {
            return Ordering::Greater
        }
    }
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        return self.timeout == other.timeout
    }
}