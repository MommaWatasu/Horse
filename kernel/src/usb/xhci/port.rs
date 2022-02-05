use super::{
    registers::PortRegisterSet,
    speed::PortSpeed
};
use crate::status::{PortConfigPhase};

pub struct Port {
    port_num: u8,
    regs: *mut PortRegisterSet,
    config_phase: PortConfigPhase
}

impl Port {
    pub fn new(port_num: u8, regs: *mut PortRegisterSet) -> Self {
        Self {
            port_num,
            regs,
            config_phase: PortConfigPhase::NotConnected
        }
    }

    pub fn config_phase(&self) -> PortConfigPhase {
        self.config_phase
    }
    pub fn set_config_phase(&mut self, cp: PortConfigPhase) {
        self.config_phase = cp;
    }

    pub fn number(&self) -> u8 {
        return self.port_num;
    }
    
    pub unsafe fn is_connected(&self) -> bool {
        return (*self.regs).portsc.read().current_connect_status() == 1;
    }
    
    pub fn is_enabled(&self) -> bool {
        unsafe { (*self.regs).portsc.read().port_enabled_disabled() == 1 }
    }
    
    pub fn is_connect_status_changed(&self) -> bool {
        unsafe { (*self.regs).portsc.read().connect_status_change() == 1 }
    }
    
    pub fn is_port_reset_changed(&self) -> bool {
        unsafe { (*self.regs).portsc.read().port_reset_change() == 1 }
    }
    
    pub fn speed(&self) -> PortSpeed {
        unsafe { (*self.regs).portsc.read().port_speed() }.into()
    }
    
    pub fn bits(&self) -> u32 {
        unsafe { (*self.regs).portsc.read().data }
    }

    pub unsafe fn reset(& self) {
        unsafe {
            (*self.regs).portsc.modify(|portsc| {
                portsc.data &= 0b_0000_1110_1111_1110_1100_0011_1110_0000;
                portsc.data |= 0b_0000_0000_0000_0010_0000_0000_0001_0000; // Write 1 to PR and CSC
            })
        };
        while unsafe { (*self.regs).portsc.read().port_reset() == 1 } {}
    }
    
    pub fn clear_connect_status_change(&mut self) {
        unsafe {
            (*self.regs).portsc.modify(|portsc| {
                portsc.data &= 0b_0000_1110_1111_1110_1100_0011_1110_0000;
                portsc.set_connect_status_change(1);
            })
        };
    }
    
    pub fn clear_port_reset_change(&mut self)  {
        unsafe {
            (*self.regs).portsc.modify(|portsc| {
                portsc.data &= 0b_0000_1110_1111_1110_1100_0011_1110_0000;
                portsc.set_port_reset_change(1);
            })
        };
    }
}
