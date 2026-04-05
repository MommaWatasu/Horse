//! Socket ABI types shared between kernel and user-space

/// Unix domain socket address
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SocketAddrUn {
    /// Address family (AF_UNIX = 1)
    pub sun_family: u16,
    /// Socket path (null-terminated, max 108 bytes)
    pub sun_path: [u8; 108],
}

impl SocketAddrUn {
    /// Create a new Unix domain socket address from a path string.
    ///
    /// Returns `None` if the path is too long (> 107 bytes).
    pub fn new(path: &str) -> Option<Self> {
        let bytes = path.as_bytes();
        if bytes.len() > 107 {
            return None;
        }
        let mut addr = Self {
            sun_family: AF_UNIX as u16,
            sun_path: [0u8; 108],
        };
        addr.sun_path[..bytes.len()].copy_from_slice(bytes);
        Some(addr)
    }
}

/// Address family: Unix domain sockets
pub const AF_UNIX: i32 = 1;

/// Socket type: stream (connection-oriented)
pub const SOCK_STREAM: i32 = 1;
/// Socket type: datagram (connectionless)
pub const SOCK_DGRAM: i32 = 2;
