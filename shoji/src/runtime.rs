use core::panic::PanicInfo;
use horse_std::prelude::*;

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    exit(1)
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
    exit(1)
}
