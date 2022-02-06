use core::ptr::NonNull;
use crate::{
    status::Result,
    usb::{
        endpoint::{
            EndpointId,
            EndpointConfig
        },
        setupdata::SetupData
    },
    trace
};
use super::{
    Driver,
    HidDriver,
    TransferRequest
};

pub struct HidMouseDriver {
    hid_driver: HidDriver,
}
impl HidMouseDriver {
    pub fn new(interface_idx: u8) -> Result<Self> {
        Ok(Self {
            hid_driver: HidDriver::new(interface_idx, 8)?,
        })
    }
}
impl Driver for HidMouseDriver {
    fn set_endpoint(&mut self, config: &EndpointConfig) -> Result<()> {
        self.hid_driver.set_endpoint(config)
    }
    fn on_control_completed(
        &mut self,
        ep_id: EndpointId,
        setup_data: SetupData,
        buf_ptr: Option<NonNull<u8>>,
        transfered_size: usize,
    ) -> Result<TransferRequest> {
        self.hid_driver
            .on_control_completed(ep_id, setup_data, buf_ptr, transfered_size)
    }
    fn on_interrupt_completed(
        &mut self,
        ep_id: EndpointId,
        buf_ptr: NonNull<u8>,
        transfered_size: usize,
    ) -> Result<TransferRequest> {
        trace!("HidMouseDriver::on_interrupt_completed ep_id = {:?}", ep_id);
        let req = self
            .hid_driver
            .on_interrupt_completed(ep_id, buf_ptr, transfered_size)?;

        // FIXME
        {
            static MOUSE_CURSOR_POS: spin::Mutex<(isize, isize)> =
                spin::Mutex::new((0, 0));

            let button = self.hid_driver.buffer()[0];
            let dx = self.hid_driver.buffer()[1];
            let dy = self.hid_driver.buffer()[2];

            let dx = if dx >= 128 {
                (dx as isize) - 256
            } else {
                dx as isize
            };
            let dy = if dy >= 128 {
                (dy as isize) - 256
            } else {
                dy as isize
            };

            let mut cursor = MOUSE_CURSOR_POS.lock();
            let (mut x, mut y) = *cursor;

            *cursor = (x, y);
        }

        Ok(req)
    }
    fn on_endpoints_configured(&mut self) -> Result<TransferRequest> {
        self.hid_driver.on_endpoints_configured()
    }
}
