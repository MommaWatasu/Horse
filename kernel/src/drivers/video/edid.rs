use core::slice::from_raw_parts;

pub struct EDID {
    data: &'static [u8]
}

impl EDID {
    pub fn new(base: u32) -> Result<Self, ()> {
        if unsafe { *(base as *const u64) } != 0x00ffffffffffff00u64 {
            return Err(());
        }
        return Ok(Self { data: unsafe { from_raw_parts(base as *const u8, 128) } });
    }

    pub fn getter(&self, index: usize) -> u8 {
        return self.data[index]
    }
}