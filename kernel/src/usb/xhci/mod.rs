mod context;
mod devmgr;
mod port;
mod registers;
mod ring;
mod speed;
mod trb;

use core::mem::MaybeUninit;
use core::ptr::{
    null_mut, null
};
use devmgr::DeviceManager;
use ring::*;
use registers::*;
use port::*;
use trb::{
    EvaluateContextCommand,
    EnableSlotCommand,
    AddressDeviceCommand,
    TransferEvent,
    Trb,
    ConfigureEndpointCommand,
    CommandCompletionEvent,
    PortStatusChangeEvent
};
use crate::{
    status_log, debug, warn, trace, error,
    status::{
        StatusCode, PortConfigPhase, Result
    },
    usb::memory::Allocator,
    volatile::Volatile
};

const KDeviceSize: usize = 0;

const MEM_POOL_SIZE: usize = 4 * 1024 * 1024;
pub static ALLOC: spin::Mutex<Allocator<MEM_POOL_SIZE>> =
    spin::Mutex::new(Allocator::new());

pub struct Controller<'a> {
    cap_regs: *mut CapabilityRegisters,
    op_regs: *mut OperationalRegisters,
    devmgr: DeviceManager,
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
    pub unsafe fn new(mmio_base: usize) -> Result<Self> {
        let cap_regs = mmio_base as *mut CapabilityRegisters;
        let max_ports = (*cap_regs).hcs_params1.read().max_ports();
        
        let dboff = (*cap_regs).db_off.read().offset();
        let doorbell_first =
            (mmio_base + dboff) as *mut DoorbellRegister;
        
        let rtsoff = (*cap_regs).rts_off.read().offset();
        let primary_interrupter = (mmio_base + rtsoff + 0x20) as *mut InterrupterRegisterSet;
        
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
            status_log!(StatusCode::Success, "controller is reseted");
            while (*op_regs).usbsts.read().controller_not_ready() != 0 {}
            status_log!(StatusCode::Success, "controller is ready");
        }
        
        let max_slots = (*cap_regs).hcs_params1.read().max_device_slots();
        let slots = core::cmp::min(max_slots, Self::DEVICES_SIZE as u8);
        debug!("up to {} slots", slots);
        (*op_regs)
            .config
            .modify(|config| config.set_max_device_slots_enabled(slots));
        
        let max_scratched_buffer_pages = (*cap_regs).hcs_params2.read().max_scratchpad_buf();
        let scratchpad_buffer_array_ptr = if max_scratched_buffer_pages > 0 {
            debug!(
                "max scratchpad buffer: {} pages",
                max_scratched_buffer_pages
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
                    .ok_or(StatusCode::NoEnoughMemory)?
                    .as_mut()
            };
            
            for ptr in buf_arr.iter_mut() {
                #[allow(unused_unsafe)]
                let buf: &mut [u8] = unsafe {
                    alloc
                        .alloc(page_size, page_size, Some(page_size))
                        .ok_or(StatusCode::NoEnoughMemory)?
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
        
        let cr = CommandRing::with_capacity(32)?;
        //register the address of the Command Ring buffer
        (*op_regs).crcr.modify(|value| {
            value.set_ring_cycle_state(cr.cycle_bit as u8);
            value.set_command_stop(0);
            value.set_command_abort(0);
            value.set_pointer(cr.buffer_ptr() as usize);
        });
        
        let mut er = EventRing::with_capacity(32)?;
        er.initialize(primary_interrupter);
        
        (*primary_interrupter).iman.modify(|iman| {
            iman.set_interrupt_pending(1);
            iman.set_interrupter_enable(1);
        });
        
        (*op_regs).usbcmd.modify(|usbcmd| {
            usbcmd.set_interrupter_enable(1);
        });
        
        let ports = {
            let port_regs_base = ((op_regs as usize) + 0x400) as *mut PortRegisterSet;
            
            #[allow(unused_unsafe)]
            let ports: &mut [MaybeUninit<Port>] = unsafe {
                ALLOC
                    .lock()
                    .alloc_slice::<Port>((max_ports + 1) as usize)
                    .ok_or(StatusCode::NoEnoughMemory)?
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

        Ok(Self {
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

    pub unsafe fn run(&mut self) -> Result<StatusCode> {
        (*self.op_regs).usbcmd.modify(|usbcmd| {
            usbcmd.set_run_stop(1);
        });

        while (*self.op_regs).usbsts.read().host_controller_halted() == 1 {};

        Ok(StatusCode::Success)
    }
    
    fn request_hc_ownership(mmio_base: usize, cap_regs: *mut CapabilityRegisters) {
        type MmExtendedReg = Volatile<ExtendedRegister>;
        
        fn next(current: *mut MmExtendedReg, step: usize) -> *mut MmExtendedReg {               
            if step == 0 {
                null_mut()
            } else {
                current.wrapping_add(step as usize)
            }
        }
        
        let hccp = unsafe { (*cap_regs).hcc_params1.read() };
        let mut ptr = next(
            mmio_base as *mut _,
            hccp.xecp() as usize
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
            Some(reg) => reg as *mut Volatile<Usblegsup>
        };
        
        let mut r = unsafe { (*reg).read() };
        if r.hc_os_owned_semaphore() == 1 {
            debug!("already os owned");
            return;
        }
        r.set_hc_os_owned_semaphore(1);
        unsafe { (*reg).write(r) };
        
        debug!("waiting untile OS owns xHC...");
        loop {
            let r = unsafe {(*reg).read()};
            if r.hc_bios_owned_semaphore() == 0 && r.hc_os_owned_semaphore() == 1 {
                break;
            }
        }
        debug!("OS has owned xHC");
    }

    pub unsafe fn reset_port(&mut self, port_num: u8) -> Result<()> {
        if !self.ports[port_num as usize].is_connected() {
            return Ok(());
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
                    return Err(StatusCode::InvalidPhase);
                }
                port.set_config_phase(PortConfigPhase::ResettingPort);
                port.reset();
            }
        }
        Ok(())
    }

    pub unsafe fn configure_port(&mut self, port: &Port) {
        let mut first = None;
        for port_num in 1..=self.max_ports {
            if !self.ports[port_num as usize].is_connected() {
                continue;
            }
            if first.is_none() {
                first = Some(port_num);
            }
            debug!("Port {}: connected", port_num);
            if self.ports[port_num as usize].config_phase() == PortConfigPhase::NotConnected {
                if let Err(e) = self.reset_port(port_num) {
                    error!("Failed to configure the port {}: {:?}", port_num, e);
                }
            }
        }
    }
    
    fn ring_doorbell(doorbell: *mut DoorbellRegister) {
        trace!("ring the doorbell zero (Command Ring)");
        unsafe { (*doorbell).ring(0) };
    }

    fn enable_slot(&mut self, port_num: u8) -> Result<()> {
        let port = &mut self.ports[port_num as usize];

        let is_enabled = port.is_enabled();
        let reset_completed = port.is_port_reset_changed();
        trace!(
            "enable_slot: is_enabled = {:?}, is_port_reset_changed = {:?}",
            is_enabled,
            reset_completed
        );

        if is_enabled && reset_completed {
            port.clear_port_reset_change();
            port.set_config_phase(PortConfigPhase::EnablingSlot);
            let cmd = EnableSlotCommand::default();
            trace!("EnableSlotCommand pushed");
            self.cr.push(cmd.upcast());
            Self::ring_doorbell(self.doorbell_first);
        }
        Ok(())
    }

    fn address_device(&mut self, port_num: u8, slot_id: u8) -> Result<()> {
        trace!("address_device: port = {}, slot = {}", port_num, slot_id);
        let port = &self.ports[port_num as usize];
        let input_ctx = self.devmgr.add_device(port, slot_id)?;

        self.ports[port_num as usize].set_config_phase(PortConfigPhase::AddressingDevice);
        let mut cmd = AddressDeviceCommand::default();
        cmd.set_input_context_ptr(input_ctx);
        cmd.set_slot_id(slot_id);
        self.cr.push(cmd.upcast());
        Self::ring_doorbell(self.doorbell_first);

        Ok(())
    }

    pub fn process_event(&mut self) -> Result<()> {
        if let Some(trb) = self.er.front() {
            trace!("event found: TRB type = {}", trb.trb_type());

            match trb.trb_type() {
                TransferEvent::TYPE => self.on_transfer_event()?,
                CommandCompletionEvent::TYPE => self.on_command_completion_event()?,
                PortStatusChangeEvent::TYPE => self.on_port_status_change_event()?,
                _ => {}
            }

            self.er.pop();
            trace!("event popped");
        }
        Ok(())
    }

    fn on_transfer_event(&mut self) -> Result<()> {
        let trb = self
            .er
            .front()
            .unwrap()
            .downcast_ref::<TransferEvent>()
            .expect("TRB is not a TransferEvent");

        let slot_id = trb.slot_id();

        if !(trb.completion_code() == 1 /* Success */ ||
             trb.completion_code() == 13/* Short Packet */)
        {
            warn!(
                "on_transfer_event_received: invalid trb = {:?} (completion code: {})",
                trb.upcast(),
                trb.completion_code(),
            );
            let issuer_trb = unsafe { &*trb.trb_pointer() };
            debug!("issuer = {:?}", issuer_trb);
            return Err(StatusCode::TransferFailed {
                slot_id: trb.slot_id(),
            });
        }

        trace!("TransferEvent: slot_id = {}", slot_id);

        let dev = self
            .devmgr
            .find_by_slot_mut(slot_id)
            .ok_or(StatusCode::InvalidSlotId)?;

        dev.on_transfer_event_received(trb)?;

        if let Some(cmd_trb) = dev.command_trb.take() {
            debug!("command TRB found");
            self.cr.push(&cmd_trb);
            Self::ring_doorbell(self.doorbell_first);
        }

        if dev.is_initialized()
            && self.ports[dev.port_num() as usize].config_phase()
                == PortConfigPhase::InitializingDevice
        {
            let input_ctx = dev.configure_endpoints()?;

            self.ports[dev.port_num() as usize]
                .set_config_phase(PortConfigPhase::ConfiguringEndpoints);
            let mut cmd = ConfigureEndpointCommand::default();
            cmd.set_input_context_ptr(input_ctx);
            cmd.set_slot_id(slot_id);
            self.cr.push(cmd.upcast());
            Self::ring_doorbell(self.doorbell_first);
        }

        Ok(())
    }
    fn on_command_completion_event(&mut self) -> Result<()> {
        let trb = self
            .er
            .front()
            .unwrap()
            .downcast_ref::<CommandCompletionEvent>()
            .expect("TRB is not a CommandCompletionEvent");

        let issuer_type = unsafe { (*trb.command_trb_pointer()).trb_type() };
        let slot_id = trb.slot_id();

        if trb.completion_code() != 1 {
            return Err(StatusCode::CommandCompletionFailed {
                slot_id: trb.slot_id(),
            });
        }

        trace!(
            "CommandCompletionEvent: slot_id = {}, issuer trb_type = {}, code = {}",
            slot_id,
            issuer_type,
            trb.completion_code()
        );

        match issuer_type {
            EnableSlotCommand::TYPE => match self.addressing_port {
                Some(port_num)
                    if self.ports[port_num as usize].config_phase()
                        == PortConfigPhase::EnablingSlot =>
                {
                    self.address_device(port_num, slot_id)
                }
                _ => {
                    warn!("addressing_port is None");
                    Err(StatusCode::InvalidPhase)
                }
            },
            AddressDeviceCommand::TYPE => {
                let dev = self
                    .devmgr
                    .find_by_slot(slot_id)
                    .ok_or(StatusCode::InvalidSlotId)?;
                let port_num = dev.port_num();
                if self.addressing_port.unwrap_or(0) != port_num
                    || self.ports[port_num as usize].config_phase()
                        != PortConfigPhase::AddressingDevice
                {
                    if self.addressing_port != Some(port_num) {
                        warn!(
                            "addressing_port = {:?}, but the event is on port = {}",
                            self.addressing_port, port_num
                        );
                    } else {
                        warn!(
                            "ports[{}].config_phase() = {:?}",
                            port_num,
                            self.ports[port_num as usize].config_phase(),
                        );
                    }
                    Err(StatusCode::InvalidPhase)
                } else {
                    self.addressing_port = None;
                    trace!("looking for the next port to address ...");
                    for i in 1..=self.max_ports {
                        if self.ports[i as usize].config_phase()
                            == PortConfigPhase::WaitingAddressed
                        {
                            trace!("the next port is port {}!", i);
                            unsafe {
                                self.reset_port(i)?;
                            }
                            break;
                        }
                    }
                    let dev = self
                        .devmgr
                        .find_by_slot_mut(slot_id)
                        .ok_or(StatusCode::InvalidSlotId)?;
                    let port_num = dev.port_num();
                    self.ports[port_num as usize]
                        .set_config_phase(PortConfigPhase::InitializingDevice);
                    dev.on_command_completion_event_received(issuer_type)?;
                    if let Some(cmd_trb) = dev.command_trb.take() {
                        debug!("command TRB found");
                        self.cr.push(&cmd_trb);
                        Self::ring_doorbell(self.doorbell_first);
                    }
                    Ok(())
                }
            }
            EvaluateContextCommand::TYPE => {
                let dev = self
                    .devmgr
                    .find_by_slot_mut(slot_id)
                    .ok_or(StatusCode::InvalidSlotId)?;
                let port_num = dev.port_num();
                if self.ports[port_num as usize].config_phase()
                    != PortConfigPhase::InitializingDevice
                {
                    Err(StatusCode::InvalidPhase)
                } else {
                    dev.on_command_completion_event_received(issuer_type)?;
                    if let Some(cmd_trb) = dev.command_trb.take() {
                        debug!("command TRB found");
                        self.cr.push(&cmd_trb);
                        Self::ring_doorbell(self.doorbell_first);
                    }
                    Ok(())
                }
            }
            ConfigureEndpointCommand::TYPE => {
                let dev = self
                    .devmgr
                    .find_by_slot_mut(slot_id)
                    .ok_or(StatusCode::InvalidSlotId)?;
                let port_num = dev.port_num();

                if self.ports[port_num as usize].config_phase()
                    != PortConfigPhase::ConfiguringEndpoints
                {
                    Err(StatusCode::InvalidPhase)
                } else {
                    dev.on_endpoints_configured()?;
                    self.ports[port_num as usize].set_config_phase(PortConfigPhase::Configured);
                    Ok(())
                }
            }
            _ => {
                warn!("unexpected Event");
                Err(StatusCode::InvalidPhase)
            }
        }
    }
    fn on_port_status_change_event(&mut self) -> Result<()> {
        let trb = self
            .er
            .front()
            .unwrap()
            .downcast_ref::<PortStatusChangeEvent>()
            .expect("TRB is not a PortStatusChangeEvent");

        let port_id = trb.port_id();
        let port = &mut self.ports[port_id as usize];
        trace!(
            "PortStatusChangeEvent: port_id = {}, phase = {:?}, (bits = {:032b})",
            port_id,
            port.config_phase(),
            port.bits(),
        );
        match port.config_phase() {
            PortConfigPhase::NotConnected => {
                if port.is_connect_status_changed() {
                    port.clear_connect_status_change();
                    unsafe { self.reset_port(port_id) }
                } else {
                    trace!("skipping reset_port: port_id = {}", port_id);
                    Ok(())
                }
            }
            PortConfigPhase::ResettingPort => {
                if port.is_port_reset_changed() {
                    self.enable_slot(port_id)
                } else {
                    trace!("skipping: enable_slot: port_id = {}", port_id);
                    Ok(())
                }
            }
            PortConfigPhase::EnablingSlot => {
                trace!("skipping: port_id = {}", port_id);
                Ok(())
            }
            PortConfigPhase::WaitingAddressed => {
                trace!("waiting addressed: port_id = {}", port_id);
                Ok(())
            }
            phase => {
                warn!(
                    "config_phase = {:?} (should be {:?}, {:?}, {:?}, or {:?})",
                    phase,
                    PortConfigPhase::NotConnected,
                    PortConfigPhase::ResettingPort,
                    PortConfigPhase::EnablingSlot,
                    PortConfigPhase::WaitingAddressed,
                );
                Err(StatusCode::InvalidPhase)
            }
        }
    }
}
