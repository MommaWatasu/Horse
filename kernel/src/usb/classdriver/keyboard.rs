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
    trace, println
};
use super::{
    Driver,
    HidDriver
};

pub struct HidKeyboardDriver {
    hid_driver: HidDriver,
    prev: [u8; 6],
}
impl HidKeyboardDriver {
    pub fn new(interface_idx: u8) -> Result<Self> {
        Ok(Self {
            hid_driver: HidDriver::new(interface_idx, 8)?,
            prev: [0; 6],
        })
    }
    fn key2ascii(shift: bool, keycode: u8) -> Option<char> {
        match (shift, keycode) {
            (false, 0x04) => Some('a'),
            (false, 0x05) => Some('b'),
            (false, 0x06) => Some('c'),
            (false, 0x07) => Some('d'),
            (false, 0x08) => Some('e'),
            (false, 0x09) => Some('f'),
            (false, 0x0a) => Some('g'),
            (false, 0x0b) => Some('h'),
            (false, 0x0c) => Some('i'),
            (false, 0x0d) => Some('j'),
            (false, 0x0e) => Some('k'),
            (false, 0x0f) => Some('l'),
            (false, 0x10) => Some('m'),
            (false, 0x11) => Some('n'),
            (false, 0x12) => Some('o'),
            (false, 0x13) => Some('p'),
            (false, 0x14) => Some('q'),
            (false, 0x15) => Some('r'),
            (false, 0x16) => Some('s'),
            (false, 0x17) => Some('t'),
            (false, 0x18) => Some('u'),
            (false, 0x19) => Some('v'),
            (false, 0x1a) => Some('w'),
            (false, 0x1b) => Some('x'),
            (false, 0x1c) => Some('y'),
            (false, 0x1d) => Some('z'),
            (true, 0x04) => Some('A'),
            (true, 0x05) => Some('B'),
            (true, 0x06) => Some('C'),
            (true, 0x07) => Some('D'),
            (true, 0x08) => Some('E'),
            (true, 0x09) => Some('F'),
            (true, 0x0a) => Some('G'),
            (true, 0x0b) => Some('H'),
            (true, 0x0c) => Some('I'),
            (true, 0x0d) => Some('J'),
            (true, 0x0e) => Some('K'),
            (true, 0x0f) => Some('L'),
            (true, 0x10) => Some('M'),
            (true, 0x11) => Some('N'),
            (true, 0x12) => Some('O'),
            (true, 0x13) => Some('P'),
            (true, 0x14) => Some('Q'),
            (true, 0x15) => Some('R'),
            (true, 0x16) => Some('S'),
            (true, 0x17) => Some('T'),
            (true, 0x18) => Some('U'),
            (true, 0x19) => Some('V'),
            (true, 0x1a) => Some('W'),
            (true, 0x1b) => Some('X'),
            (true, 0x1c) => Some('Y'),
            (true, 0x1d) => Some('Z'),

            (false, 0x1E) => Some('1'),
            (false, 0x1F) => Some('2'),
            (false, 0x20) => Some('3'),
            (false, 0x21) => Some('4'),
            (false, 0x22) => Some('5'),
            (false, 0x23) => Some('6'),
            (false, 0x24) => Some('7'),
            (false, 0x25) => Some('8'),
            (false, 0x26) => Some('9'),
            (false, 0x27) => Some('0'),

            (true, 0x1E) => Some('!'),
            (true, 0x1F) => Some('@'),
            (true, 0x20) => Some('#'),
            (true, 0x21) => Some('$'),
            (true, 0x22) => Some('%'),
            (true, 0x23) => Some('^'),
            (true, 0x24) => Some('&'),
            (true, 0x25) => Some('*'),
            (true, 0x26) => Some('('),
            (true, 0x27) => Some(')'),

            (false, 0x36) => Some(','),
            (false, 0x37) => Some('.'),

            (false, 0x2A) => Some('\x08'), // backspace

            (false, 0x2C) => Some(' '),
            (false, 0x28) => Some('\n'),

            _ => None,
        }
    }
}
impl Driver for HidKeyboardDriver {
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
        trace!(
            "HidKeyboardDriver::on_interrupt_completed ep_id = {:?}",
            ep_id
        );
        let req = self
            .hid_driver
            .on_interrupt_completed(ep_id, buf_ptr, transfered_size)?;

        // FIXME
        {
            const SHIFT_MASK: u8 = 0b00100010;
            let modifier = self.hid_driver.buffer()[0];
            let shift = modifier & SHIFT_MASK != 0;
            for i in 2..8 {
                let key = self.hid_driver.buffer()[i];
                if key == 0 || self.prev.contains(&key) {
                    continue;
                }

                let ch = Self::key2ascii(shift, key);
                println!(
                    "key down: {:?} (mod: {:02x}, key: {:02x})",
                    ch, modifier, key
                );
            }
            for key in self.prev.iter() {
                if !self.hid_driver.buffer()[2..8].contains(&key) {
                    println!("  key up: {:02x}", key);
                }
            }
            self.prev.copy_from_slice(&self.hid_driver.buffer()[2..8]);
        }

        Ok(req)
    }
    fn on_endpoints_configured(&mut self) -> Result<TransferRequest> {
        self.hid_driver.on_endpoints_configured()
    }
}
