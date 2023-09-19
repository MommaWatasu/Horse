//pub mod cdc;
pub mod hid;
pub mod keyboard;
pub mod mouse;

pub use hid::*;
pub use keyboard::*;
pub use mouse::*;

use crate::{
    drivers::usb::{
        endpoint::{EndpointConfig, EndpointId},
        setupdata::SetupData,
    },
    status::Result,
};
use core::ptr::NonNull;

pub enum TransferRequest {
    NoOp,
    ControlOut(SetupData),
    InterruptIn {
        ep_id: EndpointId,
        buf_ptr: Option<NonNull<u8>>,
        size: usize,
    },
}

pub trait Driver {
    fn set_endpoint(&mut self, config: &EndpointConfig) -> Result<()>;
    fn on_endpoints_configured(&mut self) -> Result<TransferRequest>;
    fn on_control_completed(
        &mut self,
        ep_id: EndpointId,
        setup_data: SetupData,
        buf_ptr: Option<NonNull<u8>>,
        transfered_size: usize,
    ) -> Result<TransferRequest>;
    fn on_interrupt_completed(
        &mut self,
        ep_id: EndpointId,
        buf_ptr: NonNull<u8>,
        transfered_size: usize,
    ) -> Result<TransferRequest>;
}
