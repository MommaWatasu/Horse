use core::ptr::write_volatile;
use spin::Mutex;

//static idt: Mutex<[InterruptDescriptor; 256]> = [; 256];

enum DescriptorType {
    KUpper8Bytes   = 0,
    KLDT           = 2,
    KTSSAvailable  = 9,
    KTSSBusy       = 11,
    KCallGate      = 12,
    KInterruptGate = 14,
    KTrapGate      = 15
}

#[repr(C)]
struct InterruptDescriptorAttribute {
    data: u32
}

#[repr(C, packed(4))]
struct InterruptDescriptor {
    offset_low: u16,
    segment_selector: u16,
    attr: InterruptDescriptorAttribute,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32
}

pub fn SetIDTEntry(
    desc: &mut InterruptDescriptor,
    attr: InterruptDescriptorAttribute,
    offset: u64,
    segment_selector: u16
    ) {
        desc.attr = attr;
        desc.offset_low = offset & 0xffff as u16;
        desc.offset_middle = (offset >> 16) & 0xffff as u16;
        desc.offset_high = offset >> 32;
        desc.segment_selector = segment_selector;
}

pub unsafe fn NotifyEndOfInterrupt() {
    let end_of_interrupt: *mut u32 = 0xfee000b0 as *mut u32;
    write_volatile(end_of_interrupt, 0);
}