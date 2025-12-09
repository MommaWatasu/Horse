//! I/O utilities and standard file descriptors
//!
//! This module provides convenient I/O operations for standard streams.

use crate::error::Result;
use crate::fs::write;
use crate::raw::Fd;
use core::fmt;

/// Standard input file descriptor
pub const STDIN: Fd = 0;

/// Standard output file descriptor
pub const STDOUT: Fd = 1;

/// Standard error file descriptor
pub const STDERR: Fd = 2;

/// A writer for standard output
pub struct Stdout;

impl Stdout {
    /// Write bytes to stdout
    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        write(STDOUT, buf)
    }

    /// Write all bytes to stdout
    pub fn write_all(&self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            let n = self.write(buf)?;
            buf = &buf[n..];
        }
        Ok(())
    }
}

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

/// A writer for standard error
pub struct Stderr;

impl Stderr {
    /// Write bytes to stderr
    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        write(STDERR, buf)
    }

    /// Write all bytes to stderr
    pub fn write_all(&self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            let n = self.write(buf)?;
            buf = &buf[n..];
        }
        Ok(())
    }
}

impl fmt::Write for Stderr {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

/// Get a writer for stdout
pub fn stdout() -> Stdout {
    Stdout
}

/// Get a writer for stderr
pub fn stderr() -> Stderr {
    Stderr
}

/// Print a formatted string to stdout
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::io::print;
///
/// print("Hello, World!");
/// ```
pub fn print(s: &str) {
    let _ = write(STDOUT, s.as_bytes());
}

/// Print a formatted string to stdout with a newline
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::io::println;
///
/// println("Hello, World!");
/// ```
pub fn println(s: &str) {
    let _ = write(STDOUT, s.as_bytes());
    let _ = write(STDOUT, b"\n");
}

/// Print formatted output to stdout
///
/// This is similar to the standard `print!` macro but for Horse OS.
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::print;
///
/// print!("Hello, {}!", "World");
/// ```
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::io::stdout(), $($arg)*);
    }};
}

/// Print formatted output to stdout with a newline
///
/// This is similar to the standard `println!` macro but for Horse OS.
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::println;
///
/// println!("Hello, {}!", "World");
/// ```
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = writeln!($crate::io::stdout(), $($arg)*);
    }};
}

/// Print formatted output to stderr
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::eprint;
///
/// eprint!("Error: {}", msg);
/// ```
#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::io::stderr(), $($arg)*);
    }};
}

/// Print formatted output to stderr with a newline
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::eprintln;
///
/// eprintln!("Error: {}", msg);
/// ```
#[macro_export]
macro_rules! eprintln {
    () => {
        $crate::eprint!("\n")
    };
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = writeln!($crate::io::stderr(), $($arg)*);
    }};
}
