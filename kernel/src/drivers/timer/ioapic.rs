use crate::{bit_getter, bit_setter, println, InterruptVector};

use core::ptr::{read, write};

//TODO: These Register address should be got from MADT in XSDT
const INDEX_REGISTER: *mut u8 = 0xfec00000 as *mut u8;
const DATA_REGISTER: *mut u32 = 0xfec00010 as *mut u32;
const LAPIC_ID_REGISTER: *mut u8 = 0xfee00020 as *mut u8;

struct RedirectionTable {
    pub data: u64,
}

impl RedirectionTable {
    bit_setter!(data: u64; 0xff00000000000000; u8, pub set_destination);
    bit_setter!(data: u64; 0x0000000000000800; u8, pub set_destination_mode);
    bit_setter!(data: u64; 0x00000000000000ff; u8, pub set_vector);
}

pub fn configure_redirection_table(idx: u8) {
    let mut rt: RedirectionTable;
    unsafe {
        write(INDEX_REGISTER, 2 * idx + 10);
        let lower_bit = read(DATA_REGISTER);
        write(INDEX_REGISTER, 2 * idx + 11);
        let upper_bit = read(DATA_REGISTER);
        rt = RedirectionTable {
            data: (upper_bit as u64) << 32 | lower_bit as u64,
        };
    }
    rt.set_destination_mode(1);
    let apic_id = unsafe { read(LAPIC_ID_REGISTER) };
    rt.set_destination(apic_id);
    //rt.set_vector(InterruptVector::Hpet as u8);
    unsafe {
        write(INDEX_REGISTER, 2 * idx + 10);
        let lower_bit = u32::try_from((rt.data << 32) >> 32).unwrap();
        write(DATA_REGISTER, lower_bit);

        write(INDEX_REGISTER, 2 * idx + 11);
        let upper_bit = u32::try_from(rt.data >> 32).unwrap();
        write(DATA_REGISTER, upper_bit);
    }
}
