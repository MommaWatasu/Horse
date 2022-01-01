use core::fmt::Display;
use x86_64::instructions::port::{PortReadOnly, PortWriteOnly};
use crate::status::StatusCode;

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
            Err(StatusCode::KFull)
        } else {
            self.devices[self.count] = device;
            self.count += 1;
            Ok(StatusCode::KSuccess)
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
        Ok(StatusCode::KSuccess)
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
        Ok(StatusCode::KSuccess)
    }

    fn scan_bus(&mut self, bus: u8) -> Result<StatusCode, StatusCode> {
        for device in 0..32 {
            if read_vendor_id(bus, device, 0) != 0xffff {
                self.scan_device(bus, device)?;
            }
        }
        Ok(StatusCode::KSuccess)
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
    PCI_PORT.lock().read_dev(dev, reg_addr)
}

pub fn read_bar(device: &Device, bar_index: usize) -> Result<u64, StatusCode> {
    if bar_index >= 6 {
        return Err(StatusCode::KIndexOutOfRange);
    }
    let addr: u8 = calc_bar_address(bar_index);
    let bar: u32 = read_conf_reg(device, addr);

    if (bar & 4) == 0 {
        return Ok(bar.into());
    }

    if bar_index >= 5 {
        return Err(StatusCode::KIndexOutOfRange);
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