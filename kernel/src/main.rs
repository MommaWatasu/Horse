#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_efiapi)]

#[no_mangle]
extern "efiapi" fn kernel_main(fb_config: FrameBufferConfig) -> ! {
    loop {
        unsafe {
            asm!("hlt")
        }
    }
}
