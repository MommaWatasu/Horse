use crate::warn;

#[derive(Debug)]
#[repr(u8)]
pub enum PortSpeed {
    Full = 1,
    Low = 2,
    High = 3,
    Super = 4,
    SuperSpeedPlus = 5,
}
impl PortSpeed {
    pub fn determine_max_packet_size_for_control_pipe(&self) -> u16 {
        match self {
            Self::SuperSpeedPlus | Self::Super => 512,
            Self::High => 64,
            Self::Full => {
                warn!("Max Packet Size of FullSpeed devices is one of 8, 16, 32, or 64");
                8
            }
            Self::Low => 8,
        }
    }
}
impl From<u8> for PortSpeed {
    fn from(speed: u8) -> Self {
        match speed {
            1 => Self::Full,
            2 => Self::Low,
            3 => Self::High,
            4 => Self::Super,
            5 => Self::SuperSpeedPlus,
            _ => panic!("unknown speed: {}", speed),
        }
    }
}
