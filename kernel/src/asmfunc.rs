use core::arch::asm;

pub fn ioin(address: usize) -> u32 {
    let value: u32;
    unsafe { value = *(address as *const u32); }
    value
}

pub fn ioout(address: usize, value: u32) {
    unsafe { *(address as *mut u32) = value }
}
