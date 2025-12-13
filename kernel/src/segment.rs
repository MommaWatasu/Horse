use crate::{bit_setter, trace};

use core::mem::size_of;

// GDT entries: null, kernel_cs, kernel_ss, user_cs, user_ss, tss_low, tss_high
static mut GDT: [SegmentDescriptor; 7] = [SegmentDescriptor::new(); 7];

// TSS structure for x86-64
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved0: u32,
    pub rsp0: u64,      // Stack pointer for Ring 0
    pub rsp1: u64,      // Stack pointer for Ring 1
    pub rsp2: u64,      // Stack pointer for Ring 2
    reserved1: u64,
    pub ist1: u64,      // Interrupt Stack Table 1
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    reserved2: u64,
    reserved3: u16,
    pub iomap_base: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            reserved0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved1: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved2: 0,
            reserved3: 0,
            iomap_base: size_of::<TaskStateSegment>() as u16,
        }
    }
}

// Kernel stack for Ring 0 (used when transitioning from Ring 3)
#[repr(C, align(16))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

const KERNEL_STACK_SIZE: usize = 64 * 1024; // 64KB stack

static mut KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};

static mut TSS: TaskStateSegment = TaskStateSegment::new();

#[allow(dead_code)]
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

/// Setup TSS descriptor in GDT
/// TSS descriptor in 64-bit mode is 16 bytes (two GDT entries)
unsafe fn setup_tss_descriptor(gdt_index: usize, tss_addr: u64, tss_size: u32) {
    let limit = tss_size - 1;
    
    // Lower 8 bytes of TSS descriptor
    GDT[gdt_index].data = 0;
    GDT[gdt_index].limit_low(limit as u16);
    GDT[gdt_index].base_low(tss_addr as u16);
    GDT[gdt_index].base_middle((tss_addr >> 16) as u8);
    GDT[gdt_index].ty(DescriptorType::TSSAvailable as u8);
    GDT[gdt_index].system_segment(0); // System segment (TSS)
    GDT[gdt_index].descriptor_privilege_level(0);
    GDT[gdt_index].present(1);
    GDT[gdt_index].limit_high(((limit >> 16) & 0xf) as u8);
    GDT[gdt_index].available(0);
    GDT[gdt_index].long_mode(0);
    GDT[gdt_index].default_operation_size(0);
    GDT[gdt_index].granularity(0);
    GDT[gdt_index].base_high((tss_addr >> 24) as u8);
    
    // Upper 8 bytes of TSS descriptor (contains high 32 bits of base address)
    GDT[gdt_index + 1].data = (tss_addr >> 32) & 0xFFFFFFFF;
}

unsafe fn setup_segments() {
    // TODO: GDT needs to be created for each processor.
    trace!("INITIALIZING segmentation");
    // Kernel segments (DPL=0)
    setup_code_segment(&mut GDT[1], DescriptorType::ExecuteRead, 0, 0, 0xfffff);
    setup_data_segment(&mut GDT[2], DescriptorType::ReadWrite, 0, 0, 0xfffff);
    // User segments (DPL=3)
    setup_code_segment(&mut GDT[3], DescriptorType::ExecuteRead, 3, 0, 0xfffff);
    setup_data_segment(&mut GDT[4], DescriptorType::ReadWrite, 3, 0, 0xfffff);
    
    // Setup TSS
    // Set RSP0 to point to top of kernel stack (stack grows downward)
    let kernel_stack_top = &KERNEL_STACK.data as *const _ as u64 + KERNEL_STACK_SIZE as u64;
    TSS.rsp0 = kernel_stack_top;
    trace!("TSS RSP0 set to: {:#x}", kernel_stack_top);
    
    // Setup TSS descriptor in GDT (indices 5 and 6)
    let tss_addr = &TSS as *const _ as u64;
    setup_tss_descriptor(5, tss_addr, size_of::<TaskStateSegment>() as u32);
    trace!("TSS address: {:#x}", tss_addr);
    
    load_gdt(
        (size_of::<[SegmentDescriptor; 7]>()) as u16 - 1,
        &GDT[0] as *const SegmentDescriptor as usize,
    );
}

/// Kernel code segment selector
pub const KERNEL_CS: u16 = 1 << 3;
/// Kernel stack segment selector
pub const KERNEL_SS: u16 = 2 << 3;
/// Kernel data segment selector
const KERNEL_DS: u16 = 0;
/// User code segment selector (index 3, RPL=3)
pub const USER_CS: u16 = (3 << 3) | 3;
/// User stack/data segment selector (index 4, RPL=3)
pub const USER_SS: u16 = (4 << 3) | 3;
/// TSS segment selector (index 5)
pub const TSS_SELECTOR: u16 = 5 << 3;

pub fn initialize() {
    unsafe {
        setup_segments();
        set_ds_all(KERNEL_DS);
        set_cs_ss(KERNEL_CS, KERNEL_SS);
        // Load TSS
        load_tss(TSS_SELECTOR);
        trace!("TSS loaded with selector: {:#x}", TSS_SELECTOR);
    }
}

//assembly function in asm.s
extern "C" {
    fn load_gdt(limit: u16, offset: usize);
    fn set_ds_all(value: u16);
    fn set_cs_ss(cs: u16, ss: u16);
    fn load_tss(selector: u16);
}
