use super::{
    context::DeviceContext,
    memory::Allocator
};
use crate::status::StatusCode;

const MEM_POOL_SIZE: usize = 4 * 1024 * 1024;
static ALLOC: spin::Mutex<Allocator<MEM_POOL_SIZE>> =
    spin::Mutex::new(Allocator::new());

#[derive(PartialEq)]
pub struct Device {
    context: *const DeviceContext,
}

pub struct DeviceManager<'a> {
    device_context_pointers_: &'a[&'a DeviceContext],
    max_slots_: usize,
    devices_: &'a[&'a Device]
}

impl<'a> DeviceManager<'a> {
    pub fn initialize(&mut self, max_slots: usize) -> Result<StatusCode, StatusCode> {
        self.max_slots_ = max_slots;

        match Allocator::alloc_array::<&Device>(&mut *ALLOC.lock(), max_slots+1) {
            None => {
                return Err(StatusCode::KNoEnoughMemory);
            },
            Some(t) => {
                self.devices_ = unsafe { t.as_ref() };
            }
        };

        match Allocator::alloc_array::<&DeviceContext>(&mut *ALLOC.lock(), max_slots){
            None => {
                return Err(StatusCode::KNoEnoughMemory)
            },
            Some(t) => {
                self.device_context_pointers_ = unsafe { t.as_ref() };
            }
        };

        Ok(StatusCode::KSuccess)
    }

    pub fn device_contexts(&self) -> &'a[&'a DeviceContext] {
        return self.device_context_pointers_;
    }
}
