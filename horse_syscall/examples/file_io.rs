//! File I/O example for Horse OS
//!
//! This example demonstrates reading from a file using horse_syscall.

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use horse_syscall::prelude::*;
use horse_syscall::fs::File;

/// Entry point for the application
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _ = write(STDOUT, b"File I/O Example\n");
    let _ = write(STDOUT, b"================\n\n");

    // Method 1: Using low-level functions
    let _ = write(STDOUT, b"Opening /test.txt...\n");

    match open("/test.txt", OpenFlags::RDONLY) {
        Ok(fd) => {
            let _ = write(STDOUT, b"File opened successfully!\n");

            let mut buf = [0u8; 256];
            match read(fd, &mut buf) {
                Ok(n) => {
                    let _ = write(STDOUT, b"Read ");
                    // Note: We can't easily print the number without format support
                    let _ = write(STDOUT, b" bytes:\n");
                    let _ = write(STDOUT, &buf[..n]);
                    let _ = write(STDOUT, b"\n");
                }
                Err(_) => {
                    let _ = write(STDERR, b"Failed to read file\n");
                }
            }

            let _ = close(fd);
            let _ = write(STDOUT, b"File closed.\n");
        }
        Err(_) => {
            let _ = write(STDERR, b"Failed to open file\n");
        }
    }

    // Method 2: Using the File wrapper (RAII)
    let _ = write(STDOUT, b"\nUsing File wrapper:\n");

    if let Ok(file) = File::open("/test.txt", OpenFlags::RDONLY) {
        let mut buf = [0u8; 256];
        if let Ok(n) = file.read(&mut buf) {
            let _ = write(STDOUT, b"Content: ");
            let _ = write(STDOUT, &buf[..n]);
            let _ = write(STDOUT, b"\n");
        }
        // File automatically closed when dropped
    }

    let _ = write(STDOUT, b"\nDone!\n");

    loop {}
}

/// Panic handler
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let _ = write(STDERR, b"PANIC!\n");
    loop {}
}
