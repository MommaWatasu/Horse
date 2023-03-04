use core::ptr::NonNull;
use crate::{
    status::Result,
    mouse::*,
    Graphics,
    PixelColor,
    graphics::Coord,
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
use spin::Mutex;

pub static MOUSE_CURSOR: Mutex<MouseCursor>
    = Mutex::new(MouseCursor::new(PixelColor(0, 0, 0), Coord::new(100, 100)));

pub struct HidMouseDriver {
    hid_driver: HidDriver,
    screen_size: Coord
}
impl HidMouseDriver {
    pub fn new(interface_idx: u8) -> Result<Self> {
        let graphics = Graphics::instance();
        Ok(Self {
            hid_driver: HidDriver::new(interface_idx, 8)?,
            screen_size: Coord::from_tuple(graphics.resolution())
        })
    }

    pub fn update(&mut self) {
        let _button = self.hid_driver.buffer()[0];
        let dx = self.hid_driver.buffer()[1] as i8;
        let dy = self.hid_driver.buffer()[2] as i8;

        let mut cursor = MOUSE_CURSOR.lock();
        cursor.move_relative((dx, dy), self.screen_size);
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

        self.update();

        Ok(req)
    }
    fn on_endpoints_configured(&mut self) -> Result<TransferRequest> {
        self.hid_driver.on_endpoints_configured()
    }
}
