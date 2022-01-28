#[derive(Debug)]
pub enum StatusCode {
    KSuccess,
    KFull,
    KEmpty,
    KNoEnoughMemory,
    KIndexOutOfRange,
    KHostControllerNotHalted,
    KInvalidSlotID,
    KPortNotConnected,
    KInvalidEndpointNumber,
    KTransferRingNotSet,
    KAlreadyAllocated,
    KNotImplemented,
    KInvalidDescriptor,
    KBufferTooSmall,
    KUnknownDevice,
    KNoCorrespondingSetupStage,
    KTransferFailed,
    KInvalidPhase,
    KUnknownXHCISpeedID,
    KNoWaiter,
    KLastOfCode
}

impl StatusCode {
    pub fn to_string(&self) -> &str{
        match self {
            StatusCode::KSuccess => "KSuccess",
            StatusCode::KFull => "KFull",
            StatusCode::KEmpty => "KEmpty",
            StatusCode::KNoEnoughMemory => "KNoEnoughMemory",
            StatusCode::KIndexOutOfRange => "KIndexOutOfRange",
            StatusCode::KHostControllerNotHalted => "KHostControllerNotHalted",
            StatusCode::KInvalidSlotID => "KInvalidSlotID",
            StatusCode::KPortNotConnected => "KPortNotConnected",
            StatusCode::KInvalidEndpointNumber => "KInvalidEndpointNumber",
            StatusCode::KTransferRingNotSet => "KTransferRingNotSet",
            StatusCode::KAlreadyAllocated => "KAlreadyAllocated",
            StatusCode::KNotImplemented => "KNotImplemented",
            StatusCode::KInvalidDescriptor => "KInvalidDescriptor",
            StatusCode::KBufferTooSmall => "KBufferTooSmall",
            StatusCode::KUnknownDevice => "KUnknownDevice",
            StatusCode::KNoCorrespondingSetupStage => "KNoCorrespondingSetupStage",
            StatusCode::KTransferFailed => "KTransferFailed",
            StatusCode::KInvalidPhase => "KInvalidPhase",
            StatusCode::KUnknownXHCISpeedID => "KUnknownXHCISpeedID",
            StatusCode::KNoWaiter => "KNoWaiter",
            StatusCode::KLastOfCode => "KLastOfCode"
        }
    }
}

impl core::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}", self.to_string()
        )
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum ConfigPhase {
    KNotConnected,
    KWaitingAddressed,
    KResettingPort,
    KEnablingSlot,
    KAddressingDevice,
    KInitializingDevice,
    KConfiguringEndpoints,
    KConfigured,
}
