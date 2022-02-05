use crate::status::StatusCode;
use super::descriptor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EndpointType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
}

impl core::convert::TryFrom<u8> for EndpointType {
    fn try_from(ty: u8) -> core::result::Result<Self, StatusCode> {
        match ty {
            0 => Ok(Self::Control),
            1 => Ok(Self::Isochronous),
            2 => Ok(Self::Bulk),
            3 => Ok(Self::Interrupt),
            _ => Err(StatusCode::InvalidEndpointType { ty }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndpointId {
    addr: u8,
}
impl EndpointId {
    const DEFAULT_CONTROL_PIPE: EndpointId = EndpointId { addr: 1 };

    pub fn from_addr(addr: u8) -> Self {
        Self { addr }
    }
    pub fn from_number_in(ep_num: u8) -> Self {
        Self {
            addr: (ep_num << 1) | 1,
        }
    }
    pub fn from_number_out(ep_num: u8) -> Self {
        Self {
            addr: (ep_num << 1) | (ep_num == 0) as u8,
        }
    }

    pub fn address(&self) -> u8 {
        self.addr
    }

    pub fn number(&self) -> u8 {
        self.addr >> 1
    }

    pub fn is_in(&self) -> bool {
        (self.addr & 1) == 1
    }
}

#[derive(Debug, Clone)]
pub struct EndpointConfig {
    pub ep_id: EndpointId,
    pub ep_type: EndpointType,
    pub max_packet_size: u16,
    pub interval: u8,
}
impl From<&descriptor::EndpointDescriptor> for EndpointConfig {
    fn from(ep_desc: &descriptor::EndpointDescriptor) -> Self {
        Self {
            ep_id: EndpointId::from_number_in(ep_desc.number()),
            ep_type: <EndpointType as core::convert::TryFrom<u8>>::try_from(
                ep_desc.transfer_type(),
            )
            .expect("invalid EndpointType"),
            max_packet_size: ep_desc.max_packet_size,
            interval: ep_desc.interval,
        }
    }
}