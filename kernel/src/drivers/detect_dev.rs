use crate::{info, print, println};
use super::{
    pci::{
        switch_echi2xhci,
        PciDevices
    },
    ata::pata::initialize_ide,
    video::qemu::setup_qemu_card,
    usb::xhci::{
        initialize_xhci,
        Controller
    }
};

pub fn initialize_pci_devices(pci_devices: &PciDevices) -> Option<Controller> {
    let mut xhc = None;
    for dev in pci_devices.iter() {
        match dev.class_code.base {
            // Mass Storage Controller
            0x01 => {
                match dev.class_code.sub {
                    0x01 => {
                        let mut controller = initialize_ide(&dev);
                        let mut edi = [0u8; 8];
                        controller.read_pata(0, 1, 0, edi.as_mut_ptr() as u32);
                        print!("image: ");
                        for i in 0..8 {
                            print!("{:x} ", edi[i]);
                        }
                        println!("");
                    },
                    _ => {info!("pci device isn't supported. Class Code: {:?}", dev.class_code);}
                }
            },
            // Network Controller
            0x02 => {info!("pci device isn't supported. Class Code: {:?}", dev.class_code);},
            // Display Controller
            0x03 => {
                match dev.get_vendor_id() {
                    0x1234 => {
                        setup_qemu_card(&dev);
                    }
                    _ => {info!("pci device isn't supported. Class Code: {:?}", dev.class_code);}
                }
            },
            // Multimedia Controller
            0x04 => {info!("pci device isn't supported. Class Code: {:?}", dev.class_code);},
            // Serial Bus Controller
            0x0c => {
                match dev.class_code.interface {
                    0x20 => {
                        if dev.get_vendor_id() == 0x8086 {
                            switch_echi2xhci(&dev);
                            xhc = Some(initialize_xhci(&dev));
                        }
                    },
                    0x30 => {
                        xhc = Some(initialize_xhci(&dev));
                    },
                    _ => {info!("pci device isn't supported. Class Code: {:?}", dev.class_code);}
                }
            },
            _ => {info!("pci device isn't supported. Class Code: {:?}", dev.class_code);}
        }
    }
    return xhc
}