#![feature(alloc_error_handler)]
#![feature(vec_into_raw_parts)]//into_raw_part
#![no_std]
#![no_main]

mod fb;
mod file;

use fb::*;
use file::*;

#[macro_use]
extern crate alloc;
use alloc::{
    string::ToString,
    vec::Vec
};
extern crate libloader;
use libloader::MemoryMap;
use log::error;
use goblin::elf;
use core::{
    arch::asm,
    ffi::c_void,
    mem::{
        size_of,
        transmute
    },
    ops::DerefMut,
    ptr::NonNull,
    slice::from_raw_parts_mut
};
use uefi::{
    prelude::*,
    fs::{
        FileSystem,
        Path
    },
    proto::{
        console::gop::{GraphicsOutput, Mode},
    },
    table::{
        Runtime,
        boot::{
            self,
            AllocateType,
            EventType,
            MemoryDescriptor,
            OpenProtocolParams,
            OpenProtocolAttributes,
            Tpl,
        },
    },
};

const UEFI_PAGE_SIZE: u64 = 0x1000;
const BUFFER_MARGIN: usize = 8 * size_of::<MemoryDescriptor>();

#[entry]
fn efi_main(handler: Handle, st: SystemTable<Boot>) -> Status {
    let bt = st.boot_services();

    let gop_handle = bt.get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut protocol = unsafe {
        bt.open_protocol::<GraphicsOutput>(
            OpenProtocolParams{
                handle: gop_handle,
                agent: bt.image_handle(),
                controller: None
            },
            OpenProtocolAttributes::GetProtocol
        ).expect("no gop")
    };
    let gop = protocol.deref_mut();

    unsafe {
        uefi::allocator::init(bt);
        bt.create_event(
            EventType::SIGNAL_EXIT_BOOT_SERVICES,
            Tpl::NOTIFY,
            Some(exit_signal),
            None
        )
        .map(|_| ())
        .unwrap();
    }

    if st.firmware_vendor().to_string() != "EDK II" {
        // set gop mode if it is not in QEMU
        set_gop_mode(gop);
    }

    //make FrameBufferInfo to send to kernel
    let mi = gop.current_mode_info();
    let fb = gop.frame_buffer();
    let mut fb_config = FrameBufferConfig::new(fb, mi);
    drop(protocol);

    // open file protocol
    let mut fs = open_root(bt, handler);

    //write memory map
    let mut mmap_file = FileBuffer::new();
    dump(&mut mmap_file, bt);
    mmap_file.flush(&mut fs, cstr16!("memmap"));

    //load kernel file
    let entry_point_addr = load_kernel(&mut fs, &st);
    drop(fs);
    let kernel_entry = unsafe {
        transmute::<
            *const (),
            extern "sysv64" fn(
                st: SystemTable<Runtime>,
                fb_config: *mut FrameBufferConfig,
                memmap: *const MemoryMap) -> (),
        >(entry_point_addr as *const ())
    };

    //exit bootservices and get MemoryMap
    let (st, memory_map) = exit_boot_services(st);

    kernel_entry(st, &mut fb_config, &memory_map);
    uefi::Status::SUCCESS
}

fn dump(file: &mut FileBuffer, bt: &BootServices) {
    let max_mmap_size = bt.memory_map_size().map_size + BUFFER_MARGIN;
    let mut mmap_buf = vec![0; max_mmap_size];
    let memory_map = bt.memory_map(&mut mmap_buf).expect("failed to get memory map");
    file.writeln("Index, Type, PhysicalStart, NumberOfPages, Attribute");
    for (i, d) in memory_map.entries().enumerate() {
        file.writeln(&format!(
            "{}, {:?}, {:08x}, {:x}, {:x}",
            i,
            d.ty,
            d.phys_start,
            d.page_count,
            d.att.bits() & 0xfffff
        ))
    }
}

fn load_kernel(fs: &mut FileSystem, st: &SystemTable<Boot>) -> usize {
    //open kernel file
    let buf = fs.read(Path::new(&cstr16!("horse-kernel"))).expect("failed to read kernel file");
    let elf = elf::Elf::parse(&buf).expect("failed to parse ELF");

    //find kernel_start and kernel_end using physical addresses (p_paddr)
    //For higher-half kernels, p_vaddr is the virtual address and p_paddr is the physical address
    let mut kernel_start = u64::MAX;
    let mut kernel_end = 0;
    for ph in elf.program_headers.iter() {
        if ph.p_type != elf::program_header::PT_LOAD {
            continue;
        }
        // Use physical address for memory allocation
        kernel_start = kernel_start.min(ph.p_paddr);
        kernel_end = kernel_end.max(ph.p_paddr + ph.p_memsz);
    }

    //allocate pages for kernel file at physical addresses
    let n_of_pages = ((kernel_end - kernel_start + UEFI_PAGE_SIZE - 1) / UEFI_PAGE_SIZE) as usize;
    st.boot_services().allocate_pages(
        AllocateType::Address(kernel_start),
        boot::MemoryType::LOADER_DATA,
        n_of_pages.try_into().unwrap(),
    ).expect("failed to allocate pages for kernel");

    //load kernel file to physical addresses
    for ph in elf.program_headers.iter() {
        if ph.p_type != elf::program_header::PT_LOAD {
            continue;
        }
        let ofs = ph.p_offset as usize;
        let fsize = ph.p_filesz as usize;
        let msize = ph.p_memsz as usize;
        // Load to physical address (p_paddr), not virtual address (p_vaddr)
        let dest = unsafe { from_raw_parts_mut(ph.p_paddr as *mut u8, msize) };
        dest[..fsize].copy_from_slice(&buf[ofs..ofs + fsize]);
        dest[fsize..].fill(0);
    }

    // For higher-half kernels, we need to calculate the physical entry point
    // The ELF entry is a virtual address, we need to convert it to physical
    // entry_phys = entry_virt - (p_vaddr - p_paddr) of the first LOAD segment
    let first_load = elf.program_headers.iter()
        .find(|ph| ph.p_type == elf::program_header::PT_LOAD)
        .expect("No LOAD segment found");
    let virt_to_phys_offset = first_load.p_vaddr - first_load.p_paddr;
    let entry_phys = elf.entry - virt_to_phys_offset;

    return entry_phys as usize
}

#[allow(dead_code)]
fn set_gop_mode(gop: &mut GraphicsOutput) {
    let mut mode: Option<Mode> = None;
    for m in gop.modes().into_iter() {
        let res = m.info().resolution();

        // Hardcode for GPD Pocket / Lemur Pro.
        if (mode.is_none() && (1024, 768) == res) || (1200, 1920) == res || (1920, 1080) == res {
            mode = Some(m);
        }
    }

    if let Some(mode) = mode {
        gop.set_mode(&mode).expect("failed to setup GOP mode");
    }
}

fn exit_boot_services(st: SystemTable<Boot>) -> (SystemTable<Runtime>, MemoryMap) {
    let mmap_size = st.boot_services().memory_map_size();
    let mut descriptors = Vec::with_capacity(mmap_size.map_size/mmap_size.entry_size);
    let (st, memory_map) = st.exit_boot_services();

    //make MemoryMap to send to kernel
    let memory_map = {
        for d in memory_map.entries() {
            descriptors.push(*d);
        }
        let (ptr, _, _) = descriptors.into_raw_parts();
        MemoryMap::new(ptr, mmap_size)
    };
    return (st, memory_map);
}

unsafe extern "efiapi" fn exit_signal(_: uefi::Event, _: Option<NonNull<c_void>>) {
    uefi::allocator::exit_boot_services();
}

#[alloc_error_handler]
fn out_of_memory(layout: ::core::alloc::Layout) -> ! {
    panic!(
        "Ran out of free memory while trying to allocate {:#?}",
        layout
    );
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
