//! Hello World example for Horse OS
//!
//! This is a minimal example showing how to use horse_syscall
//! to write a "Hello, World!" program.

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use horse_syscall::prelude::*;

/// Entry point for the application
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Simple way: use the write function directly
    let _ = write(STDOUT, b"Hello from Horse OS!\n");

    // Using the print! macro (requires core::fmt::Write)
    // horse_syscall::println!("Hello, {}!", "World");

    // Infinite loop (we don't have exit syscall yet)
    loop {}
}

/// Panic handler - required for no_std
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let _ = write(STDERR, b"PANIC!\n");
    loop {}
}
