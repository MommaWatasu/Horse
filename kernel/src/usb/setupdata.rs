use crate::{bit_getter, bit_setter};

pub mod request_type {
    #![allow(dead_code)]

    #[repr(u8)]
    pub enum Recipient {
        Device = 0,
        Interface = 1,
        Endpoint = 2,
        Other = 3,
    }
    #[repr(u8)]
    pub enum Type {
        Standard = 0,
        Class = 1,
        Vendor = 2,
    }
    #[repr(u8)]
    pub enum Direction {
        HostToDevice = 0,
        DeviceToHost = 1,
    }
}

#[repr(u8)]
pub enum Request {
    GetDescriptor = 6,
    SetConfiguration = 9,
}
#[repr(u8)]
pub enum HidRequest {
    SetProtocol = 11,
}

#[derive(Default, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct SetupData {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}
impl SetupData {
    bit_getter!(request_type: u8; 0b00011111; u8, pub recipient);
    bit_setter!(request_type: u8; 0b00011111; u8, pub set_recipient);

    bit_getter!(request_type: u8; 0b01100000; u8, pub typ);
    bit_setter!(request_type: u8; 0b01100000; u8, pub set_typ);

    bit_getter!(request_type: u8; 0b10000000; u8, pub direction);
    bit_setter!(request_type: u8; 0b10000000; u8, pub set_direction);
}
