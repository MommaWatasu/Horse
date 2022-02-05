#[derive(Debug)]
pub enum StatusCode {
    Success,
    Failure,
    Full,
    Empty,
    NoEnoughMemory,
    IndexOutOfRange,
    HostControllerNotHalted,
    InvalidSlotID,
    PortNotConnected,
    InvalidEndpointNumber,
    TransferRingNotSet,
    AlreadyAllocated,
    NotImplemented,
    InvalidDescriptor,
    BufferTooSmall,
    UnknownDevice,
    NoCorrespondingSetupStage,
    TransferFailed,
    InvalidPhase,
    UnknownXHCISpeedID,
    NoWaiter,
    LastOfCode
}

impl StatusCode {
    pub fn to_string(&self) -> &str{
        match self {
            StatusCode::Success => "Success",
            StatusCode::Failure => "Failure",
            StatusCode::Full => "Full",
            StatusCode::Empty => "Empty",
            StatusCode::NoEnoughMemory => "NoEnoughMemory",
            StatusCode::IndexOutOfRange => "IndexOutOfRange",
            StatusCode::HostControllerNotHalted => "HostControllerNotHalted",
            StatusCode::InvalidSlotID => "InvalidSlotID",
            StatusCode::PortNotConnected => "PortNotConnected",
            StatusCode::InvalidEndpointNumber => "InvalidEndpointNumber",
            StatusCode::TransferRingNotSet => "TransferRingNotSet",
            StatusCode::AlreadyAllocated => "AlreadyAllocated",
            StatusCode::NotImplemented => "NotImplemented",
            StatusCode::InvalidDescriptor => "InvalidDescriptor",
            StatusCode::BufferTooSmall => "BufferTooSmall",
            StatusCode::UnknownDevice => "UnknownDevice",
            StatusCode::NoCorrespondingSetupStage => "NoCorrespondingSetupStage",
            StatusCode::TransferFailed => "TransferFailed",
            StatusCode::InvalidPhase => "InvalidPhase",
            StatusCode::UnknownXHCISpeedID => "UnknownXHCISpeedID",
            StatusCode::NoWaiter => "NoWaiter",
            StatusCode::LastOfCode => "LastOfCode"
        }
    }
}

pub type Result<T> = core::result::Result<T, StatusCode>;

impl core::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}", self.to_string()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortConfigPhase {
    NotConnected,
    WaitingAddressed,
    ResettingPort,
    EnablingSlot,
    AddressingDevice,
    InitializingDevice,
    ConfiguringEndpoints,
    Configured,
}