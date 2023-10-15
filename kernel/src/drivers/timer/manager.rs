use super::FFTimer;
use crate::{println, Message, INTERRUPTION_QUEUE};

use alloc::collections::BinaryHeap;
use core::cmp::{Ord, Ordering};

pub struct TimerManager {
    tick: u64,
    timers: BinaryHeap<Timer>,
    fft: FFTimer,
}

impl TimerManager {
    pub fn new(fft: FFTimer) -> Self {
        return Self {
            tick: 0,
            timers: BinaryHeap::new(),
            fft,
        };
    }
    pub fn add_timer(&mut self, timeout: u64, value: i32, periodic: bool) {
        self.timers.push(Timer::new(self.tick, timeout, value, periodic));
    }
    pub fn tick(&mut self) -> bool {
        let mut proc = false;
        self.tick = self.tick.wrapping_add(1);
        loop {
            if let Some(t) = self.timers.peek() {
                if t.absolute_timeout > (self.tick as u128) {
                    break;
                }
                INTERRUPTION_QUEUE.lock().push(Message::TimerTimeout {
                    timeout: t.timeout,
                    value: t.value,
                });
                if t.value == -1 {
                    proc = true;
                }
                if t.periodic != 0 {
                    self.add_timer(t.periodic, t.value, true)
                }
                self.timers.pop();
            } else {
                break;
            }
        }
        return proc
    }
    pub fn wait_seconds(&self, sec: u64) {
        for i in 0..sec {
            self.fft.wait_milliseconds(1000);
        }
    }
}

//Logical Timer
#[derive(Eq)]
struct Timer {
    absolute_timeout: u128,
    pub timeout: u64,
    pub value: i32,
    periodic: u64,
}

impl Timer {
    fn new(tick: u64, timeout: u64, value: i32, periodic: bool) -> Self {
        let relational_timeout;
        if periodic {
            relational_timeout = timeout;
        } else {
            relational_timeout = 0;
        }
        return Self {
            absolute_timeout: (tick as u128) + (timeout as u128),
            timeout: tick.wrapping_add(timeout),
            value,
            periodic: relational_timeout
        };
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return Some(self.cmp(other));
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.absolute_timeout > other.absolute_timeout {
            return Ordering::Less;
        } else if self.timeout == other.timeout {
            return Ordering::Equal;
        } else {
            return Ordering::Greater;
        }
    }
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        return self.timeout == other.timeout;
    }
}
