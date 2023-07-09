use core::{
    mem::size_of,
    ptr::{
        null,
        null_mut,
        NonNull,
        addr_of_mut
    }
};
use crate::{
    fixed_vec::FixedVec,
    status::{
        StatusCode,
        Result
    },
    trace, warn, info,
    drivers::usb::{
        array_map::{
                ArrayMap,
                ArrayMapError
        },
        buffer:: Buffer,
        classdriver,
        descriptor,
        descriptor::{
            ConfigurationDescriptor,
            Descriptor,
            DeviceDescriptor,
            EndpointDescriptor,
            HidDescriptor,
            InterfaceDescriptor,
            DescIter
        },
        memory::*,
        setupdata::*,
        setupdata::request_type,
        endpoint::*
    },
};
use super::{
    DoorbellRegister,
    context::*,
    Port,
    speed::PortSpeed,
    TransferRing,
    trb,
    trb::{
        GenericTrb,
        EvaluateContextCommand,
        Trb,
        Normal,
        SetupStage,
        DataStage,
        StatusStage
    }
};

pub struct Device {
    ctx: *const DeviceContext,
    input_ctx: InputContext,
    doorbell: *mut DoorbellRegister,
    transfer_rings: [Option<TransferRing>; 31],
    pub command_trb: Option<GenericTrb>,
    slot_id: u8,
    speed: PortSpeed,

    buf: Buffer,
    init_phase: i8,
    num_configurations: u8,
    config_index: u8,
    ep_configs: FixedVec<EndpointConfig, 16>,
    class_drivers: FixedVec<&'static mut dyn classdriver::Driver, 16>,

    /// EP number --> class driver index
    class_driver_idxs: [Option<usize>; 16],

    /// Setup Data --> class driver index
    event_waiters: ArrayMap<SetupData, usize, 4>,

    /// {DataStage,StatusStage} TRB --> SetupData
    setup_data_map: ArrayMap<*const GenericTrb, SetupData, 16>,
}

impl Device {
    const BUF_SIZE: usize = 1024;

    unsafe fn initialize_ptr(
        ptr: *mut Self,
        ctx: *const DeviceContext,
        doorbell: *mut DoorbellRegister,
        slot_id: u8,
        port: &Port,
    ) -> Result<*const InputContext> {
        {
            let ctx_ptr = addr_of_mut!((*ptr).ctx);
            ctx_ptr.write(ctx);

            let input_ctx_ptr = addr_of_mut!((*ptr).input_ctx);
            InputContext::initialize_ptr(input_ctx_ptr);

            let doorbell_ptr = addr_of_mut!((*ptr).doorbell);
            doorbell_ptr.write(doorbell);

            let transfer_rings_ptr =
                addr_of_mut!((*ptr).transfer_rings) as *mut Option<TransferRing>;
            for i in 0..31 {
                transfer_rings_ptr.add(i).write(None);
            }

            let command_trb_ptr = addr_of_mut!((*ptr).command_trb);
            command_trb_ptr.write(None);

            let slot_id_ptr = addr_of_mut!((*ptr).slot_id);
            slot_id_ptr.write(slot_id);

            let speed_ptr = addr_of_mut!((*ptr).speed);
            speed_ptr.write(port.speed());

            let init_phase_ptr = addr_of_mut!((*ptr).init_phase);
            init_phase_ptr.write(-1);

            let buf_ptr: *mut Buffer = addr_of_mut!((*ptr).buf);
            buf_ptr.write(Buffer::new(Self::BUF_SIZE, 64));

            let num_configurations_ptr = addr_of_mut!((*ptr).num_configurations);
            num_configurations_ptr.write(0);

            let config_index_ptr = addr_of_mut!((*ptr).config_index);
            config_index_ptr.write(0);

            let ep_configs_ptr = addr_of_mut!((*ptr).ep_configs);
            FixedVec::initialize_ptr(ep_configs_ptr);

            let class_drivers_ptr = addr_of_mut!((*ptr).class_drivers);
            FixedVec::initialize_ptr(class_drivers_ptr);

            let class_driver_idxs_ptr =
                addr_of_mut!((*ptr).class_driver_idxs) as *mut Option<usize>;
            for i in 0..16 {
                class_driver_idxs_ptr.add(i).write(None);
            }

            let event_waiters_ptr = addr_of_mut!((*ptr).event_waiters);
            ArrayMap::initialize_ptr(event_waiters_ptr);

            let setup_data_map_ptr = addr_of_mut!((*ptr).setup_data_map);
            ArrayMap::initialize_ptr(setup_data_map_ptr);
        }
        let device = &mut *ptr;
        
        let slot_ctx = device.input_ctx.enable_slot_context();
        slot_ctx.set_route_string(0);
        slot_ctx.set_root_hub_port_number(port.number());
        slot_ctx.set_context_entries(1);
        slot_ctx.set_speed(port.speed() as u8);

        let ep0_dci = EndpointId::DEFAULT_CONTROL_PIPE.into();
        let tr_buf = device.alloc_transfer_ring(ep0_dci, 32)?.buffer_ptr();
        let max_packet_size = port.speed().determine_max_packet_size_for_control_pipe();
        trace!(
            "port.speed() = {:?}, max_packet_size = {}",
            port.speed(),
            max_packet_size
        );

        let ep0_ctx = device.input_ctx.update_endpoint(ep0_dci);
        ep0_ctx.set_endpoint_type(EndpointType::Control as u8);
        ep0_ctx.set_max_packet_size(max_packet_size);
        ep0_ctx.set_max_burst_size(0);
        ep0_ctx.set_transfer_ring_buffer(tr_buf as usize);
        ep0_ctx.set_dequeue_cycle_state(1);
        ep0_ctx.set_interval(0);
        ep0_ctx.set_max_primary_streams(0);
        ep0_ctx.set_mult(0);
        ep0_ctx.set_error_count(3);

        Ok(&device.input_ctx)
    }

    fn alloc_transfer_ring(
        &mut self,
        dci: DeviceContextIndex,
        capacity: usize,
    ) -> Result<&TransferRing> {
        let i = dci.0 - 1;
        self.transfer_rings[i] = Some(TransferRing::with_capacity(capacity)?);
        Ok(self.transfer_rings[i].as_ref().unwrap())
    }

    pub fn port_num(&self) -> u8 {
        unsafe { (*self.ctx).slot_context.root_hub_port_number() }
    }

    pub fn is_initialized(&self) -> bool {
        self.init_phase == 4
    }

    fn initialize_phase0(&mut self, transfered_size: usize) -> Result<()> {
        debug_assert_eq!(transfered_size, 8);
        trace!("initialize_phase0 on slot_id={}", self.slot_id);

        debug_assert_eq!(self.buf[..].len(), Self::BUF_SIZE);
        let device_desc = descriptor::from_bytes::<DeviceDescriptor>(&self.buf[..8]).unwrap();
        trace!("USB release = {:04x}", device_desc.usb_release as u16);
        let ep0_ctx = self
            .input_ctx
            .update_endpoint(EndpointId::DEFAULT_CONTROL_PIPE.into());
        let max_packet_size = if device_desc.usb_release >= 0x0300 {
            (1 << device_desc.max_packet_size) as u16
        } else {
            device_desc.max_packet_size as u16
        };
        trace!(
            "update max_packet_size: {} => {}",
            ep0_ctx.max_packet_size(),
            max_packet_size
        );
        ep0_ctx.set_max_packet_size(max_packet_size);
        let mut eval_ctx_cmd = EvaluateContextCommand::default();
        eval_ctx_cmd.set_input_context_ptr(&self.input_ctx);
        eval_ctx_cmd.set_slot_id(self.slot_id);
        self.command_trb = Some(eval_ctx_cmd.upcast().clone());

        Ok(())
    }

    fn initialize_phase1(&mut self, transfered_size: usize) -> Result<()> {
        trace!("initialize_phase1 on slot_id={}", self.slot_id);
        debug_assert_eq!(transfered_size, 18);
        let device_desc = descriptor::from_bytes::<DeviceDescriptor>(&self.buf[..18]).unwrap();

        self.num_configurations = device_desc.num_configurations;
        trace!("num_configurations = {}", self.num_configurations);

        self.config_index = 0;
        self.init_phase = 2;
        trace!(
            "issuing Get Config Descriptor: index = {}",
            self.config_index
        );

        self.get_descriptor(
            EndpointId::DEFAULT_CONTROL_PIPE,
            ConfigurationDescriptor::TYPE,
            self.config_index,
            Self::BUF_SIZE,
        )
    }

    fn initialize_phase2(&mut self, transfered_size: usize) -> Result<()> {
        trace!("initialize_phase2 on slot_id={}", self.slot_id);

        // TODO: handle the case where total_length > BUF_SIZE
        assert_eq!(
            descriptor::from_bytes::<ConfigurationDescriptor>(&self.buf[..])
                .unwrap()
                .total_length as usize,
            transfered_size
        );

        let mut desc_itr = DescIter::new(&self.buf[..transfered_size]);
        while let Some(if_desc) = desc_itr.next::<InterfaceDescriptor>() {
            let class_driver = match Self::new_class_driver(if_desc) {
                Ok(driver) => driver,
                Err(StatusCode::UnsupportedInterface) => continue,
                Err(e) => return Err(e),
            };

            let class_driver_idx = self.class_drivers.push(class_driver).unwrap();

            let mut num_endpoints = 0;
            trace!("if_desc.num_endpoints = {}", if_desc.num_endpoints);
            while num_endpoints < if_desc.num_endpoints {
                if let Some(ep_desc) = desc_itr.next::<EndpointDescriptor>() {
                    let conf = EndpointConfig::from(ep_desc);
                    trace!("{:?}", conf);
                    let ep_id = conf.ep_id;
                    self.ep_configs.push(conf);
                    num_endpoints += 1;
                    self.class_driver_idxs[ep_id.number() as usize] = Some(class_driver_idx);
                } else if let Some(hid_desc) = desc_itr.next::<HidDescriptor>() {
                    trace!("{:?}", hid_desc);
                }
            }
        }

        if self.class_drivers.len() == 0 {
            warn!("No available class drivers found");
            Ok(())
        } else {
            self.init_phase = 3;

            let conf_desc =
                descriptor::from_bytes::<ConfigurationDescriptor>(&self.buf[..]).unwrap();
            trace!(
                "issuing Set Configuration: conf_val = {}",
                conf_desc.configuration_value
            );
            let conf_val = conf_desc.configuration_value;
            self.set_configuration(EndpointId::DEFAULT_CONTROL_PIPE, conf_val)
        }
    }

    fn initialize_phase3(&mut self) -> Result<()> {
        trace!("initialize_phase3 on slot_id={}", self.slot_id);
        for conf in self.ep_configs.iter() {
            let driver_idx = self.class_driver_idxs[conf.ep_id.number() as usize];
            self.class_drivers
                .get_mut(driver_idx.unwrap())
                .unwrap()
                .set_endpoint(conf)?;
        }
        self.init_phase = 4;
        trace!("slot_id = {}: initialized", self.slot_id);
        Ok(())
    }

    fn new_class_driver(
        if_desc: &InterfaceDescriptor,
    ) -> Result<&'static mut dyn classdriver::Driver> {
        let class = if_desc.interface_class;
        let sub = if_desc.interface_sub_class;
        let proto = if_desc.interface_protocol;
        match (class, sub, proto) {
            (3, 1, 1) => {
                info!("keyboard found");
                use classdriver::HidKeyboardDriver;
                let keyboard_driver = unsafe {
                    let keyboard_driver: *mut HidKeyboardDriver
                        = usballoc().alloc_obj::<HidKeyboardDriver>().unwrap().as_ptr();
                    keyboard_driver.write(HidKeyboardDriver::new(if_desc.interface_number)?);
                    keyboard_driver.as_mut().unwrap()
                };
                Ok(keyboard_driver)
            }
            (3, 1, 2) => {
                info!("mouse found");
                use classdriver::HidMouseDriver;
                let mouse_driver = unsafe {
                    let mouse_driver: *mut HidMouseDriver
                        = usballoc().alloc_obj::<HidMouseDriver>().unwrap().as_ptr();
                    mouse_driver.write(HidMouseDriver::new(if_desc.interface_number)?);
                    mouse_driver.as_mut().unwrap()
                };
                Ok(mouse_driver)
            }
            (c, s, p) => {
                crate::debug!("identifer: ({}, {}, {})", c, s, p);
                Err(StatusCode::UnsupportedInterface)
            },
        }
    }

    pub fn configure_endpoints(&mut self) -> Result<*const InputContext> {
        {
            let input_ctrl_ctx_ptr: *mut u8 =
                &mut self.input_ctx.input_control_ctx as *mut _ as *mut u8;
            unsafe { input_ctrl_ctx_ptr.write_bytes(0, size_of::<InputControlContext>()) };

            let src = unsafe { &(*self.ctx).slot_context as *const SlotContext };
            let dst = &mut self.input_ctx.slot_ctx as *mut SlotContext;
            unsafe { core::ptr::copy(src, dst, 1) };
        }

        let slot_ctx = self.input_ctx.enable_slot_context();
        slot_ctx.set_context_entries(31);

        for i in 0..self.ep_configs.len() {
            let config = self.ep_configs.get(i).unwrap();
            let ep_dci = DeviceContextIndex::from(config.ep_id);
            let tr_buf_ptr = self.alloc_transfer_ring(ep_dci, 32)?.buffer_ptr();

            let config = self.ep_configs.get(i).unwrap();
            let ep_ctx = self.input_ctx.update_endpoint(ep_dci);

            match config.ep_type {
                EndpointType::Control => {
                    ep_ctx.set_endpoint_type(4);
                    trace!("ep_id = {:?}: Control Endpoint", config.ep_id);
                }
                EndpointType::Isochronous => {
                    ep_ctx.set_endpoint_type(if config.ep_id.is_in() { 5 } else { 1 });
                    trace!("ep_id = {:?}: Isoch Endpoint", config.ep_id);
                }
                EndpointType::Bulk => {
                    ep_ctx.set_endpoint_type(if config.ep_id.is_in() { 6 } else { 2 });
                    trace!("ep_id = {:?}: Bulk Endpoint", config.ep_id);
                }
                EndpointType::Interrupt => {
                    ep_ctx.set_endpoint_type(if config.ep_id.is_in() { 7 } else { 3 });
                    trace!("ep_id = {:?}: Interrupt Endpoint", config.ep_id);
                }
            }

            // [0..255] --> [0..7]
            fn interval_map(v: u8) -> u8 {
                if v == 0 {
                    0
                } else {
                    7 - v.leading_zeros() as u8
                }
            }
            let interval = match self.speed {
                PortSpeed::Full | PortSpeed::Low => {
                    if config.ep_type == EndpointType::Isochronous {
                        config.interval + 2
                    } else {
                        interval_map(config.interval) + 3
                    }
                }
                _ => config.interval - 1,
            };
            ep_ctx.set_interval(interval);

            ep_ctx.set_max_packet_size(config.max_packet_size);
            ep_ctx.set_average_trb_length(1);
            ep_ctx.set_transfer_ring_buffer(tr_buf_ptr as usize);
            ep_ctx.set_dequeue_cycle_state(1);
            ep_ctx.set_max_primary_streams(0);
            ep_ctx.set_mult(0);
            ep_ctx.set_error_count(3);
        }

        Ok(&self.input_ctx as *const InputContext)
    }

    pub fn on_endpoints_configured(&mut self) -> Result<()> {
        for idx in 0..self.class_drivers.len() {
            let class_driver = self.class_drivers.get_mut(idx).unwrap();
            match class_driver.on_endpoints_configured()? {
                classdriver::TransferRequest::NoOp => continue,
                classdriver::TransferRequest::ControlOut(setup_data) => {
                    self.control_out(
                        EndpointId::DEFAULT_CONTROL_PIPE,
                        setup_data,
                        Some(idx),
                        None,
                        0,
                    )?;
                }
                _ => unimplemented!(),
            }
        }
        Ok(())
    }

    pub fn on_command_completion_event_received(&mut self, issuer_type: u8) -> Result<()> {
        match issuer_type {
            trb::AddressDeviceCommand::TYPE => {
                self.init_phase = 0;
                self.get_descriptor(
                    EndpointId::DEFAULT_CONTROL_PIPE,
                    DeviceDescriptor::TYPE,
                    0,
                    8, // request first 8-bytes
                )
            }
            trb::EvaluateContextCommand::TYPE => {
                self.init_phase = 1;
                self.get_descriptor(
                    EndpointId::DEFAULT_CONTROL_PIPE,
                    DeviceDescriptor::TYPE,
                    0,
                    18, // entire descriptor
                )
            }
            _ => {
                unreachable!();
            }
        }
    }

    pub fn on_transfer_event_received(&mut self, trb: &trb::TransferEvent) -> Result<()> {
        trace!(
            "device::on_transfer_event_received: issuer TRB = {:p}",
            trb.trb_pointer()
        );

        let issuer_trb = unsafe { &*trb.trb_pointer() };
        let residual_bytes = trb.trb_transfer_length() as usize;

        if let Some(normal) = issuer_trb.downcast_ref::<trb::Normal>() {
            let transfer_length = normal.trb_transfer_length() as usize - residual_bytes;
            return self.on_interrupt_completed(
                trb.endpoint_id(),
                NonNull::new(normal.data_buffer()).expect("data buffer null"),
                transfer_length,
            );
        }

        let setup_data = match self
            .setup_data_map
            .remove(&(issuer_trb as *const trb::GenericTrb))
        {
            None => {
                warn!("No Correspoinding Setup Stage TRB");
                return Err(StatusCode::NoCorrespondingSetupStage);
            }
            Some((_, setup_data)) => setup_data,
        };

        let (buf, transfered_size) =
            if let Some(data_stage) = issuer_trb.downcast_ref::<trb::DataStage>() {
                let buf = NonNull::new(data_stage.data_buffer()).expect("null pointer");
                trace!("residual_bytes = {}", residual_bytes);
                (Some(buf), setup_data.length as usize - residual_bytes)
            } else if let Some(_status_stage) = issuer_trb.downcast_ref::<trb::StatusStage>() {
                (None, 0)
            } else {
                unimplemented!();
            };

        self.on_control_completed(trb.endpoint_id(), setup_data, buf, transfered_size)
    }

    fn on_control_completed(
        &mut self,
        ep_id: EndpointId,
        setup_data: SetupData,
        buf_ptr: Option<NonNull<u8>>,
        transfered_size: usize,
    ) -> Result<()> {
        trace!(
            "device::on_control_completed: transfered_size = {}, dir = {}",
            transfered_size,
            setup_data.direction(),
        );
        match self.init_phase {
            0 => {
                unsafe { self.buf.attach(buf_ptr.unwrap()) };
                let buf = &self.buf[..transfered_size];
                if setup_data.request == Request::GetDescriptor as u8
                    && descriptor::from_bytes::<DeviceDescriptor>(buf).is_some()
                {
                    self.initialize_phase0(transfered_size)
                } else {
                    warn!("failed to go initialize_phase0");
                    Err(StatusCode::InvalidPhase)
                }
            }
            1 => {
                unsafe { self.buf.attach(buf_ptr.unwrap()) };
                let buf = &self.buf[..transfered_size];
                if setup_data.request == Request::GetDescriptor as u8
                    && descriptor::from_bytes::<DeviceDescriptor>(buf).is_some()
                {
                    self.initialize_phase1(transfered_size)
                } else {
                    warn!("failed to go initialize_phase1");
                    Err(StatusCode::InvalidPhase)
                }
            }
            2 => {
                unsafe { self.buf.attach(buf_ptr.unwrap()) };
                let buf = &self.buf[..transfered_size];
                if setup_data.request == Request::GetDescriptor as u8
                    && descriptor::from_bytes::<ConfigurationDescriptor>(buf).is_some()
                {
                    self.initialize_phase2(transfered_size)
                } else {
                    warn!("failed to go initialize_phase2");
                    Err(StatusCode::InvalidPhase)
                }
            }
            3 => {
                if setup_data.request == Request::SetConfiguration as u8 {
                    debug_assert!(buf_ptr.is_none() && self.buf.own());
                    self.initialize_phase3()
                } else {
                    warn!("failed to go initialize_phase3");
                    Err(StatusCode::InvalidPhase)
                }
            }
            _ => match self.event_waiters.get(&setup_data) {
                Some(&w_idx) => {
                    let w = self
                        .class_drivers
                        .get_mut(w_idx)
                        .expect("uninitialized class driver");
                    match w.on_control_completed(ep_id, setup_data, buf_ptr, transfered_size)? {
                        classdriver::TransferRequest::InterruptIn {
                            ep_id,
                            buf_ptr,
                            size,
                        } => self.interrupt_in(ep_id, buf_ptr, size),
                        _ => unimplemented!(),
                    }
                }
                None => Err(StatusCode::NoWaiter),
            },
        }
    }

    fn on_interrupt_completed(
        &mut self,
        ep_id: EndpointId,
        buf_ptr: NonNull<u8>,
        transfered_size: usize,
    ) -> Result<()> {
        trace!(
            "Device::on_interrupt_completed: EP addr = {}",
            ep_id.address()
        );
        if let Some(driver_idx) = self.class_driver_idxs[ep_id.number() as usize] {
            let w = self.class_drivers.get_mut(driver_idx).unwrap();
            match w.on_interrupt_completed(ep_id, buf_ptr, transfered_size)? {
                classdriver::TransferRequest::InterruptIn {
                    ep_id,
                    buf_ptr,
                    size,
                } => {
                    self.interrupt_in(ep_id, buf_ptr, size)?;
                }
                _ => unimplemented!(),
            }
        } else {
            trace!("class driver not found");
        }
        Ok(())
    }

    fn ring_doorbell(&mut self, dci: DeviceContextIndex) {
        trace!("ring the doorbell with target {}", dci.0);
        unsafe { (*self.doorbell).ring(dci.0 as u8) };
    }

    fn control_in(
        &mut self,
        ep_id: EndpointId,
        setup_data: SetupData,
        issuer_idx: Option<usize>,
        buf_ptr: Option<NonNull<u8>>,
        size: usize,
    ) -> Result<()> {
        if let Some(issuer_idx) = issuer_idx {
            self.event_waiters
                .insert(setup_data.clone(), issuer_idx)
                .map_err(|e| match e {
                    ArrayMapError::NoSpace => StatusCode::TooManyWaiters,
                    ArrayMapError::SameKeyRegistered => {
                        panic!("same setup_data registered")
                    }
                })?;
        }

        trace!("Device::control_in: ep_id = {}", ep_id.address());
        if 15 < ep_id.number() {
            return Err(StatusCode::InvalidEndpointNumber);
        }

        let dci = DeviceContextIndex::from(ep_id);
        let tr = self.transfer_rings[dci.0 - 1]
            .as_mut()
            .ok_or(StatusCode::TransferRingNotSet)?;

        if let Some(buf_ptr) = buf_ptr {
            let setup_stage = SetupStage::new_in_data_stage(setup_data.clone());
            tr.push(setup_stage.upcast());

            let mut data_stage = DataStage::new_in(buf_ptr.as_ptr(), size);
            data_stage.set_interrupt_on_completion(1);
            let data_stage_trb_ptr = tr.push(data_stage.upcast()) as *const trb::GenericTrb;

            let mut status_stage = StatusStage::default();
            status_stage.set_direction(0);

            let status_stage_trb_ptr = tr.push(status_stage.upcast());
            trace!("status_stage_trb = {:p}", status_stage_trb_ptr);

            self.setup_data_map
                .insert(data_stage_trb_ptr, setup_data)
                .map_err(|e| match e {
                    ArrayMapError::NoSpace => StatusCode::TooManyWaiters,
                    ArrayMapError::SameKeyRegistered => {
                        panic!("same data_stage_trb_ptr registered")
                    }
                })?;

            self.ring_doorbell(dci);
        } else {
            unimplemented!();
        }

        Ok(())
    }

    fn control_out(
        &mut self,
        ep_id: EndpointId,
        setup_data: SetupData,
        issuer_idx: Option<usize>,
        buf_ptr: Option<NonNull<u8>>,
        size: usize,
    ) -> Result<()> {
        if let Some(issuer_idx) = issuer_idx {
            self.event_waiters
                .insert(setup_data.clone(), issuer_idx)
                .map_err(|e| match e {
                    ArrayMapError::NoSpace => StatusCode::TooManyWaiters,
                    ArrayMapError::SameKeyRegistered => {
                        panic!("same setup_data registered")
                    }
                })?;
        }

        trace!("device::control_out: ep addr = {}", ep_id.address());
        if 15 < ep_id.number() {
            return Err(StatusCode::InvalidEndpointNumber);
        }

        let dci = DeviceContextIndex::from(ep_id);
        let tr = self.transfer_rings[dci.0 - 1]
            .as_mut()
            .ok_or(StatusCode::TransferRingNotSet)?;

        if let Some(_buf_ptr) = buf_ptr {
            let _size = size;
            unimplemented!();
        } else {
            let setup_stage = SetupStage::new_no_data_stage(setup_data.clone());
            tr.push(setup_stage.upcast());

            let mut status_stage = StatusStage::default();
            status_stage.set_direction(1);
            status_stage.set_interrupt_on_completion(1);
            let status_stage_trb_ptr = tr.push(status_stage.upcast());
            trace!("status_stage_trb = {:p}", status_stage_trb_ptr);

            self.setup_data_map
                .insert(status_stage_trb_ptr, setup_data)
                .map_err(|e| match e {
                    ArrayMapError::NoSpace => StatusCode::TooManyWaiters,
                    ArrayMapError::SameKeyRegistered => {
                        panic!("same data_stage_trb_ptr registered")
                    }
                })?;

            self.ring_doorbell(dci);
        }
        Ok(())
    }

    fn interrupt_in(
        &mut self,
        ep_id: EndpointId,
        buf_ptr: Option<NonNull<u8>>,
        size: usize,
    ) -> Result<()> {
        let dci = DeviceContextIndex::from(ep_id);
        let tr = self.transfer_rings[dci.0 - 1]
            .as_mut()
            .ok_or(StatusCode::TransferRingNotSet)?;

        let mut normal = Normal::default();
        normal.set_data_buffer(buf_ptr.map(|ptr| ptr.as_ptr()).unwrap_or(null_mut()));
        normal.set_trb_transfer_length(size as u32);
        normal.set_interrupt_on_short_packet(1);
        normal.set_interrupt_on_completion(1);

        tr.push(normal.upcast());
        self.ring_doorbell(dci);

        Ok(())
    }

    fn get_descriptor(
        &mut self,
        ep_id: EndpointId,
        desc_type: u8,
        desc_index: u8,
        transfer_size: usize,
    ) -> Result<()> {
        let mut setup_data = SetupData::default();
        setup_data.set_direction(request_type::Direction::DeviceToHost as u8);
        setup_data.set_typ(request_type::Type::Standard as u8);
        setup_data.set_recipient(request_type::Recipient::Device as u8);
        setup_data.request = Request::GetDescriptor as u8;
        setup_data.value = ((desc_type as u16) << 8) | (desc_index as u16);
        setup_data.index = 0;

        let buf = self.buf.detach();
        setup_data.length = transfer_size as u16;
        self.control_in(ep_id, setup_data, None, Some(buf), transfer_size)
    }

    fn set_configuration(&mut self, ep_id: EndpointId, config_value: u8) -> Result<()> {
        let mut setup_data = SetupData::default();
        setup_data.set_direction(request_type::Direction::HostToDevice as u8);
        setup_data.set_typ(request_type::Type::Standard as u8);
        setup_data.set_recipient(request_type::Recipient::Device as u8);
        setup_data.request = Request::SetConfiguration as u8;
        setup_data.value = config_value as u16;
        setup_data.index = 0;
        setup_data.length = 0;
        self.control_out(ep_id, setup_data, None, None, 0)
    }
}

pub struct DeviceManager {
    devices: &'static mut [Option<&'static mut Device>],
    device_context_pointers: *mut [*const DeviceContext],
    doorbells: *mut DoorbellRegister, // 1-255
}
impl DeviceManager {
    pub fn new(
        max_slots: usize,
        doorbells: *mut DoorbellRegister,
        scratchpad_buf_arr: *const *const u8,
    ) -> Result<Self> {
        let devices: &mut [Option<&'static mut Device>] = unsafe {
            usballoc().alloc_slice::<Option<&'static mut Device>>(max_slots+1).unwrap().as_mut()
        };
        for p in devices.iter_mut() {
            *p = None;
        }

        // NOTE: DCBAA: alignment = 64-bytes, boundary = PAGESIZE
        let dcbaap: &mut [*const DeviceContext] = unsafe{
            usballoc().alloc_slice_ext::<*const DeviceContext>(max_slots+1, 64, None).unwrap().as_mut()
        };
        for p in dcbaap.iter_mut() {
            *p = null();
        }
        dcbaap[0] = scratchpad_buf_arr as *const _;
        let dcbaap = dcbaap as *mut [*const DeviceContext];

        trace!(
            "DeviceManager has been initialized for up to {} devices",
            max_slots
        );

        Ok(Self {
            devices,
            device_context_pointers: dcbaap,
            doorbells,
        })
    }

    pub fn add_device(&mut self, port: &Port, slot_id: u8) -> Result<*const InputContext> {
        let slot_id = slot_id as usize;
        if !(1 <= slot_id && slot_id < self.devices.len()) {
            return Err(StatusCode::InvalidSlotId);
        }
        if self.devices[slot_id].is_some() {
            return Err(StatusCode::DeviceAlreadyAllocated);
        }

        let device_ctx: &mut DeviceContext = unsafe{
            usballoc().alloc_obj::<DeviceContext>().unwrap().as_mut()
        };
        unsafe { DeviceContext::initialize_ptr(device_ctx as *mut DeviceContext) };

        let device: *mut Device = usballoc().alloc_obj::<Device>().unwrap().as_ptr();

        let input_ctx = unsafe {
            Device::initialize_ptr(
                device,
                device_ctx,
                self.doorbells.add(slot_id - 1),
                slot_id as u8,
                port,
            )?
        };

        self.devices[slot_id] = unsafe { Some(device.as_mut().unwrap()) };

        unsafe {
            (*self.device_context_pointers)
                .as_mut_ptr()
                .add(slot_id)
                .write(device_ctx)
        };
        trace!("add_device: slot_id = {}", slot_id);

        Ok(input_ctx)
    }

    pub fn dcbaap(&self) -> *const *const DeviceContext {
        let ptr = unsafe { (*self.device_context_pointers).as_ptr() };
        debug_assert!((ptr as usize) % 64 == 0);
        ptr
    }

    pub fn find_by_slot(&self, slot_id: u8) -> Option<&Device> {
        self.devices
            .get(slot_id as usize)
            .and_then(|dev| dev.as_deref())
    }

    pub fn find_by_slot_mut(&mut self, slot_id: u8) -> Option<&mut Device> {
        self.devices
            .get_mut(slot_id as usize)
            .and_then(|dev| dev.as_deref_mut())
    }
}
