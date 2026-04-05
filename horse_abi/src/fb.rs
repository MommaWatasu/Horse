//! Framebuffer ABI types shared between kernel and user-space

/// ioctl request code: get variable screen info
pub const FBIOGET_VSCREENINFO: u64 = 0x4600;
/// ioctl request code: set variable screen info
pub const FBIOPUT_VSCREENINFO: u64 = 0x4601;
/// ioctl request code: get fixed screen info
pub const FBIOGET_FSCREENINFO: u64 = 0x4602;

/// Fixed framebuffer hardware information (read-only)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FbFixScreenInfo {
    pub id: [u8; 16],    // "HorseOS FB\0..."
    pub smem_start: u64, // Physical address of the framebuffer
    pub smem_len: u32,   // Size of the framebuffer (bytes)
    pub fb_type: u32,    // FB_TYPE_PACKED_PIXELS = 0
    pub type_aux: u32,
    pub visual: u32, // FB_VISUAL_TRUECOLOR = 2
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub _pad: u16,
    pub line_length: u32, // Bytes per line (stride)
    pub mmio_start: u64,
    pub mmio_len: u32,
    pub accel: u32,
}

/// Variable framebuffer parameters (resolution, color depth, etc.)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FbVarScreenInfo {
    pub xres: u32,         // Display resolution X
    pub yres: u32,         // Display resolution Y
    pub xres_virtual: u32, // Virtual resolution X (double buffering, etc.)
    pub yres_virtual: u32,
    pub xoffset: u32, // Pan offset X
    pub yoffset: u32, // Pan offset Y

    pub bits_per_pixel: u32, // 32

    // Color channel bitfields
    pub red: FbBitfield,
    pub green: FbBitfield,
    pub blue: FbBitfield,
    pub transp: FbBitfield,

    pub nonstd: u32,
    pub activate: u32,
    pub height: u32, // mm units (0xFFFFFFFF if unknown)
    pub width: u32,
    pub accel_flags: u32,

    // Timing (for VGA hardware; 0 is fine for software FB)
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}

/// Bit-field description for a single color channel
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FbBitfield {
    pub offset: u32,    // Bit position
    pub length: u32,    // Number of bits
    pub msb_right: u32, // Usually 0
}
