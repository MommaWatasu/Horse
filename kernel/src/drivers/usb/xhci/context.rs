use core::mem::size_of;
use crate::{bit_setter, bit_getter};
use crate::drivers::usb::{
    endpoint::{
        EndpointId,
    }
};

#[repr(C, align(32))]
#[derive(PartialEq)]
pub struct SlotContext {
    data: [u32; 8],
}

impl SlotContext {
    bit_getter!(data[0]: u32; 0x000FFFFF; u32, pub route_string);
    bit_setter!(data[0]: u32; 0x000FFFFF; u32, pub set_route_string);

    bit_getter!(data[0]: u32; 0x00F00000;  u8, pub speed);
    bit_setter!(data[0]: u32; 0x00F00000;  u8, pub set_speed);

    bit_getter!(data[0]: u32; 0xF8000000;  u8, pub context_entries);
    bit_setter!(data[0]: u32; 0xF8000000;  u8, pub set_context_entries);

    bit_getter!(data[1]: u32; 0x00FF0000;  u8, pub root_hub_port_number);
    bit_setter!(data[1]: u32; 0x00FF0000;  u8, pub set_root_hub_port_number);
}

#[repr(C, align(32))]
#[derive(PartialEq)]
pub struct EndpointContext {
    data: [u32; 8],
}

impl EndpointContext {
        bit_getter!(data[0]: u32; 0x00000300;  u8, pub mult);
        bit_setter!(data[0]: u32; 0x00000300;  u8, pub set_mult);

        bit_getter!(data[0]: u32; 0x00007C00;  u8, pub max_primary_streams);
        bit_setter!(data[0]: u32; 0x00007C00;  u8, pub set_max_primary_streams);

        bit_getter!(data[0]: u32; 0x00FF0000;  u8, pub interval);
        bit_setter!(data[0]: u32; 0x00FF0000;  u8, pub set_interval);

        bit_getter!(data[1]: u32; 0x00000006;  u8, pub error_count);
        bit_setter!(data[1]: u32; 0x00000006;  u8, pub set_error_count);

        bit_getter!(data[1]: u32; 0x00000038;  u8, pub endpoint_type);
        bit_setter!(data[1]: u32; 0x00000038;  u8, pub set_endpoint_type);

        bit_getter!(data[1]: u32; 0x0000FF00;  u8, pub max_burst_size);
        bit_setter!(data[1]: u32; 0x0000FF00;  u8, pub set_max_burst_size);

        bit_getter!(data[1]: u32; 0xFFFF0000; u16, pub max_packet_size);
        bit_setter!(data[1]: u32; 0xFFFF0000; u16, pub set_max_packet_size);

        bit_getter!(data[2]: u32; 0x00000001;  u8, pub dequeue_cycle_state);
        bit_setter!(data[2]: u32; 0x00000001;  u8, pub set_dequeue_cycle_state);

        bit_getter!(data[2]: u32; 0xFFFFFFF0; u32, dequeue_pointer_lo);
        bit_setter!(data[2]: u32; 0xFFFFFFF0; u32, set_dequeue_pointer_lo);
        bit_getter!(data[3]: u32; 0xFFFFFFFF; u32, dequeue_pointer_hi);
        bit_setter!(data[3]: u32; 0xFFFFFFFF; u32, set_dequeue_pointer_hi);

        bit_getter!(data[4]: u32; 0x0000FFFF; u16, pub average_trb_length);
        bit_setter!(data[4]: u32; 0x0000FFFF; u16, pub set_average_trb_length);

        pub fn set_transfer_ring_buffer(&mut self, ptr: usize) {
            self.set_dequeue_pointer_lo((((ptr as u64) & 0x00000000FFFFFFFF) >> 4) as u32);
            self.set_dequeue_pointer_hi((((ptr as u64) & 0xFFFFFFFF00000000) >> 32) as u32);
        }
    }

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct DeviceContextIndex(pub usize);
impl From<EndpointId> for DeviceContextIndex {
    fn from(ep_id: EndpointId) -> Self {
        Self(ep_id.address() as usize)
    }
}

#[repr(C, align(64))]
#[derive(PartialEq)]
pub struct DeviceContext {
    pub slot_context: SlotContext,
    ep_ctxs: [EndpointContext; 31],
}

impl DeviceContext {
    pub unsafe fn initialize_ptr(ptr: *mut Self) {
        let ptr = ptr as *mut u8;
        ptr.write_bytes(0, size_of::<Self>());
    }
}

#[repr(C, align(32))]
pub struct InputControlContext {
    drop_context_flags: u32,
    add_context_flags: u32,
    _reserved1: [u32; 5],
    configuration_value: u8,
    interface_number: u8,
    alternate_setting: u8,
    _reserved2: u8,
}

#[repr(C, align(64))]
pub struct InputContext {
    pub input_control_ctx: InputControlContext,
    pub slot_ctx: SlotContext,
    ep_ctxs: [EndpointContext; 31],
}
impl InputContext {
    pub unsafe fn initialize_ptr(ptr: *mut Self) {
        let ptr = ptr as *mut u8;
        ptr.write_bytes(0, size_of::<Self>());
    }
    pub fn enable_slot_context(&mut self) -> &mut SlotContext {
        self.input_control_ctx.add_context_flags |= 1;
        &mut self.slot_ctx
    }
    pub fn update_endpoint(&mut self, dci: DeviceContextIndex) -> &mut EndpointContext {
        self.input_control_ctx.add_context_flags |= 1 << dci.0;
        &mut self.ep_ctxs[dci.0 - 1]
    }
}
