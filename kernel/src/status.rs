#[derive(Debug)]
pub enum StatusCode {
    Success,
    Failure,
    Full,
    Empty,
    NoEnoughMemory,
    IndexOutOfRange,
    HostControllerNotHalted,
    InvalidSlotId,
    PortNotConnected,
    InvalidEndpointNumber,
    TransferRingNotSet,
    DeviceAlreadyAllocated,
    NotImplemented,
    InvalidDescriptor,
    InvalidEndpointType { ty: u8 },
    BufferTooSmall,
    UnknownDevice,
    UnsupportedInterface,
    NoCorrespondingSetupStage,
    TransferFailed { slot_id: u8 },
    CommandCompletionFailed { slot_id: u8 },
    TooManyWaiters,
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
            StatusCode::InvalidSlotId => "InvalidSlotId",
            StatusCode::PortNotConnected => "PortNotConnected",
            StatusCode::InvalidEndpointNumber => "InvalidEndpointNumber",
            StatusCode::TransferRingNotSet => "TransferRingNotSet",
            StatusCode::DeviceAlreadyAllocated => "DeviceAlreadyAllocated",
            StatusCode::NotImplemented => "NotImplemented",
            StatusCode::InvalidDescriptor => "InvalidDescriptor",
            StatusCode::InvalidEndpointType{ ty: _ } => "InvalidEndpointType",
            StatusCode::BufferTooSmall => "BufferTooSmall",
            StatusCode::UnknownDevice => "UnknownDevice",
            StatusCode::UnsupportedInterface => "UnsupportedInterface",
            StatusCode::NoCorrespondingSetupStage => "NoCorrespondingSetupStage",
            StatusCode::TransferFailed{ slot_id: _ } => "TransferFailed",
            StatusCode::CommandCompletionFailed{ slot_id: _ } => "CommandCompletionFailed",
            StatusCode::TooManyWaiters => "TooManyWaiters",
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