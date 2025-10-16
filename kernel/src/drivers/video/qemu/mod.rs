use super::edid::*;
use crate::drivers::pci::*;

enum BGARegisters {
    VbeDisplIndexId = 0,
    VbeDisplIndexXres = 1,
    VbeDisplIndexYres = 2,
    VbeDisplIndexBpp = 3,
    VbeDisplIndexEnable = 4,
    VbeDisplIndexBank = 5,
    VbeDisplIndexVirtWidth = 6,
    VbeDisplIndexVirtHeight = 7,
    VbeDisplIndexIndexXOffset = 8,
    VbeDisplIndexIndexYOffset = 9,
}

unsafe fn bga_write_register(mmio_base: u32, index: u32, value: u16) {
    ((mmio_base + 0x500 + (index << 1)) as *mut u16).write(value)
}

unsafe fn bga_read_register(mmio_base: u32, index: u32) -> u16 {
    return ((mmio_base + 0x500 + (index << 1)) as *mut u16).read();
}

pub fn setup_qemu_card(dev: &Device) -> (usize, usize) {
    let mmio_base = read_bar32(&dev, 2).unwrap();
    let edid = EDID::new(mmio_base).expect("invalid edid");
    unsafe {
        // get resolutions
        let resolutions = edid.get_resolutions();
        let mut max_res = (0, 0);
        for res in resolutions {
            if (res.0 as u32) * (res.1 as u32) > (max_res.0 as u32) * (max_res.1 as u32) {
                max_res = res;
            }
        }
        // disable VBE extensions
        bga_write_register(mmio_base, BGARegisters::VbeDisplIndexEnable as u32, 0x00);
        bga_write_register(
            mmio_base,
            BGARegisters::VbeDisplIndexXres as u32,
            max_res.0,
        );
        bga_write_register(
            mmio_base,
            BGARegisters::VbeDisplIndexYres as u32,
            max_res.1,
        );
        // enable VBE extensions
        bga_write_register(mmio_base, BGARegisters::VbeDisplIndexEnable as u32, 0x01);
        return (max_res.0 as usize, max_res.1 as usize);
    }
}
