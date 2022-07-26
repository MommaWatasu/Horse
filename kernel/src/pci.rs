use core::fmt::Display;
use x86_64::instructions::port::{PortReadOnly, PortWriteOnly};
use crate::{
    status::StatusCode,
    bit_getter, bit_setter,
    asmfunc::{
        ioin, ioout
    }
};

const CONFIG_DATA: usize = 0x0cfc;

fn WriteData(value: u32) {
    ioout(CONFIG_DATA, value);
}

fn ReadData() -> u32 { ioin(CONFIG_DATA) }

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ClassCode {
    pub base: u8,
    pub sub: u8,
    pub interface: u8
}

impl Display for ClassCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "(0x{:02x}, 0x{:02x}, 0x{:02x})",
            self.base, self.sub, self.interface
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Device {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub header_type: u8,
    pub class_code: ClassCode
}

impl Device {
    pub fn get_vendor_id(&self) -> u16 {
        read_vendor_id(self.bus, self.device, self.function)
    }
}

const EMPTY_DEVICE: Device = Device{
    bus: 0xde,
    device: 0xad,
    function: 0xbe,
    header_type: 0xef,
    class_code: ClassCode{
        base: 0,
        sub: 0,
        interface: 0
    }
};

static PCI_PORT: spin::Mutex<PciIOPort> = spin::Mutex::new(PciIOPort::new());

pub struct PciDevices {
    devices: [Device; 32],
    count: usize
}

pub struct PciDevicesIter<'a> {
    devices: &'a [Device],
    index: usize,
}

impl PciDevices {
    const fn new() -> Self {
        Self {
            devices: [EMPTY_DEVICE; 32],
            count: 0
        }
    }

    pub fn add_device(&mut self, device: Device) -> Result<StatusCode, StatusCode> {
        if self.count > 32 {
            Err(StatusCode::Full)
        } else {
            self.devices[self.count] = device;
            self.count += 1;
            Ok(StatusCode::Success)
        }
    }

    fn scan_function(&mut self, bus: u8, device: u8, function: u8) -> Result<StatusCode, StatusCode> {
        let header_type = read_header_type(bus, device, function);
        let class_code = read_class_code(bus, device, function);
        self.add_device(Device{
            bus,
            device,
            function,
            header_type,
            class_code
        })?;
        if class_code.base == 0x06 && class_code.sub == 0x04 {
            let bus_numbers = read_bus_numbers(bus, device, function);
            let secondary_bus = ((bus_numbers >> 8) & 0xff) as u8;
            return self.scan_bus(secondary_bus);
        }
        Ok(StatusCode::Success)
    }

    fn scan_device(&mut self, bus: u8, device: u8) -> Result<StatusCode, StatusCode> {
        self.scan_function(bus, device, 0)?;
        if !is_singleton_function_device(read_header_type(bus, device, 0)) {
            for function in 1..8 {
                if read_vendor_id(bus, device, u8::from(function)) != 0xffff {
                    self.scan_function(bus, device, u8::from(function))?;
                }
            }
        }
        Ok(StatusCode::Success)
    }

    fn scan_bus(&mut self, bus: u8) -> Result<StatusCode, StatusCode> {
        for device in 0..32 {
            if read_vendor_id(bus, device, 0) != 0xffff {
                self.scan_device(bus, device)?;
            }
        }
        Ok(StatusCode::Success)
    }

    pub fn iter(&self) -> PciDevicesIter {
        PciDevicesIter {
            devices: &self.devices[..self.count],
            index: 0,
        }
    }
}

impl<'a> Iterator for PciDevicesIter<'a> {
    type Item = Device;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.devices.len() {
            None
        } else {
            let r = self.devices[self.index];
            self.index += 1;
            Some(r)
        }
    }
}

fn read_msi_capability(dev: &Device, cap_addr: u8) -> MSICapability {
    let mut msi_cap = MSICapability::default();
    
    msi_cap.data = read_conf_reg(dev, cap_addr);
    msi_cap.msg_addr = read_conf_reg(dev, cap_addr+4);
    
    let mut msg_data_addr = cap_addr+8;
    if msi_cap.addr_64_capable() != 0 {
        msi_cap.msg_upper_addr = read_conf_reg(dev, msg_data_addr);
        msg_data_addr += 4;
    }
    
    msi_cap.msg_data = read_conf_reg(dev, msg_data_addr);
    
    if msi_cap.per_vector_mask_capable() != 0 {
        msi_cap.mask_bits = read_conf_reg(dev, msg_data_addr + 4);
        msi_cap.pending_bits = read_conf_reg(dev, msg_data_addr + 8)
    }
    return msi_cap;
}

fn write_msi_capability(
    dev: &Device,
    cap_addr: u8,
    msi_cap: &MSICapability
) {
    write_conf_reg(dev, cap_addr, msi_cap.data);
    write_conf_reg(dev, cap_addr+4, msi_cap.msg_addr);
    
    let mut msg_data_addr = cap_addr + 8;
    if msi_cap.addr_64_capable() != 0 {
        write_conf_reg(dev, cap_addr+8, msi_cap.msg_upper_addr);
        msg_data_addr = cap_addr + 12;
    }
    
    write_conf_reg(dev, msg_data_addr, msi_cap.msg_data);
    
    if msi_cap.per_vector_mask_capable() != 0 {
        write_conf_reg(dev, msg_data_addr + 4, msi_cap.mask_bits);
        write_conf_reg(dev, msg_data_addr + 8, msi_cap.pending_bits);
    }
}

struct PciIOPort {
    address_port: PortWriteOnly<u32>,
    data_port: PortReadOnly<u32>
}

impl PciIOPort {
    const fn new() -> Self {
        Self {
            address_port: PortWriteOnly::new(0xcf8),
            data_port: PortReadOnly::new(0xcfc),
        }
    }

    fn makeaddress(bus: u8, device: u8, function: u8, reg_addr: u8) -> u32 {
        return 1u32 << 31
            | u32::from(bus)<<16
            | u32::from(device)<<11
            | u32::from(function)<<8
            | u32::from(reg_addr & 0xfcu8)
    }

    pub fn read(&mut self, bus: u8, device: u8, function: u8, reg_addr: u8) -> u32 {
        let addr = PciIOPort::makeaddress(bus, device, function, reg_addr);
        unsafe {
            self.address_port.write(addr);
            self.data_port.read()
        }
    }

    pub fn read_dev(&mut self, dev: &Device, reg_addr: u8) -> u32 {
        self.read(dev.bus, dev.device, dev.function, reg_addr)
    }
}

pub fn read_vendor_id(bus: u8, device: u8, function: u8) -> u16 {
    (PCI_PORT.lock().read(bus, device, function, 0x18)) as u16
}

pub fn read_header_type(bus: u8, device: u8, function: u8) -> u8 {
    (PCI_PORT.lock().read(bus, device, function, 0x0c) >> 16 & 0xff) as u8
}

pub fn read_class_code(bus: u8, device: u8, function: u8) -> ClassCode {
    let r = PCI_PORT.lock().read(bus, device, function, 0x08);
    ClassCode {
        base: ((r >> 24) & 0xff) as u8,
        sub: ((r >> 16) & 0xff) as u8,
        interface: ((r >> 8) & 0xff) as u8,
    }
}

pub fn read_bus_numbers(bus: u8, device: u8, function: u8) -> u32 {
    PCI_PORT.lock().read(bus, device, function, 0x18)
}

fn calc_bar_address(bar_index: usize) -> u8 {
    (0x10 + 4 * bar_index) as u8
}

fn read_conf_reg(dev: &Device, reg_addr: u8) -> u32 {
    PCI_PORT.lock().read_dev(dev, reg_addr);
    return ReadData();
}

fn write_conf_reg(dev: &Device, reg_addr: u8, value: u32) {
    PCI_PORT.lock().read_dev(dev, reg_addr);
    WriteData(value);
}

fn read_capability_header(dev: &Device, addr: u8) -> CapabilityHeader {
    let mut header = CapabilityHeader::default();
    header.data = read_conf_reg(dev, addr);
    return header;
}

pub fn read_bar(device: &Device, bar_index: usize) -> Result<u64, StatusCode> {
    if bar_index >= 6 {
        return Err(StatusCode::IndexOutOfRange);
    }
    let addr: u8 = calc_bar_address(bar_index);
    let bar: u32 = read_conf_reg(device, addr);

    if (bar & 4) == 0 {
        return Ok(bar.into());
    }

    if bar_index >= 5 {
        return Err(StatusCode::IndexOutOfRange);
    }

    let bar_upper: u32 = PCI_PORT.lock().read(device.bus, device.device, device.function, u8::from(addr+4));
    return Ok(bar as u64 | (bar_upper as u64) << 32);
}

pub fn is_singleton_function_device(header_type: u8) -> bool {
    header_type & 0x80 == 0
}

pub fn scan_all_bus() -> Result<PciDevices, StatusCode> {
    let mut pci_devices: PciDevices = PciDevices::new();
    let header_type = read_header_type(0, 0, 0);
    if is_singleton_function_device(header_type) {
        pci_devices.scan_bus(0)?;
    }

    for function in 1..8 {
        if read_vendor_id(0, 0, function) == 0xffffu16 {
            continue;
        }
        pci_devices.scan_bus(function)?;
    }
    return Ok(pci_devices);
}

fn configure_msi_register(
    dev: &Device,
    cap_addr: u8,
    msg_addr: u32,
    msg_data: u32,
    num_vector_exponent: u8
) -> StatusCode {
    let mut msi_cap = read_msi_capability(dev, cap_addr);
    
    if msi_cap.multi_msg_capable() <= num_vector_exponent {
        msi_cap.set_multi_msg_enable(msi_cap.multi_msg_capable());
    } else {
        msi_cap.set_multi_msg_enable(num_vector_exponent);
    }
    
    msi_cap.set_msi_enable(1);
    msi_cap.msg_addr = msg_addr;
    msi_cap.msg_data = msg_data;
    
    write_msi_capability(dev, cap_addr, &msi_cap);
    return StatusCode::Success
}

fn configure_msix_register(
    dev: &Device,
    cap_addr: u8,
    msg_addr: u32,
    msg_data: u32,
    num_vector_exponent: u8
) -> StatusCode {
    return StatusCode::NotImplemented;
}

#[repr(C)]
#[derive(Default)]
struct CapabilityHeader {
    data: u32,
}

impl CapabilityHeader {
    bit_getter!(data: u32; 0x000000FF; u8, cap_id);
    bit_getter!(data: u32; 0x0000FF00; u8, next_ptr);
}

const CAPABILITY_MSI: u8 = 0x05;
const CAPABILITY_MSIX: u8 = 0x11;

#[repr(C)]
#[derive(Default)]
struct MSICapability {
    data: u32,
    msg_addr: u32,
    msg_upper_addr: u32,
    msg_data: u32,
    mask_bits: u32,
    pending_bits: u32
}

impl MSICapability {
    bit_getter!(data: u32; 0x10000000; u8, per_vector_mask_capable);
    bit_getter!(data: u32; 0x08000000; u8, addr_64_capable);
    bit_getter!(data: u32; 0x000E0000; u8, multi_msg_capable);
    bit_setter!(data: u32; 0x00010000; u8, set_msi_enable);
    bit_setter!(data: u32; 0x00700000; u8, set_multi_msg_enable);
}

#[derive(PartialEq)]
pub enum MSITriggerMode {
    Edge = 0,
    Level = 1
}

pub enum MSIDeliveryMode {
    Fixed          = 0b000,
    LowestPriority = 0b001,
    SMI            = 0b010,
    NMI            = 0b100,
    INIT           = 0b101,
    ExtINT         = 0b111,
}

fn configure_msi(
    dev: &Device,
    msg_addr: u32,
    msg_data: u32,
    num_vector_exponent: u8
) -> StatusCode {
    let mut cap_addr: u8 = (read_conf_reg(dev, 0x34) & 0xff as u32) as u8;
    let mut msi_cap_addr: u8 = 0; let mut msix_cap_addr: u8 = 0;
    while cap_addr != 0 {
        let header = read_capability_header(dev, cap_addr);
        if header.cap_id() == CAPABILITY_MSI {
            msi_cap_addr = cap_addr;
        } else if header.cap_id() == CAPABILITY_MSIX {
            msix_cap_addr = cap_addr;
        }
        cap_addr = header.next_ptr();
    }
    
    if msi_cap_addr != 0 {
        return configure_msi_register(dev, msi_cap_addr, msg_addr, msg_data, num_vector_exponent);
    } else if msix_cap_addr != 0 {
        return configure_msix_register(dev, msix_cap_addr, msg_addr, msg_addr, num_vector_exponent);
    }
    return StatusCode::NoPCIMSI;
}

pub fn configure_msi_fixed_destination(
    dev: &Device,
    apic_id: u8,
    trigger_mode: MSITriggerMode,
    delivery_mode: MSIDeliveryMode,
    vector: u8,
    num_vector_exponent: u8
) -> StatusCode {
    let msg_addr: u32 = 0xFEE00000 as u32 | (apic_id as u32) << 12;
    let mut msg_data: u32 = (delivery_mode as u32) << 8 | vector as u32;
    if trigger_mode == MSITriggerMode::Level {
        msg_data |= 0xc000;
    }
    return StatusCode::Success;
    //return configure_msi(&dev, msg_addr, msg_data, num_vector_exponent);
}