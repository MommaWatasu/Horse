use core::intrinsics::{
    unaligned_volatile_load,
    unaligned_volatile_store
};

#[repr(transparent)]
pub struct Volatile<T>(T);

impl<T> Volatile<T> {
    pub fn read(&self) -> T {
        unsafe { (self as *const Self as *const T).read_volatile() }
    }

    pub fn write(&mut self, val: T) {
        unsafe { (self as *const Self as *mut T).write_volatile(val) }
    }

    pub fn modify<F: FnOnce(&mut T)>(&mut self, f: F) {
        let mut val = self.read();
        f(&mut val);
        self.write(val);
    }
    
    pub fn unaligned_read(addr: *const Self) -> T {
        unsafe { unaligned_volatile_load(addr as *const T) }
    }
    
    pub fn unaligned_write(addr: *mut Self, val: T) {
        unsafe { unaligned_volatile_store(addr as *mut T, val) }
    }
    
    pub fn unaligned_modify<F: FnOnce(&mut T)>(addr: *mut Self, f: F) {
        let mut val = Self::unaligned_read(addr as *const Self);
        f(&mut val);
        Self::unaligned_write(addr, val);
    }
}
