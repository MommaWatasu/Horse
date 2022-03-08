use core::ptr::write_volatile;

enum DescriptorType {
    kUpper8Bytes   = 0,
    kLDT           = 2,
    kTSSAvailable  = 9,
    kTSSBusy       = 11,
    kCallGate      = 12,
    kInterruptGate = 14,
    kTrapGate      = 15
}

pub unsafe fn NotifyEndOfInterrupt() {
    let end_of_interrupt: *mut u32 = 0xfee000b0 as *mut u32;
    write_volatile(end_of_interrupt, 0);
}