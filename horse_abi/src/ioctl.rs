//! ioctl request codes shared between kernel and user-space

/// ioctl request codes
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoctlRequest {
    // Terminal
    Tiocgwinsz = 0x5413, // get window size
    Tiocswinsz = 0x5414, // set window size
    Tcgets = 0x5401,     // get termios
    Tcsets = 0x5402,     // set termios

    // Framebuffer
    FbIoGetVScreeninfo = 0x4600, // get framebuffer metadata (resolution, stride, format)
    FbIoPutVScreeninfo = 0x4601, // set framebuffer metadata (only resolution is supported)
    FbIoGetFScreeninfo = 0x4602, // get framebuffer information
    FbIoPanDisplay = 0x4606,     // pan display (scroll without changing framebuffer content)
    FbIoGetCon2FbMap = 0x460f,   // get console to framebuffer mapping
    FbIoBlank = 0x4611,          // blank display
}

impl TryFrom<u64> for IoctlRequest {
    type Error = ();

    fn try_from(v: u64) -> Result<Self, Self::Error> {
        match v {
            0x5413 => Ok(Self::Tiocgwinsz),
            0x5414 => Ok(Self::Tiocswinsz),
            0x5401 => Ok(Self::Tcgets),
            0x5402 => Ok(Self::Tcsets),
            0x4600 => Ok(Self::FbIoGetVScreeninfo),
            0x4601 => Ok(Self::FbIoPutVScreeninfo),
            0x4602 => Ok(Self::FbIoGetFScreeninfo),
            0x4606 => Ok(Self::FbIoPanDisplay),
            0x460f => Ok(Self::FbIoGetCon2FbMap),
            0x4611 => Ok(Self::FbIoBlank),
            _ => Err(()),
        }
    }
}
