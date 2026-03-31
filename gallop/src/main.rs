//! Special file test program for Horse OS
//!
//! Tests the behavior of /dev/null, /dev/zero, /dev/stdin,
//! /dev/stdout, and /dev/stderr special files.

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use horse_syscall::prelude::*;
use horse_syscall::{println, eprintln};

// ── test helpers ────────────────────────────────────────────────────────────

static mut PASS_COUNT: usize = 0;
static mut FAIL_COUNT: usize = 0;

macro_rules! pass {
    ($name:expr) => {{
        println!("[PASS] {}", $name);
        unsafe { PASS_COUNT += 1; }
    }};
}

macro_rules! fail {
    ($name:expr, $reason:expr) => {{
        println!("[FAIL] {} -- {}", $name, $reason);
        unsafe { FAIL_COUNT += 1; }
    }};
}

macro_rules! check {
    ($name:expr, $cond:expr, $reason:expr) => {
        if $cond {
            pass!($name);
        } else {
            fail!($name, $reason);
        }
    };
}

// ── individual tests ─────────────────────────────────────────────────────────

fn test_null() {
    println!("--- /dev/null ---");

    // open
    let fd = match open("/dev/null", OpenFlags::RDWR) {
        Ok(fd) => { pass!("open /dev/null"); fd }
        Err(e) => { fail!("open /dev/null", "open failed"); eprintln!("  err: {:?}", e); return; }
    };

    // write should succeed and report all bytes written
    let written = write(fd, b"discard me");
    check!("write to /dev/null", written == Ok(10), "expected Ok(10)");

    // read should return 0 (EOF)
    let mut buf = [0xffu8; 8];
    let n = read(fd, &mut buf);
    check!("read from /dev/null (EOF)", n == Ok(0), "expected Ok(0)");

    let _ = close(fd);
    pass!("close /dev/null");
}

fn test_zero() {
    println!("--- /dev/zero ---");

    let fd = match open("/dev/zero", OpenFlags::RDWR) {
        Ok(fd) => { pass!("open /dev/zero"); fd }
        Err(e) => { fail!("open /dev/zero", "open failed"); eprintln!("  err: {:?}", e); return; }
    };

    // read should fill buffer with zeros
    let mut buf = [0xffu8; 8];
    let n = read(fd, &mut buf);
    check!("read /dev/zero returns 8 bytes", n == Ok(8), "expected Ok(8)");
    check!("read /dev/zero fills zeros", buf.iter().all(|&b| b == 0), "buffer not all zero");

    // write should succeed
    let written = write(fd, b"anything");
    check!("write to /dev/zero", written == Ok(8), "expected Ok(8)");

    let _ = close(fd);
    pass!("close /dev/zero");
}

fn test_stdin() {
    println!("--- /dev/stdin ---");

    let fd = match open("/dev/stdin", OpenFlags::RDONLY) {
        Ok(fd) => { pass!("open /dev/stdin"); fd }
        Err(e) => { fail!("open /dev/stdin", "open failed"); eprintln!("  err: {:?}", e); return; }
    };

    // read returns 0 (keyboard not yet implemented)
    let mut buf = [0u8; 4];
    let n = read(fd, &mut buf);
    check!("read /dev/stdin (not implemented -> EOF)", n == Ok(0), "expected Ok(0)");

    // write should fail with EINVAL
    let r = write(fd, b"x");
    check!("write to /dev/stdin returns Err(Inval)", r == Err(Error::Inval), "expected Err(Inval)");

    let _ = close(fd);
    pass!("close /dev/stdin");
}

fn test_stdout() {
    println!("--- /dev/stdout ---");

    let fd = match open("/dev/stdout", OpenFlags::WRONLY) {
        Ok(fd) => { pass!("open /dev/stdout"); fd }
        Err(e) => { fail!("open /dev/stdout", "open failed"); eprintln!("  err: {:?}", e); return; }
    };

    // write should display on screen
    let msg = b"  (output via /dev/stdout fd)\n";
    let r = write(fd, msg);
    check!("write to /dev/stdout", r == Ok(msg.len()), "expected Ok(len)");

    // read should fail with EINVAL
    let mut buf = [0u8; 4];
    let r = read(fd, &mut buf);
    check!("read from /dev/stdout returns Err(Inval)", r == Err(Error::Inval), "expected Err(Inval)");

    let _ = close(fd);
    pass!("close /dev/stdout");
}

fn test_stderr() {
    println!("--- /dev/stderr ---");

    let fd = match open("/dev/stderr", OpenFlags::WRONLY) {
        Ok(fd) => { pass!("open /dev/stderr"); fd }
        Err(e) => { fail!("open /dev/stderr", "open failed"); eprintln!("  err: {:?}", e); return; }
    };

    let msg = b"  (output via /dev/stderr fd)\n";
    let r = write(fd, msg);
    check!("write to /dev/stderr", r == Ok(msg.len()), "expected Ok(len)");

    let _ = close(fd);
    pass!("close /dev/stderr");
}

fn test_nonexistent() {
    println!("--- nonexistent device ---");

    let r = open("/dev/nonexistent", OpenFlags::RDONLY);
    check!(
        "open /dev/nonexistent returns Err(NoEnt)",
        r == Err(Error::NoEnt),
        "expected Err(NoEnt)"
    );
}

fn test_fd_reuse() {
    println!("--- fd reuse after close ---");

    let fd1 = open("/dev/null", OpenFlags::RDONLY).expect("open 1");
    let _ = close(fd1);

    // After closing, opening again should get the same fd number
    let fd2 = open("/dev/null", OpenFlags::RDONLY).expect("open 2");
    check!("fd reused after close", fd1 == fd2, "fd numbers differ");
    let _ = close(fd2);
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PixelFormat {
    Rgb = 0,
    Bgr,
    Bitmask,
    BltOnly,
    InvalidFormat
}

impl From<u8> for PixelFormat {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Rgb,
            1 => Self::Bgr,
            2 => Self::Bitmask,
            3 => Self::BltOnly,
            _ => Self::InvalidFormat
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
    return (width, height, stride, format)
}

fn test_fb() {
    println!("---frame buffer device---");

    let fd = open("/dev/fb", OpenFlags::RDWR).expect("open 1");

    // read test
    let mut buf = [0u8; 13];
    let n = read(fd, &mut buf).expect("read fb metadata");
    println!("read frame buffer metadata: {} bytes", n);
    let (width, height, stride, format) = convert_metadata(&buf);
    println!("width: {}, height: {}, stride: {}, format: {:?}", width, height, stride, format);
    pass!("read /dev/fb");

    // write test: paint first 512 pixels with a bright color to verify visually.
    // 512 pixels * 4 bytes/pixel = 2048 bytes (safe on stack).
    // Magenta (R=255, G=0, B=255) is visible regardless of format byte order.
    const PIXELS: usize = 800 * 30;
    let mut pixel_buf = [0u8; PIXELS * 4];
    for i in 0..PIXELS {
        let base = i * 4;
        match format {
            PixelFormat::Rgb => {
                pixel_buf[base]     = 0xFF; // R
                pixel_buf[base + 1] = 0x00; // G
                pixel_buf[base + 2] = 0xFF; // B
                // [base+3] padding stays 0
            }
            PixelFormat::Bgr => {
                pixel_buf[base]     = 0xFF; // B
                pixel_buf[base + 1] = 0x00; // G
                pixel_buf[base + 2] = 0xFF; // R
                // [base+3] padding stays 0
            }
            _ => {
                pixel_buf[base]     = 0xFF;
                pixel_buf[base + 1] = 0xFF;
                pixel_buf[base + 2] = 0xFF;
                // [base+3] padding stays 0
            }
        }
    }
    for _ in 0..1000 {
        match write(fd, &pixel_buf) {
            Ok(n) => {
            }
            Err(e) => {
            }
        }
    }

    let _ = close(fd);
}

// ── entry point ──────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("========================================");
    println!("  Special File Tests");
    println!("========================================");

    test_null();
    test_zero();
    test_stdin();
    test_stdout();
    test_stderr();
    test_nonexistent();
    test_fd_reuse();
    test_fb();

    println!("========================================");
    let pass = unsafe { PASS_COUNT };
    let fail = unsafe { FAIL_COUNT };
    println!("Result: {} passed, {} failed", pass, fail);
    println!("========================================");

    match spawn("shoji") {
        Ok(pid) => println!("shoji spawned (pid={})", pid),
        Err(e) => eprintln!("failed to spawn shoji: {:?}", e),
    }

    exit(if fail == 0 { 0 } else { 1 });
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
