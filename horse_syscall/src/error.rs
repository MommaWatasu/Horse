//! Error types for system call results

pub use horse_abi::error::{Error, Result};

/// Convert a raw syscall return value to a Result
#[inline]
pub fn check_syscall(ret: isize) -> Result<usize> {
    if ret < 0 {
        Err(Error::from_syscall_ret(ret))
    } else {
        Ok(ret as usize)
    }
}
