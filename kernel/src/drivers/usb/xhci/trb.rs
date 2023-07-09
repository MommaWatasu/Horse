use core::mem::transmute;
use crate::{bit_getter, bit_setter};
use crate::drivers::usb::{
    endpoint::EndpointId,
    setupdata::SetupData
};
use super::context::*;

#[repr(u8)]
pub enum TypeId {
    Normal = 1,
    
    SetupStage = 2,
    DataStage = 3,
    StatusStage = 4,
    Link = 6,
    
    EnableSlotCommand = 9,
    AddressDeviceCommand = 11,
    ConfigureEndpointCommand = 12,
    EvaluteContextCommand = 13,
    
    TransferEvent = 32,
    CommandCompletionEvent = 33,
    PortStatusChangeEvent = 34
}

pub trait Trb: Sized {
    const TYPE: u8;

    fn upcast(&self) -> &GenericTrb {
        unsafe { core::mem::transmute::<&Self, &GenericTrb>(self) }
    }
}

#[repr(C, align(16))]
#[derive(Clone)]
pub struct GenericTrb {
    pub data: [u32; 4]
}

impl GenericTrb {
    bit_getter!(data[2]: u32; 0xFFFFFFFF; u32, status);
    bit_setter!(data[2]: u32; 0xFFFFFFFF; u32, set_status);
    
    bit_getter!(data[3]: u32; 0x00000001;  u8, pub cycle_bit);
    bit_setter!(data[3]: u32; 0x00000001;  u8, pub set_cycle_bit);
    
    bit_getter!(data[3]: u32; 0x00000002;  u8, evalute_next_trb);
    bit_setter!(data[3]: u32; 0x00000002;  u8, set_evalute_next_trb);
    
    bit_getter!(data[3]: u32; 0x0000FC00;  u8, pub trb_type);
    bit_setter!(data[3]: u32; 0x0000FC00;  u8, set_trb_type);
    
    bit_getter!(data[3]: u32; 0xFFFF0000;  u8, control);
    bit_setter!(data[3]: u32; 0xFFFF0000;  u8, set_control);
    
    pub fn downcast_ref<T: Trb>(&self) -> Option<&T> {
        if self.trb_type() == T::TYPE {
            Some(unsafe { transmute::<&Self, &T>(self) })
        } else {
            None
        }
    }
}

impl core::fmt::Debug for GenericTrb {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let data = &self.data;
        write!(
            f,
            "TRB: {:08x} {:09x} {:08x} {:09x}",
            data[0], data[1], data[2], data[3]
        )
    }
}

#[repr(C, align(16))]
pub struct Normal {
    data: [u32; 4]
}

impl Normal {
    bit_getter!(data[0]: u32; 0xFFFFFFFF; u32, pub data_buffer_lo);
    bit_setter!(data[0]: u32; 0xFFFFFFFF; u32, pub set_data_buffer_lo);
    bit_getter!(data[1]: u32; 0xFFFFFFFF; u32, pub data_buffer_hi);
    bit_setter!(data[1]: u32; 0xFFFFFFFF; u32, pub set_data_buffer_hi);

    bit_getter!(data[2]: u32; 0x0001FFFF; u32, pub trb_transfer_length);
    bit_setter!(data[2]: u32; 0x0001FFFF; u32, pub set_trb_transfer_length);

    bit_getter!(data[2]: u32; 0x003E0000; u32, pub td_size);
    bit_setter!(data[2]: u32; 0x003E0000; u32, pub set_td_size);

    bit_getter!(data[3]: u32; 0x00000004;  u8, pub interrupt_on_short_packet);
    bit_setter!(data[3]: u32; 0x00000004;  u8, pub set_interrupt_on_short_packet);

    bit_getter!(data[3]: u32; 0x00000020;  u8, pub interrupt_on_completion);
    bit_setter!(data[3]: u32; 0x00000020;  u8, pub set_interrupt_on_completion);

    bit_setter!(data[3]: u32; 0x0000FC00;  u8, pub set_trb_type);
    
    pub fn data_buffer(&self) -> *mut u8 {
        (((self.data_buffer_hi() as usize) << 32) | (self.data_buffer_lo() as usize))
            as *mut u8
    }
    pub fn set_data_buffer(&mut self, ptr: *mut u8) {
        let ptr = ptr as usize;
        self.set_data_buffer_hi(((ptr & 0xFFFFFFFF00000000) >> 32) as u32);
        self.set_data_buffer_lo(((ptr & 0x00000000FFFFFFFF) >> 00) as u32);
    }
}

impl Default for Normal {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb
    }
}
impl Trb for Normal {
    const TYPE: u8 = TypeId::Normal as u8;
}

#[repr(C, align(16))]
pub struct SetupStage {
    data: [u32; 4],
}
impl SetupStage {
    bit_getter!(data[0]: u32; 0x000000FF;  u8, pub request_type);
    bit_setter!(data[0]: u32; 0x000000FF;  u8, pub set_request_type);

    bit_getter!(data[0]: u32; 0x0000FF00;  u8, pub request);
    bit_setter!(data[0]: u32; 0x0000FF00;  u8, pub set_request);

    bit_getter!(data[0]: u32; 0xFFFF0000; u16, pub value);
    bit_setter!(data[0]: u32; 0xFFFF0000; u16, pub set_value);

    bit_getter!(data[1]: u32; 0x0000FFFF; u16, pub index);
    bit_setter!(data[1]: u32; 0x0000FFFF; u16, pub set_index);

    bit_getter!(data[1]: u32; 0xFFFF0000; u16, pub length);
    bit_setter!(data[1]: u32; 0xFFFF0000; u16, pub set_length);

    bit_getter!(data[2]: u32; 0x0001FFFF; u32, pub trb_transfer_length);
    bit_setter!(data[2]: u32; 0x0001FFFF; u32, pub set_trb_transfer_length);

    bit_getter!(data[3]: u32; 0x00000040;  u8, pub immediate_data);
    bit_setter!(data[3]: u32; 0x00000040;  u8, pub set_immediate_data);

    bit_setter!(data[3]: u32; 0x0000FC00;  u8, set_trb_type);

    bit_getter!(data[3]: u32; 0x00030000;  u8, pub transfer_type);
    bit_setter!(data[3]: u32; 0x00030000;  u8, pub set_transfer_type);

    fn new(setup_data: SetupData, transfer_type: u8) -> Self {
        let mut setup = Self::default();
        setup.set_request_type(setup_data.request_type);
        setup.set_request(setup_data.request);
        setup.set_value(setup_data.value);
        setup.set_index(setup_data.index);
        setup.set_length(setup_data.length);
        setup.set_transfer_type(transfer_type);
        setup
    }
    pub fn new_no_data_stage(setup_data: SetupData) -> Self {
        Self::new(setup_data, 0)
    }
    pub fn new_out_data_stage(setup_data: SetupData) -> Self {
        Self::new(setup_data, 2)
    }
    pub fn new_in_data_stage(setup_data: SetupData) -> Self {
        Self::new(setup_data, 3)
    }
}
impl Default for SetupStage {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb.set_immediate_data(1);
        trb.set_trb_transfer_length(8);
        trb
    }
}
impl Trb for SetupStage {
    const TYPE: u8 = TypeId::SetupStage as u8;
}

#[repr(C, align(16))]
pub struct DataStage {
    data: [u32; 4],
}
impl DataStage {
    bit_getter!(data[0]: u32; 0xFFFFFFFF; u32, data_buffer_lo);
    bit_setter!(data[0]: u32; 0xFFFFFFFF; u32, set_data_buffer_lo);
    bit_getter!(data[1]: u32; 0xFFFFFFFF; u32, data_buffer_hi);
    bit_setter!(data[1]: u32; 0xFFFFFFFF; u32, set_data_buffer_hi);

    bit_getter!(data[2]: u32; 0x0001FFFF; u32, pub trb_transfer_length);
    bit_setter!(data[2]: u32; 0x0001FFFF; u32, pub set_trb_transfer_length);

    bit_getter!(data[2]: u32; 0x003E0000; u32, pub td_size);
    bit_setter!(data[2]: u32; 0x003E0000; u32, pub set_td_size);

    bit_getter!(data[3]: u32; 0x00000020;  u8, pub interrupt_on_completion);
    bit_setter!(data[3]: u32; 0x00000020;  u8, pub set_interrupt_on_completion);

    bit_setter!(data[3]: u32; 0x0000FC00;  u8, pub set_trb_type);

    bit_getter!(data[3]: u32; 0x00010000;  u8, pub direction);
    bit_setter!(data[3]: u32; 0x00010000;  u8, pub set_direction);

    pub fn data_buffer(&self) -> *mut u8 {
        (((self.data_buffer_hi() as usize) << 32) | (self.data_buffer_lo() as usize))
            as *mut u8
    }
    pub fn set_data_buffer(&mut self, ptr: *mut u8) {
        let ptr = ptr as usize;
        self.set_data_buffer_hi(((ptr & 0xFFFFFFFF00000000) >> 32) as u32);
        self.set_data_buffer_lo(((ptr & 0x00000000FFFFFFFF) >> 00) as u32);
    }

    fn new(buf: *mut u8, len: usize, dir_in: bool) -> Self {
        let mut trb = Self::default();
        trb.set_data_buffer(buf);
        trb.set_trb_transfer_length(len as u32);
        trb.set_td_size(0);
        trb.set_direction(dir_in as u8);
        trb
    }
    pub fn new_out(buf: *const u8, len: usize) -> Self {
        Self::new(buf as *mut u8, len, false)
    }
    pub fn new_in(buf: *mut u8, len: usize) -> Self {
        Self::new(buf, len, true)
    }
}
impl Default for DataStage {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb
    }
}

impl Trb for DataStage {
    const TYPE: u8 = TypeId::DataStage as u8;
}

#[repr(C, align(16))]
pub struct StatusStage {
    data: [u32; 4],
}

impl StatusStage {
    bit_getter!(data[3]: u32; 0x00000020; u8, pub interrupt_on_completion);
    bit_setter!(data[3]: u32; 0x00000020; u8, pub set_interrupt_on_completion);

    bit_setter!(data[3]: u32; 0x0000FC00; u8, pub set_trb_type);

    bit_getter!(data[3]: u32; 0x00010000; u8, pub direction);
    bit_setter!(data[3]: u32; 0x00010000; u8, pub set_direction);
}
impl Default for StatusStage {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb
    }
}
impl Trb for StatusStage {
    const TYPE: u8 = TypeId::StatusStage as u8;
}

#[repr(C, align(16))]
pub struct Link {
    data: [u32; 4],
}
impl Link {
    bit_getter!(data[0]: u32; 0xFFFFFFF0; u32, pub ring_segment_pointer_lo);
    bit_setter!(data[0]: u32; 0xFFFFFFF0; u32, pub set_ring_segment_pointer_lo);
    bit_getter!(data[1]: u32; 0xFFFFFFFF; u32, pub ring_segment_pointer_hi);
    bit_setter!(data[1]: u32; 0xFFFFFFFF; u32, pub set_ring_segment_pointer_hi);

    bit_setter!(data[3]: u32; 0x00000002; u8, pub set_toggle_cycle);
    bit_setter!(data[3]: u32; 0x0000FC00; u8, pub set_trb_type);

    pub fn ring_segment_pointer(&self) -> usize {
        let lo = (self.ring_segment_pointer_lo() as usize) << 4;
        let hi = (self.ring_segment_pointer_hi() as usize) << 32;
        hi | lo
    }
    pub fn set_ring_segment_pointer(&mut self, ptr: usize) {
        debug_assert!(ptr & 0xF == 0);
        let lo = ((ptr & 0x00000000FFFFFFFF) >> 4) as u32;
        let hi = ((ptr & 0xFFFFFFFF00000000) >> 32) as u32;
        self.set_ring_segment_pointer_lo(lo);
        self.set_ring_segment_pointer_hi(hi);
    }

    pub fn new(next_ring_segment_ptr: usize) -> Self {
        let mut trb = Self::default();
        trb.set_ring_segment_pointer(next_ring_segment_ptr);
        trb
    }
}
impl Default for Link {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb
    }
}
impl Trb for Link {
    const TYPE: u8 = TypeId::Link as u8;
}

#[repr(C, align(16))]
pub struct EnableSlotCommand {
    data: [u32; 4],
}
impl EnableSlotCommand {
    bit_setter!(data[3]: u32; 0x0000FC00; u8, pub set_trb_type);
}
impl Default for EnableSlotCommand {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb
    }
}
impl Trb for EnableSlotCommand {
    const TYPE: u8 = TypeId::EnableSlotCommand as u8;
}

#[repr(C, align(16))]
pub struct AddressDeviceCommand {
    data: [u32; 4],
}
impl AddressDeviceCommand {
    bit_getter!(data[0]: u32; 0xFFFFFFF0; u32, pub input_ctx_ptr_lo);
    bit_setter!(data[0]: u32; 0xFFFFFFF0; u32, pub set_input_ctx_ptr_lo);
    bit_getter!(data[1]: u32; 0xFFFFFFFF; u32, pub input_ctx_ptr_hi);
    bit_setter!(data[1]: u32; 0xFFFFFFFF; u32, pub set_input_ctx_ptr_hi);

    bit_setter!(data[3]: u32; 0x0000FC00;  u8, pub set_trb_type);

    bit_getter!(data[3]: u32; 0xFF000000;  u8, pub slot_id);
    bit_setter!(data[3]: u32; 0xFF000000;  u8, pub set_slot_id);

    pub(super) fn input_context_ptr(&self) -> *const InputContext {
        (((self.input_ctx_ptr_hi() as u64) << 32) | ((self.input_ctx_ptr_lo() << 4) as u64))
            as usize as *const InputContext
    }
    pub(super) fn set_input_context_ptr(&mut self, ptr: *const InputContext) {
        let ptr = ptr as usize as u64;
        debug_assert!(ptr & 0xF == 0);
        self.set_input_ctx_ptr_lo(((ptr & 0x00000000FFFFFFFF) >> 4) as u32);
        self.set_input_ctx_ptr_hi(((ptr & 0xFFFFFFFF00000000) >> 32) as u32);
    }
}
impl Default for AddressDeviceCommand {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb
    }
}
impl Trb for AddressDeviceCommand {
    const TYPE: u8 = TypeId::AddressDeviceCommand as u8;
}

#[repr(C, align(16))]
pub struct ConfigureEndpointCommand {
    data: [u32; 4],
}
impl ConfigureEndpointCommand {
    bit_getter!(data[0]: u32; 0xFFFFFFF0; u32, pub input_ctx_ptr_lo);
    bit_setter!(data[0]: u32; 0xFFFFFFF0; u32, pub set_input_ctx_ptr_lo);
    bit_getter!(data[1]: u32; 0xFFFFFFFF; u32, pub input_ctx_ptr_hi);
    bit_setter!(data[1]: u32; 0xFFFFFFFF; u32, pub set_input_ctx_ptr_hi);

    bit_setter!(data[3]: u32; 0x0000FC00;  u8, pub set_trb_type);

    bit_getter!(data[3]: u32; 0xFF000000;  u8, pub slot_id);
    bit_setter!(data[3]: u32; 0xFF000000;  u8, pub set_slot_id);

    pub(super) fn input_context_ptr(&self) -> *const InputContext {
        (((self.input_ctx_ptr_hi() as u64) << 32) | ((self.input_ctx_ptr_lo() << 4) as u64))
            as usize as *const InputContext
    }
    pub(super) fn set_input_context_ptr(&mut self, ptr: *const InputContext) {
        let ptr = ptr as usize as u64;
        debug_assert!(ptr & 0xF == 0);
        self.set_input_ctx_ptr_lo(((ptr & 0x00000000FFFFFFFF) >> 4) as u32);
        self.set_input_ctx_ptr_hi(((ptr & 0xFFFFFFFF00000000) >> 32) as u32);
    }
}
impl Default for ConfigureEndpointCommand {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb
    }
}
impl Trb for ConfigureEndpointCommand {
    const TYPE: u8 = TypeId::ConfigureEndpointCommand as u8;
}

#[repr(C, align(16))]
pub struct EvaluateContextCommand {
            data: [u32; 4]
}
impl EvaluateContextCommand {
    bit_getter!(data[0]: u32; 0xFFFFFFF0; u32, pub input_ctx_ptr_lo);
    bit_setter!(data[0]: u32; 0xFFFFFFF0; u32, pub set_input_ctx_ptr_lo);
    bit_getter!(data[1]: u32; 0xFFFFFFFF; u32, pub input_ctx_ptr_hi);
    bit_setter!(data[1]: u32; 0xFFFFFFFF; u32, pub set_input_ctx_ptr_hi);

    bit_setter!(data[3]: u32; 0x0000FC00;  u8, pub set_trb_type);

    bit_getter!(data[3]: u32; 0xFF000000;  u8, pub slot_id);
    bit_setter!(data[3]: u32; 0xFF000000;  u8, pub set_slot_id);

    pub(super) fn input_context_ptr(&self) -> *const InputContext {
        (((self.input_ctx_ptr_hi() as u64) << 32) | ((self.input_ctx_ptr_lo() << 4) as u64))
            as usize as *const InputContext
    }
    pub(super) fn set_input_context_ptr(&mut self, ptr: *const InputContext) {
        let ptr = ptr as usize as u64;
        debug_assert!(ptr & 0xF == 0);
        self.set_input_ctx_ptr_lo(((ptr & 0x00000000FFFFFFFF) >> 4) as u32);
        self.set_input_ctx_ptr_hi(((ptr & 0xFFFFFFFF00000000) >> 32) as u32);
    }
}
impl Default for EvaluateContextCommand {
    fn default() -> Self {
        let mut trb = Self { data: [0; 4] };
        trb.set_trb_type(Self::TYPE);
        trb
    }
}
impl Trb for EvaluateContextCommand {
    const TYPE: u8 = TypeId::EvaluteContextCommand as u8;
}

#[repr(C, align(16))]
pub struct TransferEvent {
    data: [u32; 4],
}
impl TransferEvent {
    bit_getter!(data[0]: u32; 0xFFFFFFFF; u32, pub trb_pointer_lo);
    bit_getter!(data[1]: u32; 0xFFFFFFFF; u32, pub trb_pointer_hi);

    bit_getter!(data[2]: u32; 0x00FFFFFF; u32, pub trb_transfer_length);
    bit_getter!(data[2]: u32; 0xFF000000;  u8, pub completion_code);

    bit_getter!(data[3]: u32; 0x001F0000;  u8, endpoint_id_u8);
    bit_getter!(data[3]: u32; 0xFF000000;  u8, pub slot_id);

    pub fn trb_pointer(&self) -> *const GenericTrb {
        (((self.trb_pointer_hi() as usize) << 32) | (self.trb_pointer_lo() as usize))
            as *const GenericTrb
    }

    pub fn endpoint_id(&self) -> EndpointId {
        EndpointId::from_addr(self.endpoint_id_u8())
    }
}

impl Trb for TransferEvent {
    const TYPE: u8 = TypeId::TransferEvent as u8;
}

#[repr(C, align(16))]
pub struct CommandCompletionEvent {
    data: [u32; 4],
}
impl CommandCompletionEvent {
    bit_getter!(data[0]: u32; 0xFFFFFFF0; u32, command_trb_pointer_lo);
    bit_getter!(data[1]: u32; 0xFFFFFFFF; u32, command_trb_pointer_hi);

    bit_getter!(data[2]: u32; 0xFF000000;  u8, pub completion_code);
    bit_getter!(data[3]: u32; 0xFF000000;  u8, pub slot_id);

    pub fn command_trb_pointer(&self) -> *const GenericTrb {
        let lo = self.command_trb_pointer_lo() << 4;
        let hi = self.command_trb_pointer_hi();
        (((hi as usize) << 32) | (lo as usize)) as *const GenericTrb
    }
}
impl Trb for CommandCompletionEvent {
    const TYPE: u8 = TypeId::CommandCompletionEvent as u8;
}

#[repr(C, align(16))]
pub struct PortStatusChangeEvent {
    data: [u32; 4],
}
impl PortStatusChangeEvent {
    bit_getter!(data[0]: u32; 0xFF000000; u8, pub port_id);
}
impl Trb for PortStatusChangeEvent {
    const TYPE: u8 = TypeId::PortStatusChangeEvent as u8;
}
