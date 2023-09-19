use crate::println;
use core::slice::from_raw_parts;

pub struct EDID {
    data: &'static [u8],
}

impl EDID {
    pub fn new(base: u32) -> Result<Self, ()> {
        if unsafe { *(base as *const u64) } != 0x00ffffffffffff00u64 {
            return Err(());
        }
        return Ok(Self {
            data: unsafe { from_raw_parts(base as *const u8, 128) },
        });
    }

    pub fn getter(&self, index: usize) -> u8 {
        return self.data[index];
    }

    pub fn get_resolutions(&self) -> [(u16, u16); 4] {
        let mut resolutions: [(u16, u16); 4] = [(0, 0); 4];
        for i in 0..4 {
            let base_addr = 0x36 + i * 18;
            let lower_hor: u16 = self.getter(base_addr + 0x02) as u16;
            let upper_hor: u16 = (self.getter(base_addr + 0x04) >> 4) as u16;
            let hor_res = lower_hor | (upper_hor << 8);
            let lower_ver: u16 = self.getter(base_addr + 0x08) as u16;
            let upper_ver: u16 = (self.getter(base_addr + 0x0a) >> 4) as u16;
            let ver_res = lower_ver | (upper_ver << 8);
            resolutions[i] = (hor_res, ver_res);
        }
        return resolutions;
    }
}
