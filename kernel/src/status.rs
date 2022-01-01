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
            KSuccess => "KSuccess",
            //KFull => "KFull",
            //KEmpty => "KEmpty",
            //KNoEnoughMemory => "KNoEnoughMemory",
            //KIndexOutOfRange => "KIndexOutOfRange",
            //KHostControllerNotHalted => "KHostControllerNotHalted",
            //KInvalidSlotID => "KInvalidSlotID",
            //KPortNotConnected => "KPortNotConnected",
            //KInvalidEndpointNumber => "KInvalidEndpointNumber",
            //KTransferRingNotSet => "KTransferRingNotSet",
            //KAlreadyAllocated => "KAlreadyAllocated",
            //KNotImplemented => "KNotImplemented",
            //KInvalidDescriptor => "KInvalidDescriptor",
            //KBufferTooSmall => "KBufferTooSmall",
            //KUnknownDevice => "KUnknownDevice",
            //KTransferFailed => "KTransferFailed",
            //KInvalidPhase => "KInvalidPhase",
            //KUnknownXHCISpeedID => "KUnknownXHCISpeedID",
            //KNoWaiter => "KNoWaiter",
            //KLastOfCode => "KLastOfCode"
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
