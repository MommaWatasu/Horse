mod context;
mod devmgr;
mod registers;
pub mod port;

use registers::*;
use port::*;
use crate::debug;
use crate::status::{StatusCode, ConfigPhase};
use crate::usb::memory::Allocator;

//const MEM_POOL_SIZE: usize = 4 * 1024 * 1024;
//static ALLOC: spin::Mutex<Allocator<MEM_POOL_SIZE>> =
//    spin::Mutex::new(Allocator::new());

static PORT_CONFIG_PHASE: spin::Mutex<[ConfigPhase; 256]> =
    spin::Mutex::new([ConfigPhase::KNotConnected; 256]);

static ADDRESSING_PORT: spin::Mutex<u8> =
    spin::Mutex::new(0);

pub struct Controller<'a> {
    cap_regs: &'a mut CapabilityRegisters,
    op_regs: &'a mut OperationalRegisters,
    doorbell_first: *mut Doorbell,
}

impl<'a> Controller<'a> {
    /// # Safety
    /// mmio_base must be a valid base address for xHCI device MMIO
    pub unsafe fn new(mmio_base: usize) -> Self {
        let cap_regs = &mut *(mmio_base as *mut CapabilityRegisters);
        debug!("cap regs: {}", cap_regs);
        let op_regs =
            &mut *((mmio_base + cap_regs.cap_length.read() as usize) as *mut OperationalRegisters);
        let doorbell_first =
            (mmio_base + (cap_regs.db_off.read() & 0xffff_fffc) as usize) as *mut Doorbell;

        op_regs.usbcmd.modify(|usbcmd| {
            usbcmd.set_intterupt_enable(false);
            usbcmd.set_host_system_error_enable(false);
            usbcmd.set_enable_wrap_event(false);
        });
        if !op_regs.usbsts.read().host_controller_halted() {
            debug!("hc not halted");
            op_regs.usbcmd.modify(|usbcmd| usbcmd.set_run_stop(false));
        }

        while !op_regs.usbsts.read().host_controller_halted() {}
        debug!("hc halted");

        // reset controller
        debug!(
            "hc reset value; {}",
            op_regs.usbcmd.read().host_controller_reset()
        );
        op_regs.usbcmd.modify(|usbcmd| {
            usbcmd.set_host_controller_reset(true);
        });
        while op_regs.usbcmd.read().host_controller_reset() {}
        debug!("controller reset done.");
        while op_regs.usbsts.read().controller_not_ready() {}
        debug!("controller is ready.");
        let max_slots = cap_regs.hcs_params1.read().max_device_slots();
        debug!("max device slots: {}", max_slots);
        op_regs
            .config
            .modify(|config| config.set_max_device_slots_enabled(max_slots));
        //let alloc = ALLOC.lock();

        Controller {
            cap_regs,
            op_regs,
            doorbell_first,
        }
    }

    pub fn run(&mut self) -> Result<StatusCode, StatusCode> {
        let mut usbcmd = self.op_regs.usbcmd.read();
        usbcmd.set_run_stop(true);
        self.op_regs.usbcmd.write(usbcmd);
        self.op_regs.usbcmd.read();

        while self.op_regs.usbsts.read().host_controller_halted() {};

        Ok(StatusCode::KSuccess)
    }

    pub fn reset_port(&self, port: & Port) -> Result<StatusCode, StatusCode> {
        let is_connected: bool;
        unsafe { is_connected = port.is_connected(); };
        debug!("ResetPort: port.is_connected() = {}", is_connected);
        if is_connected {
            return Ok(StatusCode::KSuccess);
        }

        if *ADDRESSING_PORT.lock() != 0 {
            PORT_CONFIG_PHASE.lock()[port.number() as usize] = ConfigPhase::KWaitingAddressed;
        } else {
            let port_phase: ConfigPhase = (*PORT_CONFIG_PHASE.lock())[port.number() as usize];
            if port_phase != ConfigPhase::KNotConnected && port_phase != ConfigPhase::KWaitingAddressed {
                return Err(StatusCode::KInvalidPhase);
            }
            *ADDRESSING_PORT.lock() = port.number();
            (*PORT_CONFIG_PHASE.lock())[port.number() as usize] = ConfigPhase::KResettingPort;
            unsafe {port.reset(); }
        }
        return Ok(StatusCode::KSuccess);
    }

    pub fn configure_port(&self, port: &Port) -> Result<StatusCode, StatusCode> {
        if (*PORT_CONFIG_PHASE.lock())[port.number() as usize] == ConfigPhase::KNotConnected {
            return self.reset_port(port);
        }
        return Ok(StatusCode::KSuccess);
    }

    pub fn port_at(&mut self, port_num: u8) -> Port {
        return Port::new(
            port_num,
            self.port_register_sets().index((port_num-1).into())
        );
    }

    pub fn max_ports(&self) -> u8 {
        return self.cap_regs.hcs_params1.read().max_ports();
    }

    fn port_register_sets(&mut self) -> PortRegisterSets {
        let p_op_regs: *mut OperationalRegisters = &mut *(self.op_regs);
        unsafe {
            return PortRegisterSets::new(
                p_op_regs as usize + 0x400usize,
                self.max_ports() as usize
            );
        };
    }
}
