#![no_std]
#![no_main]
#![feature(abi_efiapi)]

// uefi-servicesを明示的にリンク対象に含める
extern crate uefi_services;

use core::fmt::Write;
//use core::panic::PanicInfo;
use uefi::prelude::*;
use uefi::table::boot::{MemoryType, MemoryAttribute};
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::proto::media::file::{File, RegularFile, Directory, FileMode, FileAttribute};

fn u64_2_ascii(number: u64) -> [u8; 16] {
    let mut result: [u8; 16] = [0; 16];
    for i in 0..16 {
        let target_4bit = ((number >> i*4) % 16) as u8;
        if target_4bit <= 0x9 {
            result[i] = 0x30 + target_4bit;
        } else if target_4bit >= 0xa && target_4bit <= 0xf {
            result[i] = 0x57 + target_4bit;
        }
    }
    return result
}

fn u32_2_ascii(number: u32) -> [u8; 8] {
    let mut result: [u8; 8] = [0; 8];
    for i in 0..8 {
        let target_4bit = ((number >> i*4) % 16) as u8;
        if target_4bit <= 0x9 {
            result[i] = 0x30 + target_4bit;
        } else if target_4bit >= 0xa && target_4bit <= 0xf {
            result[i] = 0x57 + target_4bit;
        }
    }
    return result;
}

#[entry]
fn efi_main(handle: Handle, mut st: SystemTable<Boot>) -> Status {
    st.stdout().reset(false).unwrap_success();
    writeln!(st.stdout(), "Running bootloader...").unwrap();

    //get a memory map
    let memory_map_buffer: &mut [u8] = &mut [0; 4096*4];
    //return map_key and iterator of discriptor
    let (_memory_map_key, descriptor_iter) = st.boot_services().memory_map(memory_map_buffer).unwrap_success();
    //open root dir
    let loaded_image = st.boot_services().handle_protocol::<LoadedImage>(handle).unwrap_success().get();
    let device;
    unsafe {
        device = (*loaded_image).device();
    }
    let file_system = st.boot_services().handle_protocol::<SimpleFileSystem>(device).unwrap_success().get();
    let mut root_dir: Directory;
    unsafe {
        root_dir = (*file_system).open_volume().unwrap_success();
    }

    let memory_map_file_handle = root_dir.open("\\memmap", FileMode::CreateReadWrite, FileAttribute::empty()).unwrap_success();
    let mut memory_map_file: RegularFile;
    unsafe {
        memory_map_file = RegularFile::new(memory_map_file_handle)
    }
    let header: &[u8] = "Type, PhysicalStart, NumberOfPages, Attribute\n".as_bytes();
    memory_map_file.write(header).unwrap_success();
    // writing memory descriptor
    for descriptor in descriptor_iter {
        let memory_type:u32 = match descriptor.ty {
            MemoryType::RESERVED => 0,
            MemoryType::LOADER_CODE => 1,
            MemoryType::LOADER_DATA => 2,
            MemoryType::BOOT_SERVICES_CODE => 3,
            MemoryType::BOOT_SERVICES_DATA => 4,
            MemoryType::RUNTIME_SERVICES_CODE => 5,
            MemoryType::RUNTIME_SERVICES_DATA => 6,
            MemoryType::CONVENTIONAL => 7,
            MemoryType::UNUSABLE => 8,
            MemoryType::ACPI_RECLAIM => 9,
            MemoryType::ACPI_NON_VOLATILE => 10,
            MemoryType::MMIO => 11,
            MemoryType::MMIO_PORT_SPACE => 12,
            MemoryType::PAL_CODE => 13,
            MemoryType::PERSISTENT_MEMORY => 14,
            _ => 0xffff_ffff,
        };
        let physical_start = descriptor.phys_start;
        let number_of_pages = descriptor.page_count;
        let attribute: u64 = match descriptor.att {
            MemoryAttribute::UNCACHEABLE => 0x1,
            MemoryAttribute::WRITE_COMBINE => 0x2,
            MemoryAttribute::WRITE_THROUGH => 0x4,
            MemoryAttribute::WRITE_BACK => 0x8,
            MemoryAttribute::UNCACHABLE_EXPORTED => 0x10,
            MemoryAttribute::WRITE_PROTECT => 0x1000,
            MemoryAttribute::READ_PROTECT => 0x2000,
            MemoryAttribute::EXECUTE_PROTECT => 0x4000,
            MemoryAttribute::NON_VOLATILE => 0x8000,
            MemoryAttribute::MORE_RELIABLE => 0x10000,
            MemoryAttribute::READ_ONLY => 0x20000,
            MemoryAttribute::RUNTIME => 0x8000_0000_0000_0000,
            _ => 0,
        };
        let buffer: &mut [u8] = &mut[0;63];
        let memory_type = u32_2_ascii(memory_type);
        let physical_start = u64_2_ascii(physical_start);
        let number_of_pages = u64_2_ascii(number_of_pages);
        let attribute = u64_2_ascii(attribute);

        //memory_type
        let memory_type_len = memory_type.len();
        for i in 0..memory_type_len {
            buffer[i] = memory_type[memory_type_len-i-1];
        }
        buffer[memory_type_len] = 0x2c;
        buffer[memory_type_len+1] = 0x20;//space

        //physical_start
        let physical_start_len = physical_start.len();
        let padding = memory_type_len + 2;
        for i in 0..physical_start_len {
            buffer[padding+i] = physical_start[physical_start_len-i-1];
        }
        buffer[padding+physical_start_len] = 0x2c;
        buffer[padding+physical_start_len+1] = 0x20;//space

        //memory_of_pages
        let number_of_pages_len = number_of_pages.len();
        let padding = memory_type_len + physical_start_len + 4;
        for i in 0..number_of_pages_len {
            buffer[padding+i] = number_of_pages[number_of_pages_len-i-1];
        }
        buffer[padding+number_of_pages_len] = 0x2c;
        buffer[padding+number_of_pages_len+1] = 0x20;//space

        //atribute
        let attribute_len = attribute.len();
        let padding = memory_type_len + physical_start_len + number_of_pages_len + 6;
        for i in 0..attribute_len {
            buffer[padding+i] = attribute[attribute_len-i-1];
        }
        buffer[padding+attribute_len] = 0x0a;//LF

        memory_map_file.write(buffer).unwrap_success();
    }
    memory_map_file.flush().unwrap_success();

    writeln!(st.stdout(), "Kernel didn't execute").unwrap();

    loop {}
    //Status::SUCCESS
}
