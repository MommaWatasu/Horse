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

    println!("========================================");
    let pass = unsafe { PASS_COUNT };
    let fail = unsafe { FAIL_COUNT };
    println!("Result: {} passed, {} failed", pass, fail);
    println!("========================================");

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
