use crate::bit_getter;

#[repr(u8)]
pub enum Type {
    Device = 1,
    Configuration = 2,
    Interface = 4,
    Endpoint = 5,
    Hid = 33,
}

pub trait Descriptor {
    const TYPE: u8;
}

pub fn from_bytes<D: Descriptor>(bytes: &[u8]) -> Option<&D> {
    if !bytes.is_empty() && bytes[0] == core::mem::size_of::<D>() as u8 && bytes[1] == D::TYPE {
        let p = bytes.as_ptr() as *const D;
        Some(unsafe { &*p })
    } else {
        None
    }
}

 #[repr(C, packed)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub usb_release: u16,
    pub device_class: u8,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_release: u16,
    pub manifacturer: u8,
    pub product: u8,
    pub serial_number: u8,
    pub num_configurations: u8,
}

impl Descriptor for DeviceDescriptor {
    const TYPE: u8 = Type::Device as u8;
}

pub struct DescIter<'buf> {
    buf: &'buf [u8],
}
impl<'buf> DescIter<'buf> {
    pub fn new(conf_desc: &'buf [u8]) -> Self {
        Self { buf: conf_desc }
    }
    pub fn next<D: Descriptor>(&mut self) -> Option<&'buf D> {
        loop {
            let sz = self.buf[0] as usize;
            if self.buf.len() < sz {
                return None;
            }
            self.buf = &self.buf[sz..];
            if self.buf.is_empty() {
                return None;
            }
            if let Some(desc) = from_bytes(self.buf) {
                return Some(desc);
            }
        }
    }
}

#[repr(C, packed)]
pub struct ConfigurationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration_id: u8,
    pub attributes: u8,
    pub max_power: u8,
}
impl Descriptor for ConfigurationDescriptor {
    const TYPE: u8 = Type::Configuration as u8;
}

#[repr(C, packed)]
pub struct InterfaceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_sub_class: u8,
    pub interface_protocol: u8,
    pub interface_id: u8,
}
impl Descriptor for InterfaceDescriptor {
    const TYPE: u8 = Type::Interface as u8;
}

#[repr(C, packed)]
pub struct EndpointDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}
impl EndpointDescriptor {
    bit_getter!(endpoint_address: u8; 0b00001111; u8, pub number);
    bit_getter!(endpoint_address: u8; 0b10000000; u8, pub dir_in);
    bit_getter!(attributes: u8; 0b00000011; u8, pub transfer_type);
    bit_getter!(attributes: u8; 0b00001100; u8, pub sync_type);
    bit_getter!(attributes: u8; 0b00110000; u8, pub usage_type);
}
impl Descriptor for EndpointDescriptor {
    const TYPE: u8 = Type::Endpoint as u8;
}

#[repr(C, packed)]
pub struct HidDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub hid_release: u16,
    pub country_code: u8,
    pub num_descriptors: u8,
}
impl Descriptor for HidDescriptor {
    const TYPE: u8 = Type::Hid as u8;
}
impl core::fmt::Debug for HidDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let hid_release = self.hid_release;
        write!(f, "HidDescriptor {{ length: {}, descriptor_type: {}, hid_release: {}, country_code: {}, num_descriptors: {} }}", self.length, self.descriptor_type, hid_release, self.country_code, self.num_descriptors)
    }
}