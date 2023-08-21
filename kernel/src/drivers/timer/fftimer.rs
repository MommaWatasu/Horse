use crate::error;
use super::{PM_TIMER_FREQ, DescriptionHeader};

use alloc::string::String;
use core::{
    mem::size_of,
    ptr::read_unaligned
};

#[repr(packed, C)]
struct HpetAddress {
    address_space_id: u8,
    register_bit_width: u8,
    register_bit_offset: u8,
    reserved: u8,
    address: u64
}

#[repr(packed, C)]
pub struct Hpet {
    header: DescriptionHeader,
    hardware_rev_id: u8,
    divbits: u8,
    pci_vender_id: u16,
    address: HpetAddress,
    hpet_number: u8,
    minimum_tick: u16,
    page_protection: u8
}

//Frequency Fixed Timer
pub enum FFTimer{ 
    HPET(Hpet),
    PM(PMTimer)
}

#[repr(packed, C)]
pub struct PMTimer {
    header: DescriptionHeader,
    reserved1: [u8; 76-size_of::<DescriptionHeader>()],
    pm_tmr_blk: u32,
    reserved2: [u8; 112-80],
    flags: u32,
    reserved3: [u8; 276-116]
}

impl FFTimer {
    pub fn new(ptr: u64) -> Option<Self> {
        let signature: &str = unsafe { &Self::bytes2str(&read_unaligned(ptr as *const DescriptionHeader).signature) };
        match signature {
            //"HPET" => {return initialize_hpet(ptr)},
            "FACP" => {return Some(Self::initialize_pmtimer(ptr))},
            _ => {
                error!("FFTimer must be HPET or FACP");
                return None
            }
        }
    }
    fn bytes2str(bytes: &[u8]) -> String {
        return String::from_utf8(bytes.to_vec()).unwrap();
    }
    //fn initialize_hpet(ptr: u64) -> Self {}
    fn initialize_pmtimer(ptr: u64) -> Self {
        return unsafe { FFTimer::PM(read_unaligned(ptr as *const PMTimer)) }
    }
    pub fn initialize_lapic_itmer(&self) {}
    pub fn wait_milliseconds(&self, msec: u32) {
        match self {
            Self::HPET(hpet) => {},
            Self::PM(fadt) => {
                let pm_tmr_blk: u16 = fadt.pm_tmr_blk.try_into().unwrap();
                let pm_timer_32 = 1 == (fadt.flags >> 8) & 1;
                let start: u32 = unsafe { IoIn32(pm_tmr_blk) };
                let mut end: u32 = start + PM_TIMER_FREQ * msec / 1000;
                if !pm_timer_32 { end &= 0x00ffffff }
                if end < start {
                    while unsafe { IoIn32(pm_tmr_blk) } >= start {}
                }
                while unsafe { IoIn32(pm_tmr_blk) } < end {}
            },
            _ => { error!("timer must be PM Timer or HPET"); }
        }
    }
}

//assembly function in asm.s
extern "C" {
    // this function is used for wait milliseconds, and this function is unsafe
    fn IoIn32(addr: u16) -> u32;
}