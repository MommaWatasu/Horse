use crate::volatile::Volatile;
use crate::{bit_getter, bit_setter};

#[repr(C)]
pub struct HcsParam1 {
    data: u32,
}

impl HcsParam1 {
    // RO
    bit_getter!(data: u32; 0x000000FF; u8, pub max_device_slots);
    // RO
    bit_getter!(data: u32; 0xFF000000; u8, pub max_ports);
}

#[repr(C)]
pub struct HcsParam2 {
    data: u32,
}

impl HcsParam2 {
    // RO
    bit_getter!(data: u32; 0x03E00000; u8, max_scratchpad_bufs_hi);
    bit_getter!(data: u32; 0xF8000000; u8, max_scratchpad_bufs_lo);

    pub fn max_scratchpad_buf(&self) -> usize {
        let hi = self.max_scratchpad_bufs_hi() as usize;
        let lo = self.max_scratchpad_bufs_lo() as usize;
        (hi << 5) | lo
    }
}

#[repr(C)]
pub struct HcsParam3 {
    data: u32,
}

#[repr(C)]
pub struct HccParams1 {
    data: u32,
}

impl HccParams1 {
    // xhci extended capabilities pointer
    bit_getter!(data: u32; 0xFFFF0000; u16, pub xecp);
}

#[repr(C)]
pub struct Dboff {
    data: u32,
}
impl Dboff {
    bit_getter!(data: u32; 0xFFFFFFFC; u32, doorbell_array_offset);

    pub fn offset(&self) -> usize {
        (self.doorbell_array_offset() << 2) as usize
    }
}

#[repr(C)]
pub struct Rtsoff {
    data: u32,
}
impl Rtsoff {
    bit_getter!(data: u32; 0xFFFFFFE0; u32, runtime_register_space_offset);

    pub fn offset(&self) -> usize {
        (self.runtime_register_space_offset() << 5) as usize
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

    bit_setter!(data: u32; 0b0100; u8, pub set_interrupter_enable);
    bit_getter!(data: u32; 0b0100; u8, pub interrupter_enable);

    //bit_setter!(data: u32; 0b1000; u8, pub set_host_system_error_enable);
    //bit_getter!(data: u32; 0b1000; u8, pub host_system_error_enable);
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
    data: u32,
}

impl PageSize {
    bit_getter!(data: u32; 0x0000FFFF; u32, pub page_size_shifted);

    pub fn page_size(&self) -> usize {
        1 << (self.page_size_shifted() as usize + 12)
    }
}

#[repr(C)]
pub struct Crcr {
    data: u64,
}

impl Crcr {
    bit_getter!(data: u64; 0x0000000000000001;  u8, pub ring_cycle_state);
    bit_setter!(data: u64; 0x0000000000000001;  u8, pub set_ring_cycle_state);

    bit_getter!(data: u64; 0x0000000000000002;  u8, pub command_stop);
    bit_setter!(data: u64; 0x0000000000000002;  u8, pub set_command_stop);

    bit_getter!(data: u64; 0x0000000000000004;  u8, pub command_abort);
    bit_setter!(data: u64; 0x0000000000000004;  u8, pub set_command_abort);

    bit_getter!(data: u64; 0xFFFFFFFFFFFFFFC0;  u64, pub command_ring_pointer);
    bit_setter!(data: u64; 0xFFFFFFFFFFFFFFC0;  u64, pub set_command_ring_pointer);

    pub fn pointer(&self) -> usize {
        (self.command_ring_pointer() as usize) << 6
    }
    pub fn set_pointer(&mut self, ptr: usize) {
        let ptr = ((ptr & 0xFFFFFFFFFFFFFFC0) >> 6) as u64;
        self.set_command_ring_pointer(ptr);
    }
}

#[derive(Default)]
#[repr(C)]
pub struct Dcbaap {
    data: u64,
}

impl Dcbaap {
    bit_getter!(data: u64; 0xFFFFFFFFFFFFFFC0; u64, device_context_base_address_array_pointer);
    bit_setter!(data: u64; 0xFFFFFFFFFFFFFFC0; u64, set_device_context_base_address_array_pointer);

    pub fn pointer(&self) -> usize {
        (self.device_context_base_address_array_pointer() as usize) << 6
    }
    pub fn set_pointer(&mut self, ptr: usize) {
        let ptr = ((ptr & 0xFFFFFFFFFFFFFFC0) >> 6) as u64;
        self.set_device_context_base_address_array_pointer(ptr);
    }
}

#[repr(C)]
pub struct Config {
    data: u32,
}

impl Config {
    bit_getter!(data: u32; 0xFF; u8, pub max_device_slots_enabled);
    bit_setter!(data: u32; 0xFF; u8, pub set_max_device_slots_enabled);
}

#[repr(C)]
pub struct ExtendedRegister {
    data: u32,
}

impl ExtendedRegister {
    bit_getter!(data: u32; 0x00FF; u8, pub capability_id);

    bit_getter!(data: u32; 0xFF09; u8, pub next_capability_pointer);
}

#[repr(C)]
pub struct Usblegsup {
    data: u32,
}

impl Usblegsup {
    pub const fn id() -> u8 {
        1
    }

    bit_getter!(data: u32; 0x000000FF; u8, pub capability_id);

    bit_getter!(data: u32; 0x0000FF00; u8, pub next_capability_pointer);

    bit_getter!(data: u32; 0x00010000; u8, pub hc_bios_owned_semaphore);
    bit_setter!(data: u32; 0x00010000; u8, pub set_hc_bios_owned_semaphore);

    bit_getter!(data: u32; 0x01000000; u8, pub hc_os_owned_semaphore);
    bit_setter!(data: u32; 0x01000000; u8, pub set_hc_os_owned_semaphore);
}

#[repr(C)]
pub struct Iman {
    data: u32,
}

impl Iman {
    bit_getter!(data: u32; 0x00000001; u8, pub interrupt_pending);
    bit_setter!(data: u32; 0x00000001; u8, pub set_interrupt_pending);

    bit_getter!(data: u32; 0x00000002; u8, pub interrupter_enable);
    bit_setter!(data: u32; 0x00000002; u8, pub set_interrupter_enable);
}

#[repr(C)]
pub struct Imod {
    data: u32,
}

#[repr(C)]
pub struct Erstsz {
    data: u32,
}

impl Erstsz {
    bit_getter!(data: u32; 0x0000FFFF; u16, pub event_ring_segment_table_size);
    bit_setter!(data: u32; 0x0000FFFF; u16, pub set_event_ring_segment_table_size);
}

#[repr(C)]
pub struct Erstba {
    data: u64,
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
    data: u64,
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
    pub data: u32,
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
    data: u32,
}

#[repr(C)]
pub struct PortLi {
    data: u32,
}

#[repr(C)]
pub struct PortHlpmc {
    pub data: u32,
}

#[repr(C)]
pub struct Doorbell {
    data: u32,
}

impl Doorbell {
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
    pub hcs_params3: Volatile<HcsParam3>,
    pub hcc_params1: Volatile<HccParams1>,
    pub db_off: Volatile<Dboff>,
    pub rts_off: Volatile<Rtsoff>,
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
    pub config: Volatile<Config>,
}

#[repr(C, packed(4))]
pub struct InterrupterRegisterSet {
    pub iman: Volatile<Iman>,
    pub imod: Volatile<Imod>,
    pub erstsz: Volatile<Erstsz>,
    reserved: u32,
    pub erstba: Volatile<Erstba>,
    pub erdp: Volatile<Erdp>,
}

#[repr(C, packed(4))]
pub struct PortRegisterSet {
    pub portsc: Volatile<PortSc>,
    pub portpmsc: Volatile<PortPmsc>,
    pub portli: Volatile<PortLi>,
    pub porthlpmc: Volatile<PortHlpmc>,
}

#[repr(C)]
pub struct DoorbellRegister {
    reg: Volatile<Doorbell>,
}
impl DoorbellRegister {
    pub fn ring(&mut self, target: u8) {
        self.ring_with_stream_id(target, 0)
    }
    pub fn ring_with_stream_id(&mut self, target: u8, stream_id: u16) {
        self.reg.modify(|reg| {
            reg.set_db_target(target);
            reg.set_db_stream_id(stream_id);
        })
    }
}
