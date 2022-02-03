mod context;
mod devmgr;
pub mod registers;
pub mod port;

use core::mem::MaybeUninit;
use core::ptr::{
    null_mut, null
};
use devmgr::DeviceManager;
use registers::*;
use port::*;
use crate::{status_log, debug, warn};
use crate::status::{StatusCode, ConfigPhase};
use crate::usb::memory::Allocator;
use crate::volatile::Volatile;

const KDeviceSize: usize = 0;

const MEM_POOL_SIZE: usize = 4 * 1024 * 1024;
static ALLOC: spin::Mutex<Allocator<MEM_POOL_SIZE>> =
    spin::Mutex::new(Allocator::new());

static PORT_CONFIG_PHASE: spin::Mutex<[ConfigPhase; 256]> =
    spin::Mutex::new([ConfigPhase::KNotConnected; 256]);

pub struct Controller<'a> {
    cap_regs: *mut CapabilityRegisters,
    op_regs: *mut OperationalRegisters,
    devmgr: DeviceManager<'a>,
    cr: CommandRing,
    er: EventRing,
    ports: &'a mut [Port],
    max_ports: u8,
    addressing_port: Option<u8>,
    doorbell_first: *mut DoorbellRegister
}

impl<'a> Controller<'a> {
    const DEVICES_SIZE: usize = 16;
    /// # Safety
    /// mmio_base must be a valid base address for xHCI device MMIO
    pub unsafe fn new(mmio_base: usize) -> Result<Self, StatusCode> {
        let cap_regs = mmio_base as *mut CapabilityRegisters;
        
        let dboff = (*cap_regs).hcs_params1.read().offset();
        let doorbell_first =
            (mmio_base + dboff) as *mut DoorbellRegister;
        
        let rtsoff = (*cap_regs).rts_off.read().offset();
        let primary_interrupter = (mmio_base + rtsoff + 0x20) as *mut IntrrupterRegisterSet;
        
        let caplength = (*cap_regs).cap_length.read();
        let op_regs = (mmio_base + caplength as usize) as *mut OperationalRegisters;

        if (*op_regs).usbsts.read().host_controller_halted() == 0 {
            (*op_regs).usbcmd.modify(|usbcmd| {
                usbcmd.set_run_stop(0);
            });
        }

        // Host controller must be halted
        while (*op_regs).usbsts.read().host_controller_halted() == 0 {}
        debug!("host controller halted");
        
        let page_size = (*op_regs).pagesize.read().page_size();
        ALLOC.lock().boundary = page_size;
        
        Self::request_hc_ownership(mmio_base, cap_regs);
        
        //Reset the controller
        {
            (*op_regs).usbcmd.modify(|usbcmd| {
                usbcmd.set_host_controller_reset(1);
            });
            while (*op_regs).usbcmd.read().host_controller_reset() != 0 {}
            status_log!(StatusCode::KSuccess, "controller is reseted");
            while (*op_regs).usbsts.read().controller_not_ready() != 0 {}
            status_log!(StatusCode::KSuccess, "controller is ready");
        }
        
        let max_slots = (*cap_regs).hcsparams1.read().max_device_slots();
        let slots = core::cmp::min(max_slots, Self::DEVICES_SIZE as u8);
        debug!("up to {} slots", slots);
        op_regs
            .config
            .modify(|config| config.set_max_device_slots_enabled(slots));
        
        let max_scratched_buffer_pages = (*cap_regs).hcsparame2.read().max_scratched_buf();
        let scratchpad_buffer_array_ptr = if max_scratched_buffer_pages > 0 {
            debug!(
                "max scratchpad buffer: {} pages",
                max_scratchpad_buffer_pages
            );
            let mut alloc = ALLOC.lock();
            
            #[allow(unused_unsafe)]
            let buf_arr: &mut [MaybeUninit<*const u8>] = unsafe {
                alloc
                    .alloc_slice_ext::<*const u8>(
                        max_scratched_buffer_pages,
                        64,
                        Some(page_size)
                    )
                    .ok_or(StatusCode::KNoEnoughMemory)?
                    .as_mut()
            };
            
            for ptr in buf.arr.iter_mut() {
                #[allow(unused_unsafe)]
                let buf: &mut [u8] = unsafe {
                    alloc
                        .alloc(page_size, page_size, Some(page_size))
                        .ok_or(StatusCode::KNoEnoughMemory)?
                        .as_mut()
                };
                *ptr = MaybeUninit::new(buf.as_ptr());
            }
            
            #[allow(unused_unsafe)]
            let buf_arr = unsafe {
                core::mem::transmute::<&mut [MaybeUninit<*const u8>], &mut [*const u8]>(buf_arr)
            };
            buf_arr.as_ptr()
        } else {
            null()
        };
        
        let devmgr = DeviceManager::new(
            slots as usize,
            doorbell_first.add(1),
            scratchpad_buffer_array_ptr
        )?;
        
        let mut dcbaap = Dcbaap::default();
        let device_contexts = devmgr.dcbaap();
        dcbaap.set_pointer(device_contexts as usize);
        (*op_regs).dcbaap.write(dcbaap);
        
        let cr = CommandRing::with_capability(32)?;
        //register the address of the Command Ring buffer
        (*op_regs).crcr>modify_with(|value| {
            value.set_ring_cycle_state(cr.cycle_bit as u8);
            value.set_command_ring(0);
            value.set_command_abort(0);
            value.set_pointer(cr.buffer_ptr() as usize);
        });
        
        let mut er = EventRing::with_capability(32)?;
        er.initialize(primary_interrupter)
        
        (*primary_interrupter).iman.modify_with(|iman| {
            iman.set_interrupt_pending(1);
            iman.set_interrupt_enable(1);
        })
        
        (*op_regs).usbcmd.modify_with(|usbcmd| {
            usbcmd.set_interrupter_enable(1);
        });
        
        let ports = {
            let port_regs_base = ((op_regs as usize) + 0x400) as *mut PortRegisterSet;
            
            #[allow(unused_unsafe)]
            let port: &mut [MaybeUninit<Port>] = unsafe {
                ALLOC
                    .lock()
                    .alloc_slice::<Port>((max_ports + 1) as usize)
                    .ok_or(StatusCode::KNoEnoughMemory)?
                    .as_mut()
            };
            
            ports[0] = MaybeUninit::new(Port::new(0, null_mut()));
            for port_num in 1..=max_ports {
                let port_regs = port_regs_base.add((port_num - 1) as usize);
                ports[port_num as usize] = MaybeUninit::new(Port::new(port_num, port_regs));
            }
                
            #[allow(unused_unsafe)]
            unsafe {
                core::mem::transmute::<&mut [MaybeUninit<Port>], &mut [Port]>(ports)
            }            
        };

        Ok(Controller {
            cap_regs,
            op_regs,
            devmgr,
            cr,
            er,
            ports,
            max_ports,
            addressing_port: None,
            doorbell_first
        })
    }

    pub fn run(&mut self) -> Result<StatusCode, StatusCode> {
        unsafe {
            (*self.op_regs).usbcmd.modify(|usbcmd| {
                usbcmd.set_run_stop(1);
            })
        };

        while (*self.op_regs).usbsts.read().host_controller_halted() == 1 {};

        Ok(StatusCode::KSuccess)
    }
    
    fn request_hc_ownership(mmio_base: usize, cap_regs: *mut CapabilityRegisters) {
        type MmExtendedReg = Volatile<ExtendedRegister>;
        
        fn next(current: *mut MmExtendedReg, step: usize) -> *mut MmExtendedReg {               
            if setp == 0 {
                null_mut()
            } else {
                current.unwrapping_add(step as usize)
            }
        }
        
        let hccp = unsafe { (*cap_regs).hcc_params1.read() };
        let mut ptr = next(
            mmio_base as *mut _,
            hccp.xhci_extended_capabilities_pointer() as usize
        );
        let usb_leg_sup = loop {
            if unsafe { (*ptr).read().capability_id() } == Usblegsup::id() {
                break Some(ptr);
            }
            let next_ptr = unsafe { (*ptr).read().next_capability_pointer() };
            ptr = next(ptr, next_ptr as usize);
            if ptr.is_null() {
                break None;
            }
        };
        
        let reg = match usb_leg_sup {
            None => {
                debug!("No USB legacy support");
                return;
            },
            Some(ptr) => reg as *mut Volatile<Usblegsup>
        };
        
        let mut r = unsafe { (*reg).read() };
        if r.hc_os_owned_semaphore() == 1 {
            debug!("already os owned");
            return;
        }
        r.set_hc_owned_semaphore(1);
        unsafe { (*reg).writer(r) };
        
        debug!("waiting untile OS owns xHC...");
        loop {
            let r = unsafe {(*reg).read()};
            if r.hc_bios_owned_semaphore() == 0 && r.hc_os_owned_sema_phore == 1 {
                break;
            }
        }
        debug!("OS has owned xHC");
    }

    pub fn reset_port(&self, port: & Port) -> Result<StatusCode, StatusCode> {
        if !self.ports[port_num as usize].is_connected() {
            return Ok(StatusCode::KSuccess);
        }
        match self.addressing_port {
            Some(_) => {
                self.ports[port_num as usize]
                    .set_config_phase(PortConfigPhase::WaitingAddressed);
            }
            None => {
                self.addressing_port = Some(port_num);
                let port = &mut self.ports[port_num as usize];
                if port.config_phase() != PortConfigPhase::NotConnected
                    && port.config_phase() != PortConfigPhase::WaitingAddressed
                {
                    warn!(
                        "port.config_phase() = {:?} (should be {:?} or {:?})",
                        port.config_phase(),
                        PortConfigPhase::NotConnected,
                        PortConfigPhase::WaitingAddressed
                    );
                    return Err(StatusCode::KInvalidPhase);
                }
                port.set_config_phase(PortConfigPhase::ResettingPort);
                port.reset();
            }
        }
        Ok(())
    }

    pub fn configure_port(&self, port: &Port) -> Result<StatusCode, StatusCode> {
        if (*PORT_CONFIG_PHASE.lock())[port.number() as usize] == ConfigPhase::KNotConnected {
            return self.reset_port(port);
        }
        return Ok(StatusCode::KSuccess);
    }

    pub unsafe fn port_at(&mut self, port_num: u8) -> Port {
        unsafe {
            return Port::new(
                port_num,
                self.port_register_sets().index((port_num-1).into())
            );
        }
    }

    pub fn max_ports(&self) -> u8 {
        return (*self.cap_regs).hcs_params1.read().max_ports();
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
    
    pub fn process_event(&self) {
        
    }
}
