#![no_std]
#![no_main]

use core::panic::PanicInfo;
use horse_syscall::prelude::*;
use horse_syscall::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("This is Shoji, the Display Server for HorseOS");

    exit(0);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let _ = write(STDERR, b"PANIC: ");
    if let Some(location) = info.location() {
        let _ = write(STDERR, location.file().as_bytes());
    } else {
        let _ = write(STDERR, b"unknown location");
    }
    let _ = write(STDERR, b"\n");
    loop {
        core::hint::spin_loop();
    }
}