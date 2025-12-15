//! Hello World program for Horse OS
//!
//! This is a minimal user-space program that demonstrates
//! using system calls via the horse_syscall library.

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use horse_syscall::prelude::*;

/// Entry point for the application
///
/// This function is called by the OS after loading the ELF.
/// The name `_start` is the conventional entry point for ELF executables.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Print Hello World using the write system call
    let message = b"Hello, World from Horse OS!\n";
    let _ = write(STDOUT, message);
    // Successfully returned from syscall!

    // Note: 'out' instruction is privileged and cannot be used in user mode
    // Use write syscall instead for debug output
    let _ = write(STDOUT, b"Syscall returned successfully!\n");

    // Since we don't have an exit syscall yet, loop forever
    // In a real program, you would call exit(0) here
    loop {
        // Hint to the CPU that we're in a spin loop
        core::hint::spin_loop();
    }
}

/// Panic handler
///
/// Required for no_std programs. Called when the program panics.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try to print panic message to stderr
    let _ = write(STDERR, b"PANIC: ");

    if let Some(location) = info.location() {
        // We can't easily format strings in no_std without alloc,
        // so just print a generic message
        let _ = write(STDERR, b"at ");
        let _ = write(STDERR, location.file().as_bytes());
        let _ = write(STDERR, b"\n");
    } else {
        let _ = write(STDERR, b"unknown location\n");
    }

    loop {
        core::hint::spin_loop();
    }
}
