//! File system operations
//!
//! This module provides safe wrappers around file-related system calls.

use crate::error::{check_syscall, Result};
use crate::raw::{syscall1, syscall2, syscall3, Fd, SyscallNum};

/// Exit the process
///
/// # Arguments
///
/// * `status` - Exit status code (0 for success, non-zero for error)
///
/// # Note
///
/// This function never returns.
pub fn exit(status: i32) -> ! {
    unsafe {
        syscall1(SyscallNum::Exit as usize, status as usize);
    }
    // Should never reach here, but loop forever just in case
    loop {
        core::hint::spin_loop();
    }
}

/// File open flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenFlags(u32);

impl OpenFlags {
    /// Open for reading only
    pub const RDONLY: Self = Self(0x0000);
    /// Open for writing only
    pub const WRONLY: Self = Self(0x0001);
    /// Open for reading and writing
    pub const RDWR: Self = Self(0x0002);
    /// Create file if it doesn't exist
    pub const CREAT: Self = Self(0x0100);
    /// Truncate file to zero length
    pub const TRUNC: Self = Self(0x0200);
    /// Append to file
    pub const APPEND: Self = Self(0x0400);

    /// Create a new OpenFlags with the given raw value
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Get the raw bits of the flags
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Combine two flags
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for OpenFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for OpenFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Open a file
///
/// # Arguments
///
/// * `path` - Path to the file (null-terminated or slice)
/// * `flags` - Open flags (see [`OpenFlags`])
///
/// # Returns
///
/// * `Ok(fd)` - File descriptor on success
/// * `Err(e)` - Error on failure
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::fs::{open, OpenFlags};
///
/// let fd = open("/test.txt", OpenFlags::RDONLY)?;
/// ```
pub fn open(path: &str, flags: OpenFlags) -> Result<Fd> {
    // We need a null-terminated string for the syscall
    // Since we're in no_std, we'll use a stack buffer
    let path_bytes = path.as_bytes();
    if path_bytes.len() >= 4096 {
        return Err(crate::error::Error::Inval);
    }

    // Create null-terminated path on stack
    let mut buf = [0u8; 4096];
    buf[..path_bytes.len()].copy_from_slice(path_bytes);
    // buf is already zero-initialized, so it's null-terminated

    let ret = unsafe {
        syscall2(
            SyscallNum::Open as usize,
            buf.as_ptr() as usize,
            flags.bits() as usize,
        )
    };

    check_syscall(ret).map(|fd| fd as Fd)
}

/// Open a file with a null-terminated path (more efficient)
///
/// # Safety
///
/// The path must be a valid null-terminated C string.
pub unsafe fn open_cstr(path: *const u8, flags: OpenFlags) -> Result<Fd> {
    let ret = syscall2(
        SyscallNum::Open as usize,
        path as usize,
        flags.bits() as usize,
    );
    check_syscall(ret).map(|fd| fd as Fd)
}

/// Read from a file descriptor
///
/// # Arguments
///
/// * `fd` - File descriptor
/// * `buf` - Buffer to read into
///
/// # Returns
///
/// * `Ok(n)` - Number of bytes read
/// * `Err(e)` - Error on failure
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::fs::read;
///
/// let mut buf = [0u8; 256];
/// let n = read(fd, &mut buf)?;
/// ```
pub fn read(fd: Fd, buf: &mut [u8]) -> Result<usize> {
    let ret = unsafe {
        syscall3(
            SyscallNum::Read as usize,
            fd as usize,
            buf.as_mut_ptr() as usize,
            buf.len(),
        )
    };
    check_syscall(ret)
}

/// Write to a file descriptor
///
/// # Arguments
///
/// * `fd` - File descriptor
/// * `buf` - Buffer to write from
///
/// # Returns
///
/// * `Ok(n)` - Number of bytes written
/// * `Err(e)` - Error on failure
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::fs::write;
///
/// let n = write(fd, b"Hello, World!\n")?;
/// ```
pub fn write(fd: Fd, buf: &[u8]) -> Result<usize> {
    let ret = unsafe {
        syscall3(
            SyscallNum::Write as usize,
            fd as usize,
            buf.as_ptr() as usize,
            buf.len(),
        )
    };
    check_syscall(ret)
}

/// Close a file descriptor
///
/// # Arguments
///
/// * `fd` - File descriptor to close
///
/// # Returns
///
/// * `Ok(())` - Success
/// * `Err(e)` - Error on failure
///
/// # Example
///
/// ```rust,ignore
/// use horse_syscall::fs::close;
///
/// close(fd)?;
/// ```
pub fn close(fd: Fd) -> Result<()> {
    let ret = unsafe { syscall1(SyscallNum::Close as usize, fd as usize) };
    check_syscall(ret).map(|_| ())
}

/// A file handle that automatically closes when dropped
#[derive(Debug)]
pub struct File {
    fd: Fd,
}

impl File {
    /// Open a file
    pub fn open(path: &str, flags: OpenFlags) -> Result<Self> {
        let fd = open(path, flags)?;
        Ok(Self { fd })
    }

    /// Get the raw file descriptor
    pub fn fd(&self) -> Fd {
        self.fd
    }

    /// Read from the file
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        read(self.fd, buf)
    }

    /// Write to the file
    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        write(self.fd, buf)
    }

    /// Write all bytes to the file
    pub fn write_all(&self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            let n = self.write(buf)?;
            buf = &buf[n..];
        }
        Ok(())
    }

    /// Consume self and return the raw file descriptor without closing
    pub fn into_raw(self) -> Fd {
        let fd = self.fd;
        core::mem::forget(self);
        fd
    }
}

impl Drop for File {
    fn drop(&mut self) {
        let _ = close(self.fd);
    }
}
