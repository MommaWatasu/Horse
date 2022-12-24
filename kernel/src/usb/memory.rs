use alloc::alloc::*;
use core::{
    mem::{align_of, size_of, MaybeUninit},
    ptr::{NonNull, slice_from_raw_parts_mut},
};
use crate::{
    StatusCode,
    trace,
};

pub unsafe fn usb_alloc(mut size: usize, align: usize, boundary: Option<usize>) -> Result<NonNull<[u8]>, StatusCode> {
    let boundary = boundary.unwrap_or(4096);
    if boundary < size.max(align) {
        size = boundary;
    }
    let ptr = alloc(
        Layout::from_size_align(
            size,
            align.min(boundary)
        ).unwrap()
    );
    if ptr.is_null() {
        Err(StatusCode::NoEnoughMemory)
    } else {
        Ok(NonNull::new_unchecked(slice_from_raw_parts_mut(ptr, size)))
    }
}

pub fn usb_slice_alloc<T: 'static>(len: usize) -> Result<NonNull<[T]>, StatusCode> {
    unsafe { usb_slice_ext_alloc(len, align_of::<T>(), None) }
}

pub unsafe fn usb_slice_ext_alloc<T: 'static>(
    len: usize,
    align: usize,
    boundary: Option<usize>
) -> Result<NonNull<[T]>, StatusCode> {
    let buf: &mut [u8] = usb_alloc(size_of::<T>() * len, align, boundary)?.as_mut();
    let ptr = buf.as_mut_ptr() as *mut T;
    Ok(NonNull::new_unchecked(slice_from_raw_parts_mut(ptr, len)))
}

pub fn usb_obj_alloc<T: 'static>()  -> Result<NonNull<T>, StatusCode> {
    unsafe { usb_obj_ext_alloc::<T>(align_of::<T>(), None) }
}

pub unsafe fn usb_obj_ext_alloc<T: 'static>(align: usize, boundary: Option<usize>) -> Result<NonNull<T>, StatusCode> {
    debug_assert!(align % align_of::<T>() == 0);
    let buf: &mut [u8] = usb_alloc(size_of::<T>(), align, boundary)?.as_mut();
    let obj = buf.as_mut_ptr() as *mut T;
    Ok(NonNull::new_unchecked(obj))
}