use core::ptr::write_volatile;
use spin::Mutex;
pub use x86_64::structures::idt::InterruptDescriptorTable;

pub static IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());

#[repr(usize)]
pub enum InterruptVector {
    Xhci = 0x40,
    LAPICTimer = 0x41,
    Syscall = 0x80,
}

pub unsafe fn notify_end_of_interrupt() {
    let end_of_interrupt: *mut u32 = 0xfee000b0 as *mut u32;
    write_volatile(end_of_interrupt, 0);
}

// Assembly syscall handler
extern "C" {
    pub fn syscall_handler_asm();
}
