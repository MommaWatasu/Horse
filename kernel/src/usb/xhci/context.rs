#[repr(C, align(32))]
#[derive(PartialEq)]
pub struct SlotContext {
    data: [u32; 8],
}

#[repr(C, align(32))]
#[derive(PartialEq)]
pub struct EndpointContext {
    data: [u32; 8],
}

#[repr(C, align(64))]
#[derive(PartialEq)]
pub struct DeviceContext {
    slot_context: SlotContext,
    endpoint_contexts: [EndpointContext; 31],
}
