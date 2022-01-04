use crate::usb::xhci::registers::PortRegisterSet;

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
}
