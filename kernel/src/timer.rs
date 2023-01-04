use core::ptr::{
    read_volatile,
    write_volatile
};

const COUNT_MAX: u32 = u32::MAX;
const LVT_TIMER: *mut u32 = 0xfee00320 as *mut u32;
const INITIAL_COUNT: *mut u32 = 0xfee00380 as *mut u32;
const CURRENT_COUNT: *const u32 = 0xfee00390 as *const u32;
const DIVIDE_CONFIG: *mut u32 = 0xfee003e0 as *mut u32;

pub fn initialize_lapic_itmer() {
    unsafe {
        write_volatile(DIVIDE_CONFIG, 0b1011);
        write_volatile(LVT_TIMER, (0b001 << 16) | 32);
    }
}

pub fn start_lapic_timer() {
    unsafe { write_volatile(INITIAL_COUNT, COUNT_MAX); }
}

pub fn lapic_timer_elapsed() -> u32 {
    return COUNT_MAX - unsafe { read_volatile(CURRENT_COUNT) }
}

pub fn stop_lapic_timer() {
    unsafe { write_volatile(INITIAL_COUNT, 0); }
}