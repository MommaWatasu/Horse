mod definition;
use definition::*;

use crate::{
    error,
    print,
    println,
    sleep,
    drivers::pci::*,
    lib::{
        bytes::{
            bytes2str,
            negative
        },
        io::*,
    },
};

use alloc::string::String;
use core::{
    arch::asm,
    ptr::{read, write},
};
use spin::Mutex;

const DEFAULT_IDE_DEVICE: IdeDevice = IdeDevice { reserved: 0, channel: 0, drive: 0, ata_type: 0, signature: 0, capabilities: 0, commandsets: 0, size: 0, model: [0; 41] };
const DEFAULT_CHANNEL_REGISTER: IdeChannelRegister = IdeChannelRegister { base: 0, ctrl: 0, bmide: 0, no_int: 0 };
static IDE_DEVICES: Mutex<[IdeDevice; 4]> = Mutex::new([DEFAULT_IDE_DEVICE; 4]);
static CHANNELS: Mutex<[IdeChannelRegister; 2]> = Mutex::new([DEFAULT_CHANNEL_REGISTER; 2]);

fn ide_read(channel: usize, reg: u16) -> u8 {
    let result: u8;
    if 0x07 < reg && reg < 0x0c {
        ide_write(channel, Register::AtaRegControlAltstatus as u16, 0x80 | CHANNELS.lock()[channel].no_int);
    }
    unsafe {
        if reg < 0x08 {
            result = inb(CHANNELS.lock()[channel].base + reg - 0x00);
        } else if reg < 0x0c {
            result = inb(CHANNELS.lock()[channel].base + reg - 0x06);
        } else if reg < 0x0e {
            result = inb(CHANNELS.lock()[channel].ctrl + reg - 0x0a);
        } else if reg < 0x16 {
            result = inb(CHANNELS.lock()[channel].bmide + reg - 0x0e);
        } else {
            result = 0;
        }
    }
    if 0x07 < reg && reg < 0x0c {
        ide_write(channel, Register::AtaRegControlAltstatus as u16, CHANNELS.lock()[channel].no_int);
    }
    return result
}

fn ide_write(channel: usize, reg: u16, data: u8) {
    if 0x07 < reg && reg < 0x0c {
        ide_write(channel, Register::AtaRegControlAltstatus as u16, 0x80 | CHANNELS.lock()[channel].no_int);
    }
    unsafe {
        if reg < 0x08 {
            outb(CHANNELS.lock()[channel].base + reg - 0x00, data);
        } else if reg < 0x0c {
            outb(CHANNELS.lock()[channel].base + reg - 0x06, data);
        } else if reg < 0x0e {
            outb(CHANNELS.lock()[channel].ctrl + reg - 0x0a, data);
        } else if reg < 0x16 {
            outb(CHANNELS.lock()[channel].bmide + reg - 0x0e, data);
        }
    }
    if 0x07 < reg && reg < 0x0c {
        ide_write(channel, Register::AtaRegControlAltstatus as u16, CHANNELS.lock()[channel].no_int);
    }
}

/* WARNING: This code contains a serious bug. The inline assembly trashes ES and
*           ESP for all of the code the compiler generates between the inline
*           assembly blocks.
*/
fn ide_read_buffer(channel: usize, reg: u16, buffer: &mut [u32], quads: u32) {
    if reg > 0x07 && reg < 0x0C {
        ide_write(channel, Register::AtaRegControlAltstatus as u16, 0x80 | CHANNELS.lock()[channel].no_int);
    }
    unsafe {
        if reg < 0x08 {
            insl(CHANNELS.lock()[channel].base + reg - 0x00, buffer, quads);
        } else if reg < 0x0C {
            insl(CHANNELS.lock()[channel].base + reg - 0x06, buffer, quads);
        } else if reg < 0x0E {
            insl(CHANNELS.lock()[channel].ctrl + reg - 0x0a, buffer, quads);
        } else if reg < 0x16 {
            insl(CHANNELS.lock()[channel].bmide + reg - 0x0e, buffer, quads);
        }
    }
    if reg > 0x07 && reg < 0x0C {
        ide_write(channel, Register::AtaRegControlAltstatus as u16, CHANNELS.lock()[channel].no_int);
    }
}

fn ide_polling(channel: usize, advanced_check: u32) -> u8 {
    for i in 0..4 {
        ide_read(channel, Register::AtaRegControlAltstatus as u16);
    }

    while (ide_read(channel, Register::AtaRegCommandStatus as u16) & Status::AtaSrBsy as u8) != 0 {}

    if advanced_check != 0 {
        let state: u8 = ide_read(channel, Register::AtaRegCommandStatus as u16);

        // Check for Errors
        if (state & Status::AtaSrErr as u8) != 0 {
            return 2 //Error
        }
        // Check if device fault
        if (state & Status::AtaSrDf as u8) != 0 {
            return 1 // Device fault
        }
        // Check DRQ
        if (state & Status::AtaSrDrq as u8) == 0 {
            return 3 // DRQ should be set
        }
    }
    return 0;
}

fn ide_print_error(drive: usize, mut err: u8) -> u8 {
    if err == 0 { return 0 }
    print!("IDE: ");
    if err == 1 {
        println!("Device Fault");
        err = 19
    } else if err == 2 {
        let st = ide_read(IDE_DEVICES.lock()[drive].channel, Register::AtaRegErrorFeatures as u16);
        if st & Error::AtaErAmnf as u8 == 1 {
            println!("No Address Mark Found");
            err = 7;
        } else if st & Error::AtaErTk0nf as u8 == 1 {
            println!("No Media or Media Error");
            err = 3;
        } else if st & Error::AtaErAbrt as u8 == 1 {
            println!("Command Aborted");
            err = 20;
        } else if st & Error::AtaErMcr as u8 == 1 {
            println!("No Media or Media Error");
            err = 3;
        } else if st & Error::AtaErIdnf as u8 == 1 {
            println!("ID mark not Found");
            err = 21;
        } else if st & Error::AtaErMc as u8 == 1 {
            println!("No Media or Media Error");
            err = 3;
        } else if st & Error::AtaErUnc as u8 == 1 {
            println!("Uncorrectable Data Error");
            err = 22;
        } else if st & Error::AtaErBbk as u8 == 1 {
            println!("Bad Sectors");
            err = 13;
        }
    } else if err == 3 {
        println!("Reads Nothing");
        err = 25;
    } else if err == 4 {
        println!("Write Protected");
        err = 8;
    }
    let ide_device = IDE_DEVICES.lock()[drive];
    println!(
        "- [{}, {}] {}",
        ["Primary", "Secondary"][ide_device.channel],
        ["Master", "Slave"][ide_device.drive as usize],
        bytes2str(&ide_device.model)
    );
    return err;
}

pub fn initialize_ide(dev: &Device) {
    let mut bars = [0; 5];
    for i in 0..5 {
        bars[i] = read_bar32(dev, i).unwrap();
    }
    let mut count: usize = 0;

    // Detect I/O Port which interface IDE Controller
    let mut channels = [DEFAULT_CHANNEL_REGISTER; 2];
    channels[0].base  = ((bars[0] & 0x0000fffc) + 0x1f0 * (negative(bars[0]))).try_into().unwrap();
    channels[0].ctrl  = ((bars[1] & 0x0000fffc) + 0x3f6 * (negative(bars[1]))).try_into().unwrap();
    channels[1].base  = ((bars[2] & 0x0000fffc) + 0x170 * (negative(bars[2]))).try_into().unwrap();
    channels[1].ctrl  = ((bars[3] & 0x0000fffc) + 0x376 * (negative(bars[3]))).try_into().unwrap();
    channels[0].bmide = ((bars[4] & 0x0000fffc) + 0).try_into().unwrap(); // Bus Master IDE
    channels[1].bmide = ((bars[4] & 0x0000fffc) + 8).try_into().unwrap(); // Bus Master IDE
    *CHANNELS.lock() = channels;

    // Disable IRQs
    ide_write(Channel::Primary as usize, Register::AtaRegControlAltstatus as u16, 2);
    ide_write(Channel::Secondary as usize, Register::AtaRegControlAltstatus as u16, 2);

    // Detect ATA-ATAPI device
    for i in 0..2 {
        for j in 0..2 {
            let mut err: u8 = 0;
            let mut interface_type = InterfaceType::IdeAta;
            let mut status: u8;
            let mut ide_buf: [u32; 128] = [0; 128];
            IDE_DEVICES.lock()[count].reserved = 0; // Assuming that no driver here

            // Select Drive
            ide_write(i, Register::AtaRegHddevsel as u16, 0xa0 | (j << 4));
            sleep(1);

            status = ide_read(i, Register::AtaRegCommandStatus as u16);

            // Send Identification Command
            ide_write(i, Register::AtaRegCommandStatus as u16, Command::AtaCmdIdentify as u8);
            sleep(1);

            status = ide_read(i, Register::AtaRegCommandStatus as u16);

            // Polling
            if ide_read(i, Register::AtaRegCommandStatus as u16) == 0 { continue }
            loop {
                status = ide_read(i, Register::AtaRegCommandStatus as u16);
                if status & (Status::AtaSrErr as u8) != 0 { err = 1; break }
                if status & (Status::AtaSrBsy as u8) == 0 && status & (Status::AtaSrDrq as u8) != 0 { break }
                sleep(1);
            }

            // Probe for ATAPI device
            if err != 0 {
                ide_print_error(j.into(), err);
                let cl = ide_read(i, Register::AtaRegLba1 as u16);
                let ch = ide_read(i, Register::AtaRegLba2 as u16);

                if (cl == 0x14 && ch == 0xEB) || (cl == 0x69 && ch == 0x96) {
                    interface_type = InterfaceType::IdeAtapi;
                } else {
                    continue;
                }

                ide_write(i, Register::AtaRegCommandStatus as u16, Command::AtaCmdIdentifyPacket as u8);
                sleep(1);
            }

            // Read identification space of the device
            ide_read_buffer(i, Register::AtaRegData as u16, &mut ide_buf, 128);

            // Read device parameters
            let reserved = 1;
            let ata_type = interface_type as u16;
            let channel  = i;
            let drive    = j;
            let (signature, capabilities): (u16, u16);
            let (commandsets, size): (u32, u32);
            //println!("capabilities: {}", unsafe { *((ide_buf.as_ptr() as usize + Identification::AtaIdentCapabilities as usize) as *const u16) });
            unsafe {
                signature    = *((ide_buf.as_ptr() as usize + Identification::AtaIdentDevicetype as usize) as *const u16);
                capabilities = *((ide_buf.as_ptr() as usize + Identification::AtaIdentCapabilities as usize) as *const u16);
                commandsets  = *((ide_buf.as_ptr() as usize + Identification::AtaIdentCommandsets as usize) as *const u32);

                // Get size
                if commandsets & (1 << 26) != 0 {
                    size = *((ide_buf.as_ptr() as usize + Identification::AtaIdentMaxLbaExt as usize) as *const u32);
                } else {
                    size = *((ide_buf.as_ptr() as usize + Identification::AtaIdentMaxLba as usize) as *const u32);
                }
            }

            let mut model = [0u8; 41];
            unsafe { for k in (0..40).step_by(2) {
                model[k] = *((ide_buf.as_ptr() as usize + Identification::AtaIdentModel as usize + k + 1) as *const u8);
                model[k+1] = *((ide_buf.as_ptr() as usize + Identification::AtaIdentModel as usize + k) as *const u8);
            }}

            IDE_DEVICES.lock()[count] = IdeDevice {
                reserved,
                ata_type,
                channel,
                drive,
                signature,
                capabilities,
                commandsets,
                size,
                model
            };

            count += 1;
        }
    }

    // Print summary
    for i in 0..4 {
        let ide_device = IDE_DEVICES.lock()[i];
        if ide_device.reserved == 1 {
            if ide_device.size < 1024 * 1024 * 2 {
                println!(
                    "found {} drive {}MB - {}",
                    ["ATA", "ATAPI"][ide_device.ata_type as usize],
                    ide_device.size / 1024 / 2,
                    bytes2str(&ide_device.model)
                );
            } else {
                println!(
                    "found {} drive {}GB - {}",
                    ["ATA", "ATAPI"][ide_device.ata_type as usize],
                    ide_device.size / 1024 / 1024 / 2,
                    bytes2str(&ide_device.model)
                );
            }
        }
    }
}

fn pata_access(direction: u8, drive: usize, lba: u32, numsects: u8, selector: u16, mut edi: u32) -> u8 {
    let lba_mode: u8;
    let dma: u8;
    let mut lba_io = [0u8; 6];
    let channel: usize = IDE_DEVICES.lock()[drive].channel;
    let slavebit: u32 = IDE_DEVICES.lock()[drive].drive.into();
    let bus: u32 = CHANNELS.lock()[channel].base.into();
    let words: u32 = 256;
    let err: u8;
    let (cyl, i): (u16, u16);
    let (head, sect): (u32, u32);

    let ide_irq_invoked = 0x0;
    CHANNELS.lock()[channel].no_int = ide_irq_invoked + 0x02;
    ide_write(channel, Register::AtaRegControlAltstatus as u16, ide_irq_invoked + 0x02);

    // Select one from LBA28, LBA48 or CHS
    if lba >= 0x10000000 {
        // LBA48
        lba_mode  = 2;
        lba_io[0] = ((lba & 0x000000ff) >> 0).try_into().unwrap();
        lba_io[1] = ((lba & 0x0000ff00) >> 8).try_into().unwrap();
        lba_io[2] = ((lba & 0x00ff0000) >> 16).try_into().unwrap();
        lba_io[3] = ((lba & 0xff000000) >> 24).try_into().unwrap();
        // LBA28 is integer, so 32-bits are enough to access 2TB. So the lba_io[4..6] is left.
        head = 0;
    } else if IDE_DEVICES.lock()[drive].capabilities & 0x200 != 0 {
        // LBA28
        lba_mode  = 1;
        lba_io[0] = ((lba & 0x000000ff) >> 0).try_into().unwrap();
        lba_io[1] = ((lba & 0x0000ff00) >> 8).try_into().unwrap();
        lba_io[2] = ((lba & 0x00ff0000) >> 16).try_into().unwrap();
        // These Register are not here.
        head      = (lba & 0xf0000000) >> 24;
    } else {
        // CHS
        lba_mode  = 0;
        sect      = (lba % 63) + 1;
        cyl       = ((lba + 1 - sect) / (16 * 63)).try_into().unwrap();
        lba_io[0] = (sect & 0xff).try_into().unwrap();
        lba_io[1] = ((cyl >> 0) & 0xff).try_into().unwrap();
        lba_io[2] = ((cyl >> 8) & 0xff).try_into().unwrap();
        head      = (lba + 1 - sect) % (16 * 63) / 63;// Head number is written to HDDEVSEL lower 4-bits.
    }

    // See if the drive supports DMA or not
    dma = 0; // currently, we don't support DMA

    // Wait if the drive is busy
    while ide_read(channel, Register::AtaRegCommandStatus as u16) & Status::AtaSrBsy as u8 != 0 {}

    // Select the drive from controller
    if lba_mode == 0 {
        ide_write(channel, Register::AtaRegHddevsel as u16, ((0xa0 | (slavebit << 4) | head) & 0xff).try_into().unwrap()); // Drive and CHS
    } else {
        ide_write(channel, Register::AtaRegHddevsel as u16, ((0xe0 | (slavebit << 4) | head) & 0xff).try_into().unwrap()); // Drive and LBA
    }

    // Write parameters
    if lba_mode == 2 {
        ide_write(channel, Register::AtaRegSeccount1 as u16, 0);
        ide_write(channel, Register::AtaRegLba3 as u16, lba_io[3]);
        ide_write(channel, Register::AtaRegLba4 as u16, lba_io[4]);
        ide_write(channel, Register::AtaRegLba5 as u16, lba_io[5]);
    }
    ide_write(channel, Register::AtaRegSeccount0 as u16, numsects);
    ide_write(channel, Register::AtaRegLba0 as u16, lba_io[0]);
    ide_write(channel, Register::AtaRegLba1 as u16, lba_io[1]);
    ide_write(channel, Register::AtaRegLba2 as u16, lba_io[2]);

    // Set command
    let cmd: u8;
    match (lba_mode, dma, direction) {
        (0, 0, 0) => { cmd = Command::AtaCmdReadPio as u8 },
        (1, 0, 0) => { cmd = Command::AtaCmdReadPio as u8 },
        (2, 0, 0) => { cmd = Command::AtaCmdReadPioExt as u8 },
        (0, 1, 0) => { cmd = Command::AtaCmdReadDma as u8 },
        (1, 1, 0) => { cmd = Command::AtaCmdReadDma as u8 },
        (2, 1, 0) => { cmd = Command::AtaCmdReadDmaExt as u8 },
        (0, 0, 1) => { cmd = Command::AtaCmdWritePio as u8 },
        (1, 0, 1) => { cmd = Command::AtaCmdWritePio as u8 },
        (2, 0, 1) => { cmd = Command::AtaCmdWritePioExt as u8 },
        (0, 1, 1) => { cmd = Command::AtaCmdWriteDma as u8 },
        (1, 1, 1) => { cmd = Command::AtaCmdWriteDma as u8 },
        (2, 1, 1) => { cmd = Command::AtaCmdWriteDmaExt as u8 },
        (_, _, _) => {
            error!("invalid mode");
            return 20 
        }
    }
    ide_write(channel, Register::AtaRegCommandStatus as u16, cmd as u8);

    if dma != 0 {
        // TODO: Implement DMA R/W
        if direction == 0 {
            // DMA Read
        } else {
            // DMA Write
        }
    } else {
        if direction == 0 {
            // PIO Read
            for i in 0..numsects {
                let err = ide_polling(channel, 1);
                if err != 0 {
                    return err
                }
                unsafe {
                    // Recieve data
                    asm!(
                        "push bx",
                        "mov es, bx",
                        "push bx",
                        "mov ax, es",
                        "rep insw",
                        "pop bx",
                        "mov bx, es",
                        "pop bx",
                        in("ax") selector,
                        in("ecx") words,
                        in("edx") bus,
                        in("edi") edi
                    );
                }
                edi += words*2;
            }
        } else {
            // PIO Write
            for i in 0..numsects {
                ide_polling(channel, 0); // Polling.
                unsafe {
                    // Send data
                    asm!(
                        "push bx",
                        "mov ds, bx",
                        "push bx",
                        "mov ax, ds",
                        "rep outsw",
                        "pop bx",
                        "mov bx, ds",
                        "pop bx",
                        in("ax") selector,
                        in("ecx") words,
                        in("edx") bus,
                        in("esi") edi
                    );
                }
                edi += words*2;
            }
            if lba_mode == 2{
                ide_write(channel, Register::AtaRegCommandStatus as u16, Command::AtaCmdCacheFluxhExt as u8);
            } else {
                ide_write(channel, Register::AtaRegCommandStatus as u16, Command::AtaCmdCacheFluxh as u8);
            }
            ide_polling(channel, 0);
        }
    }
    return 0
}

pub fn read_pata(drive: usize, numsects: u8, lba: u32, es: u16, edi: u32) -> u8 {
    if drive > 3 || IDE_DEVICES.lock()[drive].reserved == 0 {
        return 1
    }
    let device = IDE_DEVICES.lock()[drive];
    if lba + numsects as u32 > device.size && device.ata_type == InterfaceType::IdeAta as u16 {
        return 2
    }
    let mut err = 0;
    if device.ata_type == InterfaceType::IdeAta as u16 {
        err = pata_access(Directions::Read as u8, drive, lba, numsects, es, edi);
    } else {
        for i in 0..numsects {
            err = pata_access(Directions::Read as u8, drive, lba, 1, es, edi + i as u32 * 2048)
        }
    }
    return ide_print_error(drive, err)
}