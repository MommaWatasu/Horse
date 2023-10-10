use alloc::boxed::Box;

use super::{
    ata::{
        pata::initialize_ide,
        vata::VataController
    },
    fs::core::STORAGE_CONTROLLERS,
    pci::{switch_echi2xhci, PciDevices},
    usb::xhci::{initialize_xhci, Controller},
    video::qemu::setup_qemu_card,
};
use crate::{info, warn};

pub fn initialize_pci_devices(pci_devices: &PciDevices) -> Option<Controller> {
    let mut xhc = None;
    for dev in pci_devices.iter() {
        match dev.class_code.base {
            // Mass Storage Controller
            0x01 => match dev.class_code.sub {
                0x01 => {
                    STORAGE_CONTROLLERS.lock().push(Box::new(initialize_ide(&dev)));
                }
                _ => {
                    info!(
                        "pci device isn't supported. Class Code: {:?}",
                        dev.class_code
                    );
                }

            },
            // Network Controller
            0x02 => {
                info!(
                    "pci device isn't supported. Class Code: {:?}",
                    dev.class_code
                );
            }
            // Display Controller
            0x03 => match dev.get_vendor_id() {
                0x1234 => {
                    setup_qemu_card(&dev);
                }
                _ => {
                    info!(
                        "pci device isn't supported. Class Code: {:?}",
                        dev.class_code
                    );
                }
            },
            // Multimedia Controller
            0x04 => {
                info!(
                    "pci device isn't supported. Class Code: {:?}",
                    dev.class_code
                );
            }
            // Serial Bus Controller
            0x0c => match dev.class_code.interface {
                0x20 => {
                    if dev.get_vendor_id() == 0x8086 {
                        switch_echi2xhci(&dev);
                        xhc = Some(initialize_xhci(&dev));
                    }
                }
                0x30 => {
                    xhc = Some(initialize_xhci(&dev));
                }
                _ => {
                    info!(
                        "pci device isn't supported. Class Code: {:?}",
                        dev.class_code
                    );
                }
            },
            _ => {
                info!(
                    "pci device isn't supported. Class Code: {:?}",
                    dev.class_code
                );
            }
        }
    }
    let mut disk_controllers = STORAGE_CONTROLLERS.lock();
    if disk_controllers.len() == 0 {
        warn!("fallback: virtual hard disk will be used...");
        disk_controllers.push(Box::new(VataController::new()));
    }
    return xhc;
}
