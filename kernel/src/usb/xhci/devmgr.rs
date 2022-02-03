use crate::usb::xhci::context::DeviceContext;
use crate::usb::memory::Allocator;
use crate::status::StatusCode;

const MEM_POOL_SIZE: usize = 4 * 1024 * 1024;
static ALLOC: spin::Mutex<Allocator<MEM_POOL_SIZE>> =
    spin::Mutex::new(Allocator::new());

#[derive(PartialEq)]
pub struct Device {
    context: *const DeviceContext,
}

pub struct DeviceManager<'a> {
    device_context_pointers: &'a[&'a DeviceContext],
    max_slots: usize,
    devices: &'a[&'a Device]
}

impl<'a> DeviceManager<'a> {
    pub fn new(max_slots: usize) -> Result<Self, StatusCode> {
        let device_context_pointers: &'a[&'a DeviceContext];
        let devices: &'a[&'a Device];
        match Allocator::alloc_array::<&Device>(&mut *ALLOC.lock(), max_slots+1) {
            None => {
                return Err(StatusCode::KNoEnoughMemory);
            },
            Some(t) => {
                unsafe {
                    devices = t.as_ref();
                }
            }
        };

        match Allocator::alloc_array::<&DeviceContext>(&mut *ALLOC.lock(), max_slots){
            None => {
                return Err(StatusCode::KNoEnoughMemory)
            },
            Some(t) => {
                unsafe {
                    device_context_pointers = t.as_ref();
                }
            }
        };

        return Ok(DeviceManager{
            device_context_pointers,
            max_slots,
            devices
        });
    }

    pub fn device_contexts(&self) -> &'a[&'a DeviceContext] {
        return self.device_context_pointers;
    }
}
