//! System call numbers shared between kernel and user-space

/// System call numbers (Linux-compatible where applicable)
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallNum {
    /// Read from a file descriptor
    Read = 0,
    /// Write to a file descriptor
    Write = 1,
    /// Open a file
    Open = 2,
    /// Close a file descriptor
    Close = 3,
    /// Create a socket
    Socket = 41,
    /// Connect a socket to an address
    Connect = 42,
    /// Accept a connection on a socket
    Accept = 43,
    /// Bind a socket to an address
    Bind = 49,
    /// Listen for connections on a socket
    Listen = 50,
    /// ioctl — device-specific control
    Ioctl = 54,
    /// Exit the process
    Exit = 60,
    /// Spawn a new process from an ELF binary (Horse OS-specific)
    Spawn = 900,
}

impl TryFrom<usize> for SyscallNum {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Read),
            1 => Ok(Self::Write),
            2 => Ok(Self::Open),
            3 => Ok(Self::Close),
            41 => Ok(Self::Socket),
            42 => Ok(Self::Connect),
            43 => Ok(Self::Accept),
            49 => Ok(Self::Bind),
            50 => Ok(Self::Listen),
            54 => Ok(Self::Ioctl),
            60 => Ok(Self::Exit),
            900 => Ok(Self::Spawn),
            _ => Err(()),
        }
    }
}
