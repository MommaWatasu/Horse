use core::{
    mem::size_of,
    slice::from_raw_parts
};

use alloc::string::String;
use spin::Once;

const CRC32_POLYNOMIAL: u32 = 0xedb88320;
static CRC_TABLE: Once<[u32; 256]> = Once::new();

pub fn bytes2str(bytes: &[u8]) -> String {
    return String::from_utf8(bytes.to_vec()).unwrap();
}

pub fn sum_bytes<T>(data: &T, len: usize) -> u8 {
    let data = unsafe { data as *const T as *const u8 };
    let mut sum: u8 = 0;
    for i in 0..len {
        sum.wrapping_add(unsafe { *data.wrapping_add(i) });
    }
    return sum;
}

fn make_crc_table() -> [u32; 256] { 
    let mut c: u32;
    let mut crc_table = [0; 256];
    for n in 0..256 {
        c = n;
        for k in 0..8 {
            if c & 1 == 1 {
                c = CRC32_POLYNOMIAL ^ (c >> 1);
            } else {
                c = c >> 1;
            }
        }
        crc_table[n as usize] = c;
    }
    return crc_table
}

pub fn update_crc<T>(crc: u32, data: &T) -> u32 {
    let buf = unsafe { from_raw_parts(data as *const T as *const u8, size_of::<T>()) };
    let mut c = crc ^ 0xffffffff;
    if !CRC_TABLE.is_completed() {
        CRC_TABLE.call_once(make_crc_table);
    }
    for &byte in buf {
        c = CRC_TABLE.get().unwrap()[(c ^ byte as u32) as usize & 0xff] ^ (c >> 8);
    }
    return c ^ 0xffffffff
}

pub fn crc<T>(data: &T) -> u32 {
    return update_crc(0, data)
}

pub fn negative(x: u32) -> u32 {
    if x != 0 {
        return 0;
    } else {
        return 1;
    }
}

pub fn bytes2u32(bytes: &[u8]) -> u32 {
    let mut r= 0;
    for i in 0..4 {
        r += (bytes[i] as u32) << (i * 4);
    }
    return r
}

pub fn bytes2u64(bytes: &[u8]) -> u64 {
    let mut r= 0;
    for i in 0..8 {
        r |= (bytes[i] as u64) << (i * 4);
    }
    return r
}