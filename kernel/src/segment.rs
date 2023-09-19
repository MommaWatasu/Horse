use crate::{bit_setter, trace};

use core::mem::size_of;

static mut GDT: [SegmentDescriptor; 3] = [SegmentDescriptor::new(); 3];

enum DescriptorType {
    Upper8Bytes = 0,
    TSSAvailable = 9,
    TSSBusy = 11,
    CallGate = 12,
    InterruptGate = 14,
    TrapGate = 15,

    //code & segment types
    ReadWrite = 2,
    ExecuteRead = 10,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SegmentDescriptor {
    data: u64,
}

impl SegmentDescriptor {
    const fn new() -> Self {
        Self { data: 0 }
    }
    bit_setter!(data: u64; 0x000000000000FFFF; u16, limit_low);
    bit_setter!(data: u64; 0x00000000FFFF0000; u16, base_low);
    bit_setter!(data: u64; 0x000000FF00000000; u8, base_middle);
    bit_setter!(data: u64; 0x00000F0000000000; u8, ty);
    bit_setter!(data: u64; 0x0000100000000000; u8, system_segment);
    bit_setter!(data: u64; 0x0000600000000000; u8, descriptor_privilege_level);
    bit_setter!(data: u64; 0x0000800000000000; u8, present);
    bit_setter!(data: u64; 0x000F000000000000; u8, limit_high);
    bit_setter!(data: u64; 0x0010000000000000; u8, available);
    bit_setter!(data: u64; 0x0020000000000000; u8, long_mode);
    bit_setter!(data: u64; 0x0040000000000000; u8, default_operation_size);
    bit_setter!(data: u64; 0x0080000000000000; u8, granularity);
    bit_setter!(data: u64; 0xFF00000000000000; u8, base_high);
}

fn setup_code_segment(
    descriptor: &mut SegmentDescriptor,
    ty: DescriptorType,
    descriptor_privilege_level: u8,
    base: u32,
    limit: u32,
) {
    descriptor.data = 0;

    descriptor.base_low(base as u16);
    descriptor.base_middle((base >> 16) as u8);
    descriptor.base_high((base >> 24) as u8);

    descriptor.limit_low(limit as u16);
    descriptor.limit_high(((limit >> 16) & 0xf) as u8);

    descriptor.ty(ty as u8);
    descriptor.system_segment(1);
    descriptor.descriptor_privilege_level(descriptor_privilege_level);
    descriptor.present(1);
    descriptor.available(0);
    descriptor.long_mode(1);
    descriptor.default_operation_size(0);
    descriptor.granularity(1);
}

fn setup_data_segment(
    descriptor: &mut SegmentDescriptor,
    ty: DescriptorType,
    descriptor_privilege_level: u8,
    base: u32,
    limit: u32,
) {
    setup_code_segment(descriptor, ty, descriptor_privilege_level, base, limit);
    descriptor.long_mode(0);
    descriptor.default_operation_size(1);
}

unsafe fn setup_segments() {
    // TODO: GDT needs to be created for each processor.
    trace!("INITIALIZING segmentation");
    setup_code_segment(&mut GDT[1], DescriptorType::ExecuteRead, 0, 0, 0xfffff);
    setup_data_segment(&mut GDT[2], DescriptorType::ReadWrite, 0, 0, 0xfffff);
    load_gdt(
        (size_of::<[SegmentDescriptor; 3]>()) as u16 - 1,
        &GDT[0] as *const SegmentDescriptor as usize,
    );
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

//assembly function in asm.s
extern "C" {
    fn load_gdt(limit: u16, offset: usize);
    fn set_ds_all(value: u16);
    fn set_cs_ss(cs: u16, ss: u16);
}
