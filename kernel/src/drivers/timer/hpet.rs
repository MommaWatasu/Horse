use crate::{bit_getter, bit_setter, error, info, println};
use super::{
    DescriptionHeader,
    ioapic::*
};

use core::ptr::{read_unaligned, write_unaligned, read, write};
use spin::Mutex;

static HPET_INTERRUPTION: Mutex<bool> = Mutex::new(false);

#[repr(packed, C)]
pub struct HpetAddress {
    address_space_id: u8,
    register_bit_width: u8,
    register_bit_offset: u8,
    reserved: u8,
    pub address: u64
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

#[derive(Copy, Clone)]
pub struct HpetController {
    addr: u64,
    n_timers: u8,
    minimal_tick: u16,
    frequency: u32
}

impl HpetController {
    // time is time in femtoseconds from now to interrupt
    fn one_shot(&self, timer: u64, time: u64) {
        let mut tcc = unsafe { read((self.addr + 0x100 + 0x20 * timer) as *const TCCRegister) };
        tcc.set_type_cnf(0); //non-periodic
        tcc.set_int_enb_cnf(1); //ensure interruption enabled
        tcc.set_int_type_cnf(1);
        unsafe {
            write((self.addr + 0x100 + 0x20 * timer) as *mut TCCRegister, tcc);
            write((self.addr + 0x108 + 0x20 * timer) as *mut u64, read((self.addr + 0xf0) as *const u64) + time);
            while read((self.addr + 0x20) as *const u32) != 1 {}
            write((self.addr + 0x20) as *mut u32, 1);
        }
    }
    // time is time in femtoseconds from now to interrupt
    pub fn periodic(&self, timer: u64, time: u64) {
        let mut tcc = unsafe { read((self.addr + 0x100 + 0x20 * timer) as *const TCCRegister) };
        tcc.set_type_cnf(1); // periodic
        tcc.set_int_enb_cnf(1); //ensure interruption enabled
        tcc.set_int_type_cnf(1);
        tcc.set_val_set_cnf(1);
        unsafe { write((self.addr + 0x100 + 0x20 * timer) as *mut TCCRegister, tcc); }
        if tcc.size_cap() == 1 {
            unsafe {
                let main_counter = read((self.addr + 0xf0) as *const u64);
                write((self.addr + 0x108 + 0x20 * timer) as *mut u64, main_counter + time);
                write((self.addr + 0x108 + 0x20 * timer) as *mut u64, time);
            }
        } else {
            error!("HPET driver supports only 64bit comparator");
        }
    }
    //warning: the minimum tick is 10^-8 seconds! So the real time is time * 10 ns.
    pub fn wait_nano_seconds(&self, time: u64) {
        self.one_shot(0, time);
    }
}

impl Hpet {
    pub fn initialize(&self) -> HpetController {
        let addr = self.address.address;
        //calculate the frequency
        let gcid = unsafe { read(addr as *const GCIDRegister) };
        let frequency = (10_u64.pow(15) / gcid.counter_clk_period() as u64) as u32;
        //save minimal tick
        let minimal_tick = self.minimum_tick;
        //initialize comparator
        let num_timers = gcid.num_tim_cap()+1;
        let mut routes: u32 = 0;
        for i in 0..num_timers {
            let mut tcc = unsafe { read_unaligned((addr + 0x100 + 0x20 * (i as u64)) as *const TCCRegister) };
            tcc.set_fsb_en_cnf(0);
            tcc.set_32mode_cnf(0);
            routes |= tcc.int_route_cap();
            unsafe { write_unaligned((addr + 0x100 + 0x20 * (i as u64)) as *mut TCCRegister, tcc) }
        }
        for i in 0..32 {
            if routes >> i & 1 == 1 {
                configure_redirection_table(i);
            }
        }
        //ensure HPET is enabled
        unsafe {
            let mut gc = read_unaligned((addr + 0x10) as *const GCRegister);
            gc.set_enable_cnf(1);
            write_unaligned((addr + 0x10) as *mut GCRegister, gc);
        }
        info!("Initialize HPET has been done");
        return HpetController {
            addr,
            n_timers: num_timers,
            minimal_tick,
            frequency
        }
    }
}

//General Capability and ID Register
#[repr(packed, C)]
struct GCIDRegister {
    data: u64
}

impl GCIDRegister {
    bit_getter!(data: u64; 0xffffffff00000000; u32, pub counter_clk_period);
    bit_getter!(data: u64; 0x0000000000002000; u8, pub count_size_cap);
    bit_getter!(data: u64; 0x0000000000001f00; u8, pub num_tim_cap);
}

//General Configuration Register
#[repr(packed, C)]
struct GCRegister {
    pub data: u64
}

impl GCRegister {
    bit_setter!(data: u64; 0x0000000000000001; u8, pub set_enable_cnf);
}

//Timer Comparator and Capability Register
#[derive(Copy, Clone)]
struct TCCRegister {
    data: u64
}

impl TCCRegister {
    bit_getter!(data: u64; 0xffffffff00000000; u32, pub int_route_cap);
    bit_getter!(data: u64; 0x0000000000008000; u8, pub fsb_int_del_cap);
    bit_getter!(data: u64; 0x0000000000000020; u8, pub size_cap);
    bit_getter!(data: u64; 0x0000000000000010; u8, pub per_int_cap);

    bit_setter!(data: u64; 0x0000000000004000; u8, pub set_fsb_en_cnf);
    bit_setter!(data: u64; 0x0000000000003e00; u8, pub set_int_route_cnf);
    bit_setter!(data: u64; 0x0000000000000100; u8, pub set_32mode_cnf);
    bit_setter!(data: u64; 0x0000000000000040; u8, pub set_val_set_cnf);
    bit_setter!(data: u64; 0x0000000000000008; u8, pub set_type_cnf);
    bit_setter!(data: u64; 0x0000000000000004; u8, pub set_int_enb_cnf);
    bit_setter!(data: u64; 0x0000000000000002; u8, pub set_int_type_cnf);
}