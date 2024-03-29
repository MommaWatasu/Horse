pub mod fftimer;
pub mod hpet;
mod ioapic;
mod manager;

use fftimer::*;
use hpet::*;
use manager::*;

use alloc::string::String;
use core::{
    mem::size_of,
    ptr::{read, read_unaligned, write},
};
use spin::{Mutex, Once};

use crate::{error, println, DescriptionHeader, InterruptVector};

const PM_TIMER_FREQ: u32 = 3579545;
const COUNT_MAX: u32 = 1000000;
const LVT_TIMER: *mut u32 = 0xfee00320 as *mut u32;
const INITIAL_COUNT: *mut u32 = 0xfee00380 as *mut u32;
const CURRENT_COUNT: *const u32 = 0xfee00390 as *const u32;
const DIVIDE_CONFIG: *mut u32 = 0xfee003e0 as *mut u32;

static LAPIC_FREQUENCY: Once<u32> = Once::new();
pub static TIMER_MANAGER: Mutex<Once<TimerManager>> = Mutex::new(Once::new());

pub fn initialize_lapic_itmer(fftimer: FFTimer) {
    TIMER_MANAGER
        .lock()
        .call_once(|| TimerManager::new(fftimer));

    unsafe {
        write(DIVIDE_CONFIG, 0b1011); //divide 1:1
        write(LVT_TIMER, 0b001 << 16); //masked, one-shot
    }

    start_lapic_timer();
    fftimer.wait_milliseconds(100);
    let elapsed = lapic_timer_elapsed();
    stop_lapic_timer();
    LAPIC_FREQUENCY.call_once(|| elapsed * 10);

    unsafe {
        write(DIVIDE_CONFIG, 0b1011); //divide 1:1
        write(
            LVT_TIMER,
            (0b010 << 16) | InterruptVector::LAPICTimer as u32,
        ); //not-masked, periodic
        write(INITIAL_COUNT, *LAPIC_FREQUENCY.get().unwrap());
    }
}

pub fn start_lapic_timer() {
    unsafe {
        write(INITIAL_COUNT, COUNT_MAX);
    }
}

pub fn lapic_timer_elapsed() -> u32 {
    return COUNT_MAX - unsafe { read(CURRENT_COUNT) };
}

pub fn stop_lapic_timer() {
    unsafe {
        write(INITIAL_COUNT, 0);
    }
}

pub fn sleep(t: u64) {
    TIMER_MANAGER.lock().get_mut().unwrap().wait_seconds(t);
}
