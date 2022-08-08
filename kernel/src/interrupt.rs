use core::ptr::write_volatile;
use spin::Mutex;
pub use x86_64::structures::idt::{
    InterruptDescriptorTable
};

pub static IDT: Mutex<InterruptDescriptorTable> =
    Mutex::new(InterruptDescriptorTable::new());

pub enum InterruptVector {
    KXHCI = 64
}

pub unsafe fn notify_end_of_interrupt() {
    let end_of_interrupt: *mut u32 = 0xfee000b0 as *mut u32;
    write_volatile(end_of_interrupt, 0);
}