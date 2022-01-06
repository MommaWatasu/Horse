use crate::usb::xhci::registers::PortRegisterSet;
use crate::status::StatusCode;

pub struct Port {
    port_num: u8,
    port_reg_set: *mut PortRegisterSet
}

impl Port {
    pub fn new(port_num: u8, port_reg_set: *mut PortRegisterSet) -> Self {
        return Self {
            port_num,
            port_reg_set
        };
    }

    pub unsafe fn is_connected(&self) -> bool {
        return (*self.port_reg_set).portsc.read().current_connect_status();
    }

    pub fn number(&self) -> u8 {
        return self.port_num;
    }

    pub unsafe fn reset(& self) -> Result<StatusCode, StatusCode> {
        let mut portsc = (*self.port_reg_set).portsc.read();
        portsc.data &= 0x0e00c3e0;
        portsc.data |= 0x00020010;
        (*self.port_reg_set).portsc.write(portsc);

        while (*self.port_reg_set).portsc.read().port_reset() {}

        return Ok(StatusCode::KSuccess);
    }
}
