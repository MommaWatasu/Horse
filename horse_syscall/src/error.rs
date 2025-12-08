//! Error types for system call results

use core::fmt;

/// System call error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(isize)]
pub enum Error {
    /// Operation not permitted
    Perm = -1,
    /// No such file or directory
    NoEnt = -2,
    /// I/O error
    Io = -5,
    /// Bad file descriptor
    BadFd = -9,
    /// Try again
    Again = -11,
    /// Out of memory
    NoMem = -12,
    /// Permission denied
    Access = -13,
    /// File exists
    Exist = -17,
    /// Not a directory
    NotDir = -20,
    /// Is a directory
    IsDir = -21,
    /// Invalid argument
    Inval = -22,
    /// Too many open files
    MFile = -24,
    /// Function not implemented
    NoSys = -38,
    /// Unknown error
    Unknown = -255,
}

impl Error {
    /// Create an Error from a raw system call return value
    pub fn from_syscall_ret(ret: isize) -> Self {
        match ret {
            -1 => Error::Perm,
            -2 => Error::NoEnt,
            -5 => Error::Io,
            -9 => Error::BadFd,
            -11 => Error::Again,
            -12 => Error::NoMem,
            -13 => Error::Access,
            -17 => Error::Exist,
            -20 => Error::NotDir,
            -21 => Error::IsDir,
            -22 => Error::Inval,
            -24 => Error::MFile,
            -38 => Error::NoSys,
            _ => Error::Unknown,
        }
    }

    /// Get the error code as an isize
    pub fn code(self) -> isize {
        self as isize
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Perm => write!(f, "Operation not permitted"),
            Error::NoEnt => write!(f, "No such file or directory"),
            Error::Io => write!(f, "I/O error"),
            Error::BadFd => write!(f, "Bad file descriptor"),
            Error::Again => write!(f, "Try again"),
            Error::NoMem => write!(f, "Out of memory"),
            Error::Access => write!(f, "Permission denied"),
            Error::Exist => write!(f, "File exists"),
            Error::NotDir => write!(f, "Not a directory"),
            Error::IsDir => write!(f, "Is a directory"),
            Error::Inval => write!(f, "Invalid argument"),
            Error::MFile => write!(f, "Too many open files"),
            Error::NoSys => write!(f, "Function not implemented"),
            Error::Unknown => write!(f, "Unknown error"),
        }
    }
}

/// Result type for system call operations
pub type Result<T> = core::result::Result<T, Error>;

/// Convert a raw syscall return value to a Result
#[inline]
pub fn check_syscall(ret: isize) -> Result<usize> {
    if ret < 0 {
        Err(Error::from_syscall_ret(ret))
    } else {
        Ok(ret as usize)
    }
}
