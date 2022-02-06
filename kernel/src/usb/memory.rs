use core::{
    mem::{align_of, size_of, MaybeUninit},
    ptr::{slice_from_raw_parts_mut, NonNull},
};
use crate::{trace};

#[repr(C, align(4096))]
pub struct Allocator<const BUF_SIZE: usize> {
    buf: MaybeUninit<[u8; BUF_SIZE]>,
    ptr: usize,
    end: usize,
    initialized: bool,
    pub boundary: usize,
}

impl<const BUF_SIZE: usize> Allocator<BUF_SIZE> {
    pub const fn new() -> Self {
        Self {
            buf: MaybeUninit::uninit(),
            ptr: 0,
            end: 0,
            initialized: false,
            boundary: 4096,
        }
    }

    pub fn ensure_initialized(&mut self) {
        if self.initialized {
            return;
        }

        let ptr: *mut [u8; BUF_SIZE] = self.buf.as_mut_ptr();
        unsafe {
            ptr.write_bytes(0, 1) // fill the entire buffer with zeros
        };
        self.ptr = unsafe { (*ptr).as_mut_ptr() as usize };
        self.end = self.ptr + BUF_SIZE;

        self.initialized = true;
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
        self.ensure_initialized();

        let mut ptr = Self::ceil(self.ptr, align);
        let next_boundary = Self::ceil(self.ptr, boundary.unwrap_or(self.boundary));
        if next_boundary < ptr + size {
            ptr = next_boundary;
        }

        if self.end < ptr + size {
            None
        } else {
            trace!("memory allocated: start={:#x}, size={:#x}", ptr, size);
            debug_assert!(!(ptr as *mut u8).is_null());
            self.ptr = ptr + size;

            // NOTE: this is safe because these bytes are guaranteed to be initialized with 0
            //       and the range is within self.buf (this is to say, it is a valid slice)
            Some(unsafe { NonNull::new_unchecked(slice_from_raw_parts_mut(ptr as *mut u8, size)) })
        }
    }
    
    pub fn alloc_slice<T: 'static>(&mut self, len: usize) -> Option<NonNull<[MaybeUninit<T>]>> {
        unsafe { self.alloc_slice_ext::<T>(len, align_of::<T>(), None) }
    }

    pub unsafe fn alloc_slice_ext<T: 'static>(
        &mut self,
        len: usize,
        align: usize,
        boundary: Option<usize>,
    ) -> Option<NonNull<[MaybeUninit<T>]>> {
        let buf: &mut [u8] = self.alloc(size_of::<T>() * len, align, boundary)?.as_mut();
        let ptr = buf.as_mut_ptr() as *mut MaybeUninit<T>;
        Some(NonNull::new_unchecked(slice_from_raw_parts_mut(ptr, len)))
    }
    
    pub fn alloc_obj<T: 'static>(&mut self) -> Option<NonNull<MaybeUninit<T>>> {
        unsafe { self.alloc_obj_ext::<T>(align_of::<T>(), None) }
    }

    /// Safety: `align` must be a multiple of `core::mem::align_of::<T>()`.
    pub unsafe fn alloc_obj_ext<T: 'static>(
        &mut self,
        align: usize,
        boundary: Option<usize>,
    ) -> Option<NonNull<MaybeUninit<T>>> {
        debug_assert!(align % align_of::<T>() == 0);
        let buf: &mut [u8] = self.alloc(size_of::<T>(), align, boundary)?.as_mut();
        let obj = buf.as_mut_ptr() as *mut MaybeUninit<T>;
        Some(NonNull::new_unchecked(obj))
    }
}
