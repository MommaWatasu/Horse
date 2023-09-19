use super::edid::*;
use crate::{drivers::pci::*, println};

enum BGARegisters {
    VBE_DISPI_INDEX_ID = 0,
    VBE_DISPI_INDEX_XRES = 1,
    VBE_DISPI_INDEX_YRES = 2,
    VBE_DISPI_INDEX_BPP = 3,
    VBE_DISPI_INDEX_ENABLE = 4,
    VBE_DISPI_INDEX_BANK = 5,
    VBE_DISPI_INDEX_VIRT_WIDTH = 6,
    VBE_DISPI_INDEX_VIRT_HEIGHT = 7,
    VBE_DISPI_INDEX_INDEX_X_OFFSET = 8,
    VBE_DISPI_INDEX_INDEX_Y_OFFSET = 9,
}

unsafe fn bga_write_register(mmio_base: u32, index: u32, value: u16) {
    ((mmio_base + 0x500 + (index << 1)) as *mut u16).write(value)
}

unsafe fn bga_read_register(mmio_base: u32, index: u32) -> u16 {
    return ((mmio_base + 0x500 + (index << 1)) as *mut u16).read();
}

pub fn setup_qemu_card(dev: &Device) {
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
        bga_write_register(mmio_base, BGARegisters::VBE_DISPI_INDEX_ENABLE as u32, 0x00);
        bga_write_register(
            mmio_base,
            BGARegisters::VBE_DISPI_INDEX_XRES as u32,
            max_res.0,
        );
        bga_write_register(
            mmio_base,
            BGARegisters::VBE_DISPI_INDEX_YRES as u32,
            max_res.1,
        );
        // enable VBE extensions
        bga_write_register(mmio_base, BGARegisters::VBE_DISPI_INDEX_ENABLE as u32, 0x01);
    }
}
