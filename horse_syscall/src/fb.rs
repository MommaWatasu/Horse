//! Framebuffer device wrappers
//!
//! Provides high-level wrappers around framebuffer ioctl calls.
//!
//! ## Example
//!
//! ```rust,ignore
//! use horse_syscall::fb::{fb_get_vscreeninfo, fb_put_vscreeninfo, fb_get_fscreeninfo};
//! use horse_syscall::fs::{open, OpenFlags};
//!
//! let fd = open("/dev/fb0", OpenFlags::RDWR).unwrap();
//! let vinfo = fb_get_vscreeninfo(fd).unwrap();
//! let finfo = fb_get_fscreeninfo(fd).unwrap();
//! ```

use crate::error::{check_syscall, Result};
use crate::raw::{syscall3, Fd, SyscallNum};

pub use horse_abi::fb::{
    FbBitfield, FbFixScreenInfo, FbVarScreenInfo, FBIOGET_FSCREENINFO, FBIOGET_VSCREENINFO,
    FBIOPUT_VSCREENINFO,
};

/// Perform a raw ioctl syscall
///
/// # Arguments
///
/// * `fd` - File descriptor
/// * `request` - ioctl request code (e.g. [`FBIOGET_VSCREENINFO`])
/// * `arg` - Request-specific argument (usually a pointer cast to u64)
pub fn ioctl(fd: Fd, request: u64, arg: u64) -> Result<()> {
    let ret = unsafe {
        syscall3(
            SyscallNum::Ioctl as usize,
            fd as usize,
            request as usize,
            arg as usize,
        )
    };
    check_syscall(ret).map(|_| ())
}

/// Read the variable screen info from a framebuffer device
///
/// # Arguments
///
/// * `fd` - File descriptor for a framebuffer device (e.g. `/dev/fb0`)
///
/// # Returns
///
/// * `Ok(FbVarScreenInfo)` - Resolution, bpp, and color layout
/// * `Err(e)` - Error on failure
pub fn fb_get_vscreeninfo(fd: Fd) -> Result<FbVarScreenInfo> {
    let mut info = unsafe { core::mem::zeroed::<FbVarScreenInfo>() };
    ioctl(fd, FBIOGET_VSCREENINFO, &mut info as *mut FbVarScreenInfo as u64)?;
    Ok(info)
}

/// Write variable screen info to a framebuffer device
///
/// Only `xres`, `yres`, and `bits_per_pixel` (must be 32) are applied by the kernel.
///
/// # Arguments
///
/// * `fd` - File descriptor for a framebuffer device
/// * `info` - Desired variable screen parameters
pub fn fb_put_vscreeninfo(fd: Fd, info: &FbVarScreenInfo) -> Result<()> {
    ioctl(fd, FBIOPUT_VSCREENINFO, info as *const FbVarScreenInfo as u64)
}

/// Read the fixed screen info from a framebuffer device
///
/// # Arguments
///
/// * `fd` - File descriptor for a framebuffer device
///
/// # Returns
///
/// * `Ok(FbFixScreenInfo)` - Physical address, buffer size, and stride
/// * `Err(e)` - Error on failure
pub fn fb_get_fscreeninfo(fd: Fd) -> Result<FbFixScreenInfo> {
    let mut info = unsafe { core::mem::zeroed::<FbFixScreenInfo>() };
    ioctl(fd, FBIOGET_FSCREENINFO, &mut info as *mut FbFixScreenInfo as u64)?;
    Ok(info)
}
