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
use alloc::string::ToString;
extern crate libloader;
use libloader::BootMemoryMap;
use log::error;
use goblin::elf;
use core::{
    arch::asm,
    mem::transmute,
    ops::DerefMut,
    slice::from_raw_parts_mut
};
use uefi::{
    boot::{
        allocate_pages, exit_boot_services, get_handle_for_protocol, image_handle, memory_map, open_protocol, AllocateType, MemoryType, OpenProtocolAttributes, OpenProtocolParams,
    }, fs::{
        FileSystem,
        Path
    }, mem::memory_map::MemoryMap, prelude::*, proto::console::gop::{GraphicsOutput, Mode}, system::firmware_vendor, table::system_table_raw
};
use uefi_raw::table::system::SystemTable;

const UEFI_PAGE_SIZE: u64 = 0x1000;

#[entry]
fn efi_main() -> Status {
    let gop_handle = get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut protocol = unsafe {
        open_protocol::<GraphicsOutput>(
            OpenProtocolParams{
                handle: gop_handle,
                agent: image_handle(),
                controller: None
            },
            OpenProtocolAttributes::GetProtocol
        ).expect("no gop")
    };
    let gop = protocol.deref_mut();

    if firmware_vendor().to_string() != "EDK II" {
        // set gop mode if it is not in QEMU
        set_gop_mode(gop);
    }

    //make FrameBufferInfo to send to kernel
    let mi = gop.current_mode_info();
    let fb = gop.frame_buffer();
    let mut fb_config = FrameBufferConfig::new(fb, mi);
    drop(protocol);

    // open file protocol
    let mut fs = open_root();

    //write memory map
    let mut mmap_file = FileBuffer::new();
    dump(&mut mmap_file);
    mmap_file.flush(&mut fs, cstr16!("memmap"));

    //load kernel file
    let entry_point_addr = load_kernel(&mut fs);
    drop(fs);
    let kernel_entry = unsafe {
        transmute::<
            *const (),
            extern "sysv64" fn(
                sys_table: SystemTable,
                fb_config: *mut FrameBufferConfig,
                memmap: BootMemoryMap) -> (),
        >(entry_point_addr as *const ())
    };

    //exit bootservices and get MemoryMap
    let memory_map = unsafe { BootMemoryMap::new(exit_boot_services(None)) };
    let sys_table = unsafe { system_table_raw().expect("failed to get system table").read() };

    kernel_entry(sys_table, &mut fb_config, memory_map);
    uefi::Status::SUCCESS
}

fn dump(file: &mut FileBuffer) {
    let memory_map = memory_map(MemoryType::LOADER_DATA).expect("failed to get memory map");
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

fn load_kernel(fs: &mut FileSystem) -> usize {
    //open kernel file
    let buf = fs.read(Path::new(&cstr16!("horse-kernel"))).expect("failed to read kernel file");
    let elf = elf::Elf::parse(&buf).expect("failed to parse ELF");

    //find kernel_start and kernel_end
    let mut kernel_start = u64::MAX;
    let mut kernel_end = 0;
    for ph in elf.program_headers.iter() {
        if ph.p_type != elf::program_header::PT_LOAD {
            continue;
        }
        kernel_start = kernel_start.min(ph.p_vaddr);
        kernel_end = kernel_end.max(ph.p_vaddr + ph.p_memsz);
    }

    //allocate pages for kernel file
    let n_of_pages = ((kernel_end - kernel_start + UEFI_PAGE_SIZE - 1) / UEFI_PAGE_SIZE) as usize;
    allocate_pages(
        AllocateType::Address(kernel_start),
        boot::MemoryType::LOADER_DATA,
        n_of_pages.try_into().unwrap(),
    ).expect("failed to allocate pages for kernel");

    //load kernel file
    for ph in elf.program_headers.iter() {
        if ph.p_type != elf::program_header::PT_LOAD {
            continue;
        }
        let ofs = ph.p_offset as usize;
        let fsize = ph.p_filesz as usize;
        let msize = ph.p_memsz as usize;
        let dest = unsafe { from_raw_parts_mut(ph.p_vaddr as *mut u8, msize) };
        dest[..fsize].copy_from_slice(&buf[ofs..ofs + fsize]);
        dest[fsize..].fill(0);
    }

    return elf.entry as usize
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
