//! Debugcon output module for QEMU debug output
//!
//! This module provides functions to output debug information to QEMU's debugcon
//! port (0xE9). Use `-debugcon stdio` or `-debugcon file:debug.log` when running QEMU.

/// Debugcon port for QEMU debug output (0xE9)
const DEBUGCON_PORT: u16 = 0xE9;

/// Write a single byte to debugcon
#[inline]
pub fn write_byte(byte: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") DEBUGCON_PORT,
            in("al") byte,
            options(nostack, preserves_flags)
        );
    }
}

/// Write a string to debugcon
pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

/// Write a hex number to debugcon
pub fn write_hex(value: u64) {
    write_str("0x");
    for i in (0..16).rev() {
        let nibble = ((value >> (i * 4)) & 0xF) as u8;
        let c = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + nibble - 10
        };
        write_byte(c);
    }
}

/// Write a decimal number to debugcon
pub fn write_dec(mut value: u64) {
    if value == 0 {
        write_byte(b'0');
        return;
    }

    let mut buf = [0u8; 20]; // u64 max is 20 digits
    let mut i = 0;

    while value > 0 {
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
        i += 1;
    }

    // Print in reverse order
    while i > 0 {
        i -= 1;
        write_byte(buf[i]);
    }
}

/// Write a newline to debugcon
#[inline]
pub fn write_newline() {
    write_byte(b'\n');
}

/// Macro for formatted debugcon output (similar to print!)
#[macro_export]
macro_rules! debugcon_print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::debugcon::DebugconWriter, $($arg)*);
    }};
}

/// Macro for formatted debugcon output with newline (similar to println!)
#[macro_export]
macro_rules! debugcon_println {
    () => {
        $crate::debugcon::write_newline();
    };
    ($($arg:tt)*) => {{
        $crate::debugcon_print!($($arg)*);
        $crate::debugcon::write_newline();
    }};
}

/// Writer struct for core::fmt::Write implementation
pub struct DebugconWriter;

impl core::fmt::Write for DebugconWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(s);
        Ok(())
    }
}
