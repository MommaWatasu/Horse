#![feature(abi_efiapi)]
#![feature(alloc_error_handler)]
#![feature(vec_into_raw_parts)]//into_raw_part
#![no_std]
#![no_main]

mod file;
use file::*;

#[macro_use]
extern crate alloc;
use alloc::{
    string::ToString,
    vec::Vec
};
extern crate libloader;
use libloader::{
    FrameBufferInfo,
    MemoryMap
};
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
    proto::{
        console::gop::{GraphicsOutput, Mode, ModeInfo},
        media::{
            file::{
                File, RegularFile
            },
        }
    },
    table::boot::{
        self,
        AllocateType,
        EventType,
        MemoryDescriptor,
        OpenProtocolParams,
        OpenProtocolAttributes,
        Tpl
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
        uefi::alloc::init(bt);
        bt.create_event(
            EventType::SIGNAL_EXIT_BOOT_SERVICES,
            Tpl::NOTIFY,
            Some(exit_signal),
            None
        )
        .map(|_| ())
        .expect("Error1");
    }

    if st.firmware_vendor().to_string() != "EDK II" {
        // set gop mode if it is not in QEMU
        set_gop_mode(gop);
    }

    //make FrameBufferInfo to send to kernel
    let mut mi = gop.current_mode_info();
    let mut fb = gop.frame_buffer();
    let fb_pt = fb.as_mut_ptr();
    let fb_size = fb.size();
    let mut fb = FrameBufferInfo {
        fb: fb_pt,
        size: fb_size,
    };
    drop(protocol);

    // open file protocol
    let mut root = open_root(bt, handler);

    //write memory map
    let mut mmap_file = create_file(&mut root, &cstr16!("memmap"));
    dump(&mut mmap_file, bt);
    mmap_file.close();

    //load kernel file
    let mut kernel_file = open_file(&mut root, &cstr16!("horse-kernel"));
    let entry_point_addr = load_kernel(&mut kernel_file, &st);
    kernel_file.close();
    let kernel_entry = unsafe {
        transmute::<
            *const (),
            extern "sysv64" fn(
                fb: *mut FrameBufferInfo, mi: *mut ModeInfo,
                memmap: MemoryMap) -> (),
        >(entry_point_addr as *const ())
    };

    //exit bootservices and get MemoryMap
    let memory_map = exit_boot_services(handler, st);

    kernel_entry(&mut fb, &mut mi, memory_map);
    uefi::Status::SUCCESS
}

fn dump(file: &mut RegularFile, bt: &BootServices) {
    let max_mmap_size = bt.memory_map_size().map_size + BUFFER_MARGIN;
    let mut mmap_buf = vec![0; max_mmap_size];
    let (_, descriptors) = bt.memory_map(&mut mmap_buf).expect("failed to get memory map");
    fwriteln!(file, "Index, Type, PhysicalStart, NumberOfPages, Attribute");
    for (i, d) in descriptors.enumerate() {
        fwriteln!(
            file,
            "{}, {:x}, {:?}, {:08x}, {:x}, {:x}",
            i,
            d.ty.0,
            d.ty,
            d.phys_start,
            d.page_count,
            d.att.bits() & 0xfffff
        )
    }
}

fn load_kernel(file: &mut RegularFile, st: &SystemTable<Boot>) -> usize {
    let buf = read_file_to_vec(file);
    let elf = elf::Elf::parse(&buf).expect("failed to parse ELF");

    let mut kernel_start = u64::MAX;
    let mut kernel_end = u64::MIN;
    for ph in elf.program_headers.iter() {
        if ph.p_type != elf::program_header::PT_LOAD {
            continue;
        }
        kernel_start = kernel_start.min(ph.p_vaddr);
        kernel_end = kernel_end.max(ph.p_vaddr + ph.p_memsz);
    }
    let n_of_pages = (kernel_end - kernel_start + UEFI_PAGE_SIZE - 1) / UEFI_PAGE_SIZE;
    st.boot_services().allocate_pages(
        AllocateType::Address(kernel_start),
        boot::MemoryType::LOADER_DATA,
        n_of_pages.try_into().unwrap(),
    ).unwrap();

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
        gop.set_mode(&mode).expect("Error2");
    }
}

fn exit_boot_services(handler: Handle, st: SystemTable<Boot>) -> MemoryMap{
    let mmap_size = st.boot_services().memory_map_size();
    let max_mmap_size = mmap_size.map_size + BUFFER_MARGIN;
    let mmap_buf = vec![0; max_mmap_size].leak();
    let mut descriptors = Vec::with_capacity(mmap_size.map_size/mmap_size.entry_size);
    let (_st, raw_descriptors) = st
        .exit_boot_services(handler, mmap_buf)
        .expect("failed to exit boot services");

    //make MemoryMap to send to kernel
    let memory_map = {
        for d in raw_descriptors {
            descriptors.push(*d);
        }
        let (ptr, _, _) = descriptors.into_raw_parts();
        MemoryMap::new(ptr, mmap_size)
    };
    return memory_map
}

unsafe extern "efiapi" fn exit_signal(_: uefi::Event, _: Option<NonNull<c_void>>) {
    uefi::alloc::exit_boot_services();
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
