use crate::{error, fftimer::FFTimer, info, initialize_lapic_itmer, horse_lib::bytes::*};

use alloc::vec::Vec;
use core::{
    mem::size_of,
    ptr::{null, read_unaligned},
};
use uefi::{
    table::{Runtime, SystemTable},
    Guid,
};

const EFI_ACPI_TABLE_GUID: Guid = Guid::new(
    [0x71, 0xe8, 0x68, 0x88],
    [0xf1, 0xe4],
    [0xd3, 0x11],
    0xbc,
    0x22,
    [0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
);

#[derive(Copy, Clone, Debug)]
#[repr(packed, C)]
pub struct RSDP {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

impl RSDP {
    fn new(base: *const RSDP) -> Self {
        return unsafe { read_unaligned(base) };
    }

    fn validate(&self) -> bool {
        if bytes2str(&self.signature) != "RSD PTR " {
            error!("RSDP -invalid signature: {:?}", self.signature);
            return false;
        }
        if self.revision != 2 {
            error!("RSDP -inavlid signature");
            return false;
        }
        let sum = sum_bytes(&self, 20);
        if sum != 0 {
            error!("RSDP -sum of 20 bytes must be 0: {}", sum);
            return false;
        }
        let sum = sum_bytes(&self, 36);
        if sum != 0 {
            error!("RSDP -sum of 36 bytes must be 0: {}", sum);
            return false;
        }
        return true;
    }
}

#[derive(Copy, Clone)]
#[repr(packed, C)]
pub struct DescriptionHeader {
    pub signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

impl DescriptionHeader {
    fn validate(&self, signature: &str) -> bool {
        if signature != bytes2str(&self.signature) {
            error!("XSDT -invalid signature: {}", bytes2str(&self.signature));
            return false;
        }
        let length = self.length as usize;
        let sum = sum_bytes(&self, length);
        if sum != 0 {
            error!("XSDT -sum of {} bytes must be 0: {}", length, sum);
            return false;
        }
        return true;
    }
}

struct Xsdt {
    header: DescriptionHeader,
    entries: Vec<u64>,
}

impl Xsdt {
    unsafe fn new(base: u64) -> Option<Self> {
        let addr = { base as *const DescriptionHeader };
        let header = read_unaligned(addr);
        if !header.validate("XSDT") {
            return None;
        }
        let table_addr = addr.offset(1) as *const u64;
        let length = (header.length as usize - size_of::<DescriptionHeader>()) / 8;
        let mut entries = Vec::with_capacity(length);
        for i in 0..length {
            entries.push(read_unaligned(table_addr.wrapping_add(i)));
        }
        return Some(Self { header, entries });
    }

    fn get_timer(&self) -> Option<FFTimer> {
        let mut fadt = None;
        let mut hpet = None;
        for i in 0..self.entries.len() {
            let signature: &str = unsafe {
                &bytes2str(&read_unaligned(self.entries[i] as *const DescriptionHeader).signature)
            };
            match signature {
                "HPET" => {
                    hpet = Some(self.entries[i]);
                }
                "FACP" => {
                    fadt = Some(self.entries[i]);
                }
                _ => {}
            }
        }
        if hpet == None {
            if fadt == None {
                return None;
            }
            info!("TimerManager -fallback to PM Timer");
            return FFTimer::new(fadt.unwrap());
        }
        return FFTimer::new(hpet.unwrap()); //hpet
    }
}

fn get_rsdp(st: SystemTable<Runtime>) -> Option<RSDP> {
    let table = st.config_table();
    let mut acpi_table: *const RSDP = null();
    for i in 0..table.len() {
        if EFI_ACPI_TABLE_GUID == table[i].guid {
            acpi_table = table[i].address as *const RSDP;
            break;
        }
    }
    if acpi_table.is_null() {
        return None;
    }
    let rsdp = RSDP::new(acpi_table);
    if rsdp.validate() {
        return Some(rsdp);
    } else {
        return None;
    }
}

pub fn initialize_acpi(st: SystemTable<Runtime>) {
    let rsdp = get_rsdp(st).unwrap();
    let xsdt = unsafe { Xsdt::new(rsdp.xsdt_address).unwrap() };
    let fftimer = xsdt.get_timer().unwrap();
    initialize_lapic_itmer(fftimer);
}
