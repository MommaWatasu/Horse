use crate::trace;
use alloc::{vec, vec::Vec};
use core::{
    mem::{align_of, size_of},
    ptr::{slice_from_raw_parts_mut, NonNull},
};
use spin::{
    Once,
    Mutex,
};

const MEM_POOL_SIZE: usize = 4 * 1024 * 1024;
pub static USB_ALLOC: Mutex<Once<USBAlloc<MEM_POOL_SIZE>>> = Mutex::new(Once::new());
pub fn initialize_usballoc() {
    USB_ALLOC.lock().call_once(|| USBAlloc::new());
}

#[repr(C)]
pub struct USBAlloc<const BUF_SIZE: usize> {
    buf: Vec<u8>,
    ptr: usize,
    end: usize,
    boundary: usize,
}

impl<const BUF_SIZE: usize> USBAlloc<BUF_SIZE> {
    // NOTE: This allocator must be initialized after initialize global allocator
    pub fn new() -> Self {
        let mut buf: Vec<u8> = vec![0; BUF_SIZE];
        let ptr = buf.as_mut_ptr() as usize;
        let end = ptr + BUF_SIZE;
        Self {
            buf,
            ptr,
            end,
            boundary: 4096,
        }
    }

    // roundup to alignment; only effective when val is power of two
    fn ceil(addr: usize, alignment: usize) -> usize {
        (addr + alignment - 1) & !(alignment - 1)
    }

    pub fn alloc(
        &mut self,
        size: usize,
        align: usize,
        boundary: Option<usize>,
    ) -> Option<NonNull<[u8]>> {
        let mut ptr = Self::ceil(self.ptr, align);
        let next_boundary = Self::ceil(self.ptr, boundary.unwrap_or(self.boundary));
        if next_boundary < ptr + size {
            ptr = next_boundary;
        }

        if self.end < ptr + size {
            None
        } else {
            trace!("memory allocated(usb): start={:#x}, size={:#x}", ptr, size);
            debug_assert!(!(ptr as *mut u8).is_null());
            self.ptr = ptr + size;

            // NOTE: this is safe because these butes are guaranteed to be initialized with 0
            //       and the range is within self.buf (this is to say, it is a vaild slice)
            Some(unsafe { NonNull::new_unchecked(slice_from_raw_parts_mut(ptr as *mut u8, size)) })
        }
    }

    pub fn alloc_slice<T: 'static>(&mut self, len: usize) -> Option<NonNull<[T]>> {
        unsafe { self.alloc_slice_ext::<T>(len, align_of::<T>(), None) }
    }

    pub unsafe fn alloc_slice_ext<T: 'static>(
        &mut self,
        len: usize,
        align: usize,
        boundary: Option<usize>,
    ) -> Option<NonNull<[T]>> {
        let buf: &mut [u8] = self.alloc(size_of::<T>() * len, align, boundary)?.as_mut();
        let ptr = buf.as_mut_ptr() as *mut T;
        Some(NonNull::new_unchecked(slice_from_raw_parts_mut(ptr, len)))
    }

    pub fn alloc_obj<T: 'static>(&mut self) -> Option<NonNull<T>> {
        unsafe { self.alloc_obj_ext::<T>(align_of::<T>(), None) }
    }

    pub unsafe fn alloc_obj_ext<T: 'static>(
        &mut self,
        align: usize,
        boundary: Option<usize>,
    ) -> Option<NonNull<T>> {
        debug_assert!(align % align_of::<T>() == 0);
        let buf: &mut [u8] = self.alloc(size_of::<T>(), align, boundary)?.as_mut();
        let obj = buf.as_mut_ptr() as *mut T;
        Some(NonNull::new_unchecked(obj))
    }
}
