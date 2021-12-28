#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_efiapi)]

mod ascii_font;
pub mod log;
pub mod graphics;
pub mod console;

use log::*;

use console::Console;
use core::panic::PanicInfo;
use graphics::{FrameBuffer, Graphics, ModeInfo, PixelColor};
//use pci::PciDevices;
//use pci::{read_bar, read_class_code, read_vendor_id, scan_all_bus, ClassCode, Device};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct FrameBufferInfo {
    pub fb: *mut u8,
    pub size: usize,
}

#[no_mangle]
extern "sysv64" fn kernel_main(fb: *mut FrameBuffer, mi: *mut ModeInfo) -> ! {
    loop {
        unsafe {
            asm!("hlt")
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}
