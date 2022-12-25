use core::ptr::NonNull;
use crate::{
    status::Result,
    usb::{
        buffer::Buffer,
        endpoint::{
            EndpointId,
            EndpointConfig,
            EndpointType
        },
        setupdata::{
            SetupData,
            HidRequest
        },
        setupdata::request_type,
    },
    warn, trace
};
use super::{
    Driver,
    TransferRequest
};

pub struct HidDriver {
    interface_idx: u8,
    in_packet_size: usize,
    ep_interrupt_in: Option<EndpointId>,
    ep_interrupt_out: Option<EndpointId>,
    init_phase: u8,
    buf: Buffer,
    prev_buf: Buffer,
}

impl HidDriver {
    const BUF_SIZE: usize = 1024;

    pub fn new(interface_idx: u8, in_packet_size: usize) -> Result<Self> {
        Ok(Self {
            interface_idx,
            in_packet_size,
            ep_interrupt_in: None,
            ep_interrupt_out: None,
            buf: Buffer::new(Self::BUF_SIZE, 64),
            prev_buf: Buffer::new(Self::BUF_SIZE, 64),
            init_phase: 0,
        })
    }

    fn swap_buffer(&mut self) {
        let buf = &mut self.buf;
        let prev_buf = &mut self.prev_buf;
        core::mem::swap(buf, prev_buf);
    }

    pub fn buffer(&mut self) -> &mut [u8] {
        &mut self.prev_buf[..]
    }
}

impl Driver for HidDriver {
    fn set_endpoint(&mut self, config: &EndpointConfig) -> Result<()> {
        if config.ep_type == EndpointType::Interrupt {
            if config.ep_id.is_in() {
                if self.ep_interrupt_in.is_some() {
                    warn!("ep_interrupt_in overwritten");
                }
                self.ep_interrupt_in = Some(config.ep_id);
            } else {
                if self.ep_interrupt_out.is_some() {
                    warn!("ep_interrupt_out overwritten");
                }
                self.ep_interrupt_out = Some(config.ep_id);
            }
        }
        Ok(())
    }

    fn on_control_completed(
        &mut self,
        _ep_id: EndpointId,
        _setup_data: SetupData,
        buf_ptr: Option<NonNull<u8>>,
        transfered_size: usize,
    ) -> Result<TransferRequest> {
        trace!(
            "HidDriver::on_control_completed: phase = {}, transfered_size = {}",
            self.init_phase,
            transfered_size
        );

        match self.init_phase {
            1 => {
                debug_assert!(buf_ptr.is_none());
                self.init_phase = 2;
                let buf_ptr = self.buf.detach();
                Ok(TransferRequest::InterruptIn {
                    ep_id: self.ep_interrupt_in.expect("Endpoint not initialized"),
                    buf_ptr: Some(buf_ptr),
                    size: self.in_packet_size,
                })
            }
            _ => unimplemented!(),
        }
    }

    fn on_interrupt_completed(
        &mut self,
        ep_id: EndpointId,
        buf_ptr: NonNull<u8>,
        transfered_size: usize,
    ) -> Result<TransferRequest> {
        if ep_id.is_in() {
            debug_assert!(transfered_size <= self.in_packet_size);

            unsafe { self.buf.attach(buf_ptr) };
            self.swap_buffer();
            let buf_ptr = self.buf.detach();

            Ok(TransferRequest::InterruptIn {
                ep_id: self.ep_interrupt_in.expect("Endpoint not initialized"),
                buf_ptr: Some(buf_ptr),
                size: self.in_packet_size,
            })
        } else {
            unreachable!();
        }
    }

    fn on_endpoints_configured(&mut self) -> Result<TransferRequest> {
        let mut setup_data = SetupData::default();
        setup_data.set_direction(request_type::Direction::HostToDevice as u8);
        setup_data.set_typ(request_type::Type::Class as u8);
        setup_data.set_recipient(request_type::Recipient::Interface as u8);
        setup_data.request = HidRequest::SetProtocol as u8;
        setup_data.value = 0; // boot protocol
        setup_data.index = self.interface_idx as u16;
        setup_data.length = 0;

        self.init_phase = 1;
        Ok(TransferRequest::ControlOut(setup_data))
    }
}
