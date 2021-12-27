#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_efiapi)]

use core::panic::PanicInfo;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct FrameBufferInfo {
    pub fb: *mut u8,
    pub size: usize,
}

#[no_mangle]
extern "sysv64" fn kernel_main(fb_config: FrameBufferInfo) -> ! {
    loop {
        unsafe {
            asm!("hlt")
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    //error!("{}", info);
    loop {}
}
