use x86_64::structures::gdt::{
    Descriptor,
    GlobalDescriptorTable
};

use crate::trace;

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
const CODE_SEGMENT: u64 = 0b0000000010101111100110100000000000000000000000001111111111111111;
const DATA_SEGMENT: u64 = 0b0000000011001111100100100000000000000000000000001111111111111111;

unsafe fn setup_segments() {
    // TODO: GDT needs to be created for each processor.
    trace!("INITIALIZING segmentation");
    GDT.add_entry(Descriptor::UserSegment(0));
    GDT.add_entry(Descriptor::UserSegment(CODE_SEGMENT));
    GDT.add_entry(Descriptor::UserSegment(DATA_SEGMENT));
    GDT.load();
}

pub fn initialize() {
    const KERNEL_CS: u16 = 1 << 3;
    const KERNEL_SS: u16 = 2 << 3;

    unsafe {
        setup_segments();
        set_ds_all(0);
        set_cs_ss(KERNEL_CS, KERNEL_SS);
    }
}

extern "C" {
    fn set_ds_all(value: u16);
    fn set_cs_ss(cs: u16, ss: u16);
}