use crate::volatile::Volatile;
use crate::{bit_getter, bit_setter};
use crate::register::ArrayWrapper;
use crate::debug;

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
    bit_getter!(data: u32; 0xFFFF0000; u16, pub xecp);
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

#[repr(C)]
pub struct UsbCmd {
    data: u32,
}

impl UsbCmd {
    bit_setter!(data: u32; 0b0001; u8, pub set_run_stop);
    bit_getter!(data: u32; 0b0001; u8, pub run_stop);
    
    bit_setter!(data: u32; 0b0010; u8, pub set_host_controller_reset);
    bit_getter!(data: u32; 0b0010; u8, pub host_controller_reset);
    
    bit_setter!(data: u32; 0b0100; u8, pub set_intterupt_enable);
    bit_getter!(data: u32; 0b0100; u8, pub intterupt_enable);
    
    bit_setter!(data: u32; 0b1000; u8, pub set_host_system_error_enable);
    bit_getter!(data: u32; 0b1000; u8, pub host_system_error_enable);
    //bit_setter!(data: u32; 10, pub set_enable_wrap_event);
    //bit_getter!(data: u32; 10, pub enable_wrap_event);
}

#[repr(C)]
pub struct UsbSts {
    data: u32,
}

impl UsbSts {
    bit_getter!(data:u32; 0x00000001; u8, pub host_controller_halted);
    bit_getter!(data:u32; 0x00000800; u8, pub controller_not_ready);
}

#[repr(C)]
pub struct PageSize {
    data: u32
}

impl PageSize {
    bit_getter!(data: u32; 0x0000FFFF; u32, pub page_size_shifted);
    
    pub fn page_size(&self) -> usize {
        1 << (self.page_size_sifted() as usize + 12)
    }
}

#[repr(C)]
pub struct Crcr {
    data: u64
}

impl Crcr {
    bit_getter!(data: u64; 0x0000000000000001;  u8, pub ring_cycle_state);
    bit_setter!(data: u64; 0x0000000000000001;  u8, pub set_ring_cycle_state);
    
    bit_getter!(data: u64; 0x0000000000000002;  u8, pub command_stop);
    bit_setter!(data: u64; 0x0000000000000002;  u8, pub set_command_stop);
    
    bit_getter!(data: u64; 0x0000000000000004;  u8, pub command_abort);
    bit_setter!(data: u64; 0x0000000000000004;  u8, pub set_command_abort);
    
    bit_getter!(data: u64; 0xFFFFFFFFFFFFFFC0;  u8, pub command_ring_pointer);
    bit_setter!(data: u64; 0xFFFFFFFFFFFFFFC0;  u8, pub set_command_ring_pointer);
    
    pub fn pointer(&self) -> usize {
        (self.command_ring_pointer() as usize) << 6
    }
    pub fn set_pointer(&self, ptr: usize) {
        let ptr = ((ptr & 0xFFFFFFFFFFFFFFC0) >> 6) as u64;
        self.set_command_ring_pointer(ptr.try_into().unwrap());
    }
}

#[derive(Default)]
#[repr(C)]
pub struct Dcbaap {
    data: u64
}

impl Dcbaap {
    bit_getter!(data: u64; 0xFFFFFFFFFFFFFFC0; u64, device_context_base_address_array_pointer);
    bit_setter!(data: u64; 0xFFFFFFFFFFFFFFC0; u64, set_device_context_base_address_array_pointer);
    
    pub fn pointer(&self) -> usize {
        (self.device_context_base_address_array_pointer() as usize) << 6
    }
    pub fn set_pointer(&self, ptr: usize) {
        let ptr = ((ptr & 0xFFFFFFFFFFFFFFC0) >> 6) as u64;
        self.set_device_context_base_address_array_pointer(ptr);
    }
}

#[repr(C)]
pub struct Config {
    data: u32
}

impl Config {
    bit_getter!(data: u32; 0xFF; u8, pub max_device_slots_enabled);
    bit_setter!(data: u32; 0xFF; u8, pub set_max_device_slots_enabled);
}

#[repr(C)]
pub struct ExtendedRegister {
    data: u32
}

impl ExtendedRegister {
    bit_getter!(data: u32; 0x00FF; u8, pub capability_id);
    
    bit_getter!(data: u32; 0xFF09; u8, pub next_capability_pointer);
}

#[repr(C)]
pub struct Iman {
    data: u32
}

impl Iman {
    bit_getter!(data: u32; 0x00000001; u8, pub interrupt_pending);
    bit_setter!(data: u32; 0x00000001; u8, pub set_interrupt_pending);
    
    bit_getter!(data: u32; 0x00000002; u8, pub interrupter_enable);
    bit_setter!(data: u32; 0x00000002; u8, pub set_interrupter_enable);
}

#[repr(C)]
pub struct Imod {
    data: u32
}

#[repr(C)]
pub struct Erstsz {
    data: u32
}

impl Erstsz {
    bit_getter!(data: u32; 0x0000FFFF; u16, pub event_ring_segment_table_size);
    bit_setter!(data: u32; 0x0000FFFF; u16, pub set_event_ring_segment_table_size);
}

#[repr(C)]
pub struct Erstba {
    data: u64
}

impl Erstba {
    bit_getter!(data: u64; 0xFFFFFFFFFFFFFFC0; u64, pub event_ring_segment_table_base_address);
    bit_setter!(data: u64; 0xFFFFFFFFFFFFFFC0; u64, pub set_event_ring_segment_table_base_address);
    
    pub fn pointer(&self) -> usize {
        (self.event_ring_segment_table_base_address() << 6) as usize
    }
    
    pub fn set_pointer(&mut self, ptr: usize) {
        self.set_event_ring_segment_table_base_address((ptr as u64) >> 6)
    }
}

#[repr(C)]
pub struct Erdp {
    data: u64
}

impl Erdp {
    bit_getter!(data: u64; 0xFFFFFFFFFFFFFFF0; u64, event_ring_dequeue_pointer);
    bit_setter!(data: u64; 0xFFFFFFFFFFFFFFF0; u64, set_event_ring_dequeue_pointer);
    
    pub fn pointer(&self) -> usize {
        (self.event_ring_dequeue_pointer() << 4) as usize
    }
    pub fn set_pointer(&mut self, ptr: usize) {
        self.set_event_ring_dequeue_pointer((ptr as u64) >> 4)
    }
}

#[repr(C)]
pub struct PortSc {
    pub data: u32
}

impl PortSc {
    bit_getter!(data: u32; 0x00000001; u8, pub current_connect_status);
    
    bit_getter!(data: u32; 0x00000002; u8, pub port_enabled_disabled);
    
    bit_getter!(data: u32; 0x00000010; u8, pub port_reset);
    
    bit_getter!(data: u32; 0x00003C00; u8, pub port_speed);
    
    bit_getter!(data: u32; 0x00020000; u8, pub connect_status_change);
    bit_setter!(data: u32; 0x00020000; u8, pub set_connect_status_change);
    
    bit_getter!(data: u32; 0x00200000; u8, pub port_reset_change);
    bit_setter!(data: u32; 0x00200000; u8, pub set_port_reset_change);
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

#[repr(C)]
pub struct DoorbellRegister {
    data: u32
}

impl DoorbellRegister {
    bit_setter!(data: u32; 0x000000FF;  u8, pub set_db_target);
    bit_setter!(data: u32; 0xFFFF0000; u16, pub set_db_stream_id);
}

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

#[repr(C, packed(4))]
pub struct OperationalRegisters {
    pub usbcmd: Volatile<UsbCmd>,
    pub usbsts: Volatile<UsbSts>,
    pub pagesize: Volatile<PageSize>,
    pub reserved1: [u32; 2],
    pub dnctrl: Volatile<u32>,
    pub crcr: Volatile<Crcr>,
    pub reserved2: [u32; 4],
    pub dcbaap: Volatile<Dcbaap>,
    pub config: Volatile<Config>
}

#[repr(C, packed(4))]
pub struct InterrupterRegisterSet {
    pub iman: Volatile<Iman>,
    pub imod: Volatile<Imod>,
    pub erstsz: Volatile<Erstsz>,
    reserved: u32,
    pub erstba: Volatile<Erstba>,
    erdp: Volatile<Erdp>
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
        let array = array_base_addr as *mut PortRegisterSet;
        return Self{
            array,
            size
        };
    }
    
    pub unsafe fn index(&self, idx: usize) -> *mut PortRegisterSet {
        unsafe {
            return self.array.offset(idx.try_into().unwrap());
        }
    }
}