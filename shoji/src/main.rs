#![no_std]
#![no_main]

use core::panic::PanicInfo;
use horse_syscall::prelude::*;
use horse_syscall::println;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PixelFormat {
    Rgb = 0,
    Bgr,
    Bitmask,
    BltOnly,
    InvalidFormat,
}

impl From<u8> for PixelFormat {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Rgb,
            1 => Self::Bgr,
            2 => Self::Bitmask,
            3 => Self::BltOnly,
            _ => Self::InvalidFormat,
        }
    }
}

fn convert_metadata(metadata: &[u8]) -> (u32, u32, u32, PixelFormat) {
    let mut buf: [u8; 4] = [0; 4];
    buf.copy_from_slice(&metadata[0..4]);
    let width = u32::from_le_bytes(buf);
    buf.copy_from_slice(&metadata[4..8]);
    let height = u32::from_le_bytes(buf);
    buf.copy_from_slice(&metadata[8..12]);
    let stride = u32::from_le_bytes(buf);
    let format = PixelFormat::from(metadata[12]);
    return (width, height, stride, format);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("---frame buffer device---");

    let fd = open("/dev/fb", OpenFlags::RDWR).expect("open 1");

    // read test
    let mut buf = [0u8; 13];
    let n = read(fd, &mut buf).expect("read fb metadata");
    println!("read frame buffer metadata: {} bytes", n);
    let (width, height, stride, format) = convert_metadata(&buf);
    println!(
        "width: {}, height: {}, stride: {}, format: {:?}",
        width, height, stride, format
    );

    // write test: paint first 512 pixels with a bright color to verify visually.
    // 512 pixels * 4 bytes/pixel = 2048 bytes (safe on stack).
    // Magenta (R=255, G=0, B=255) is visible regardless of format byte order.
    const PIXELS: usize = 1920 * 1080;
    let mut pixel_buf = [0u8; PIXELS * 4];
    for i in 0..PIXELS {
        let base = i * 4;
        match format {
            PixelFormat::Rgb => {
                pixel_buf[base] = 0x00; // R
                pixel_buf[base + 1] = 0x00; // G
                pixel_buf[base + 2] = 0xff; // B
                                            // [base+3] padding stays 0
            }
            PixelFormat::Bgr => {
                pixel_buf[base] = 0xff; // B
                pixel_buf[base + 1] = 0x00; // G
                pixel_buf[base + 2] = 0x00; // R
                                            // [base+3] padding stays 0
            }
            _ => {
                pixel_buf[base] = 0x00;
                pixel_buf[base + 1] = 0x00;
                pixel_buf[base + 2] = 0x00;
                // [base+3] padding stays 0
            }
        }
    }
    let mut fb_info = FbVarScreenInfo::default();
    ioctl(
        fd,
        IoctlRequest::FbIoGetVScreeninfo as u64,
        &mut fb_info as *mut _ as u64,
    )
    .expect("ioctl get resolution");
    fb_info.width = 1920;
    fb_info.height = 1080;
    fb_info.xres = 1920;
    fb_info.yres = 1080;
    ioctl(
        fd,
        IoctlRequest::FbIoPutVScreeninfo as u64,
        &mut fb_info as *mut _ as u64,
    )
    .expect("ioctl set resolution");
    ioctl(
        fd,
        IoctlRequest::FbIoGetVScreeninfo as u64,
        &mut fb_info as *mut _ as u64,
    )
    .expect("ioctl get resolution");
    let _ = write(fd, &pixel_buf);

    let _ = close(fd);

    exit(0);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let _ = write(STDERR, b"PANIC: ");
    if let Some(location) = info.location() {
        let _ = write(STDERR, location.file().as_bytes());
    } else {
        let _ = write(STDERR, b"unknown location");
    }
    let _ = write(STDERR, b"\n");
    loop {
        core::hint::spin_loop();
    }
}
