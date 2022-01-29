use crate::volatile::Volatile;
use crate::{bit_getter, bit_setter};
use crate::register::ArrayWrapper;
use core::ops::Index;
use crate::debug;
#[repr(C, packed(4))]
pub struct CapabilityRegisters {
    pub cap_length: Volatile<u8>,
    reserved: u8,
    pub hci_version: Volatile<u16>,
    pub hcs_params1: Volatile<HcsParam1>,
    pub hcs_params2: Volatile<HcsParam2>,
    pub hcs_params3: Volatile<u32>,
    pub hcc_params1: Volatile<HccParams1>,
    pub db_off: Volatile<u32>,
    pub rts_off: Volatile<u32>,
    pub hcc_params2: Volatile<u32>,
}

impl core::fmt::Display for CapabilityRegisters {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "cap_length: {}, hci_version: 0x{:02x}, hcs_params1: {}, hcs_params2: {}, hcs_params3: 0x{:08x}, hcc_params1: {}, db_off: 0x{:08x}, rts_off: 0x{:08x}, hcc_params2: 0x{:08x}",
            self.cap_length.read(),
            self.hci_version.read(),
            self.hcs_params1.read(),
            self.hcs_params2.read(),
            self.hcs_params3.read(),
            self.hcc_params1.read(),
            self.db_off.read() & 0xffff_fffc,
            self.rts_off.read() & 0xffff_ffe0,
            self.hcc_params2.read()
        )
    }
}

#[repr(C)]
pub struct HcsParam1 {
    data: u32,
}

impl HcsParam1 {
    pub fn max_device_slots(&self) -> u8 {
        (self.data & 0xff) as u8
    }

    pub fn max_ports(&self) -> u8 {
        (self.data >> 24 & 0xff) as u8
    }
}

impl core::fmt::Display for HcsParam1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "0x{:08x} (slots: {} ports: {})",
            self.data,
            self.max_device_slots(),
            self.max_ports()
        )
    }
}

#[repr(C)]
pub struct HcsParam2 {
    data: u32,
}

impl HcsParam2 {
    pub fn max_scratchpad_buf(&self) -> usize {
        let hi = (self.data >> 21 & 0b11111) as usize;
        let lo = (self.data >> 27 & 0b11111) as usize;
        (hi << 5) | lo
    }
}

impl core::fmt::Display for HcsParam2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "0x{:08x} (max_scratchpad_buf: {})",
            self.data,
            self.max_scratchpad_buf()
        )
    }
}

#[repr(C)]
pub struct HccParams1 {
    data: u32,
}

impl HccParams1 {
    pub fn xecp(&self) -> u16 {
        (self.data >> 16) as u16
    }
    bit_getter!(data: u32; 2, context_size);
}

impl core::fmt::Display for HccParams1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "0x{:08x} (xECP: 0x{:08x}, CSZ: {})",
            self.data,
            self.xecp(),
            self.context_size()
        )
    }
}

#[repr(C, packed(4))]
pub struct OperationalRegisters {
    pub usbcmd: Volatile<UsbCmd>,
    pub usbsts: Volatile<UsbSts>,
    pub pagesize: u32,
    pub reserved1: [u32; 2],
    pub dnctrl: u32,
    pub crcr: u64,
    pub reserved2: [u32; 4],
    pub dcbaap: u64,
    pub config: Volatile<ConfigRegister>,
}

#[repr(C)]
pub struct UsbCmd {
    data: u32,
}

impl UsbCmd {
    bit_setter!(data: u32; 0, pub set_run_stop);
    bit_getter!(data: u32; 0, pub run_stop);
    bit_setter!(data: u32; 1, pub set_host_controller_reset);
    bit_getter!(data: u32; 1, pub host_controller_reset);
    bit_setter!(data: u32; 2, pub set_intterupt_enable);
    bit_getter!(data: u32; 2, pub intterupt_enable);
    bit_setter!(data: u32; 3, pub set_host_system_error_enable);
    bit_getter!(data: u32; 3, pub host_system_error_enable);
    bit_setter!(data: u32; 10, pub set_enable_wrap_event);
    bit_getter!(data: u32; 10, pub enable_wrap_event);
}

#[repr(C)]
pub struct UsbSts {
    data: u32,
}

impl UsbSts {
    bit_getter!(data:u32; 0, pub host_controller_halted);
    bit_getter!(data:u32; 11, pub controller_not_ready);
}

#[repr(C)]
pub struct PortSc {
    pub data: u32
}

impl PortSc {
    bit_getter!(data: u32; 0, pub current_connect_status);
    bit_getter!(data: u32; 4, pub port_reset);
}

#[repr(C)]
pub struct PortPmsc {
    data: u32
}

#[repr(C)]
pub struct PortLi {
    data: u32
}

#[repr(C)]
pub struct PortHlpmc {
    pub data: u32
}

#[repr(C, packed(4))]
pub struct PortRegisterSet {
    pub portsc: Volatile<PortSc>,
    pub portpmsc: Volatile<PortPmsc>,
    pub portli: Volatile<PortLi>,
    pub porthlpmc: Volatile<PortHlpmc>
}

pub type PortRegisterSets = ArrayWrapper<PortRegisterSet>;

impl PortRegisterSets {
    pub unsafe fn new(array_base_addr: usize, size: usize) -> Self {
        let array = &mut **(array_base_addr as *mut *mut PortRegisterSet);
        return Self{
            array,
            size
        };
    }
    
    pub fn index(&self, idx: usize) -> *mut PortRegisterSet {
        return (self.array as usize + idx) as *mut PortRegisterSet;
    }
}

#[repr(C)]
pub struct ConfigRegister {
    data: u32,
}

impl ConfigRegister {
    pub fn set_max_device_slots_enabled(&mut self, val: u8) {
        self.data |= val as u32;
    }

    pub fn max_device_slots_enabled(&mut self) -> u8 {
        (self.data & 0xff) as u8
    }
}

pub struct Doorbell {
    data: u32,
}

impl Doorbell {
    pub fn set_db_target(&mut self, target: u8) {
        self.data = self.data & 0xffff_fff0 | target as u32;
    }

    pub fn set_db_stream_id(&mut self, stream_id: u16) {
        self.data = self.data & 0x0000_ffff | (stream_id as u32) << 16;
    }
}
