use crate::drivers::video::qemu::change_qemu_resolution;
use crate::graphics::RAW_GRAPHICS;
use crate::horse_lib::fd::FileDescriptor;
use crate::syscall::SyscallError;
use horse_abi::{
    fb::{FbBitfield, FbFixScreenInfo, FbVarScreenInfo},
    ioctl::IoctlRequest,
};
use libloader::PixelFormat;

pub struct FrameBufferDevice;

impl FileDescriptor for FrameBufferDevice {
    /// Returns 13 bytes of framebuffer metadata:
    ///   width(4 LE) + height(4 LE) + stride(4 LE) + format(1)
    ///   format: 0 = RGB, 1 = BGR
    fn read(&self, buf: &mut [u8]) -> isize {
        const META_LEN: usize = 13;
        if buf.len() < META_LEN {
            return -22; // EINVAL
        }
        let guard = RAW_GRAPHICS.lock();
        let gfx = match guard.as_ref() {
            Some(g) => g,
            None => return -5, // EIO
        };
        let (width, height) = gfx.fb.resolution;
        let stride = gfx.fb.stride;
        let format = gfx.fb.format as u8;
        buf[0..4].copy_from_slice(&(width as u32).to_le_bytes());
        buf[4..8].copy_from_slice(&(height as u32).to_le_bytes());
        buf[8..12].copy_from_slice(&(stride as u32).to_le_bytes());
        buf[12] = format;
        return META_LEN as isize;
    }

    /// Writes raw pixel bytes to the physical framebuffer starting at offset 0.
    /// Buffer is clamped to framebuffer size (stride * height * 4 bytes).
    fn write(&self, buf: &[u8]) -> isize {
        if buf.is_empty() {
            return -22; // EINVAL
        }
        let guard = RAW_GRAPHICS.lock();
        let gfx = match guard.as_ref() {
            Some(g) => g,
            None => return -5, // EIO
        };
        let (_, height) = gfx.fb.resolution;
        let stride = gfx.fb.stride;
        let fb_size = stride * height * 4; // all supported formats are 4 bpp
        let write_len = buf.len().min(fb_size);
        unsafe {
            let fb_ptr = gfx.fb.get_fb_mut_ptr();
            core::ptr::copy_nonoverlapping(buf.as_ptr(), fb_ptr, write_len);
        }
        write_len as isize
    }

    fn close(&self) {}

    fn ioctl(&self, req: IoctlRequest, arg: u64) -> isize {
        match req {
            IoctlRequest::FbIoGetVScreeninfo => {
                let guard = RAW_GRAPHICS.lock();
                let gfx = match guard.as_ref() {
                    Some(g) => g,
                    None => return SyscallError::IoError as isize,
                };
                let (width, height) = gfx.fb.resolution;
                let (red, green, blue) = match gfx.fb.format {
                    PixelFormat::Rgb => (
                        FbBitfield {
                            offset: 0,
                            length: 8,
                            msb_right: 0,
                        },
                        FbBitfield {
                            offset: 8,
                            length: 8,
                            msb_right: 0,
                        },
                        FbBitfield {
                            offset: 16,
                            length: 8,
                            msb_right: 0,
                        },
                    ),
                    _ => (
                        FbBitfield {
                            offset: 16,
                            length: 8,
                            msb_right: 0,
                        },
                        FbBitfield {
                            offset: 8,
                            length: 8,
                            msb_right: 0,
                        },
                        FbBitfield {
                            offset: 0,
                            length: 8,
                            msb_right: 0,
                        },
                    ),
                };
                let info = FbVarScreenInfo {
                    xres: width as u32,
                    yres: height as u32,
                    xres_virtual: width as u32,
                    yres_virtual: height as u32,
                    xoffset: 0,
                    yoffset: 0,
                    bits_per_pixel: 32,
                    red,
                    green,
                    blue,
                    transp: FbBitfield {
                        offset: 24,
                        length: 8,
                        msb_right: 0,
                    },
                    nonstd: 0,
                    activate: 0,
                    height: 0xFFFFFFFF,
                    width: 0xFFFFFFFF,
                    accel_flags: 0,
                    pixclock: 0,
                    left_margin: 0,
                    right_margin: 0,
                    upper_margin: 0,
                    lower_margin: 0,
                    hsync_len: 0,
                    vsync_len: 0,
                    sync: 0,
                    vmode: 0,
                    rotate: 0,
                    colorspace: 0,
                    reserved: [0; 4],
                };
                drop(guard);
                unsafe {
                    crate::syscall::copy_to_user(
                        arg as *mut u8,
                        &info as *const FbVarScreenInfo as *const u8,
                        core::mem::size_of::<FbVarScreenInfo>(),
                    );
                }
                0
            }
            IoctlRequest::FbIoGetFScreeninfo => {
                let guard = RAW_GRAPHICS.lock();
                let gfx = match guard.as_ref() {
                    Some(g) => g,
                    None => return SyscallError::IoError as isize,
                };
                let (_, height) = gfx.fb.resolution;
                let stride = gfx.fb.stride;
                let fb_size = (stride * height * 4) as u32;
                let smem_start = unsafe { gfx.fb.get_fb_mut_ptr() } as u64;
                let mut id = [0u8; 16];
                let name = b"HorseOS FB";
                id[..name.len()].copy_from_slice(name);
                let info = FbFixScreenInfo {
                    id,
                    smem_start,
                    smem_len: fb_size,
                    fb_type: 0, // FB_TYPE_PACKED_PIXELS
                    type_aux: 0,
                    visual: 2, // FB_VISUAL_TRUECOLOR
                    xpanstep: 0,
                    ypanstep: 0,
                    ywrapstep: 0,
                    _pad: 0,
                    line_length: (stride * 4) as u32,
                    mmio_start: 0,
                    mmio_len: 0,
                    accel: 0,
                };
                drop(guard);
                unsafe {
                    crate::syscall::copy_to_user(
                        arg as *mut u8,
                        &info as *const FbFixScreenInfo as *const u8,
                        core::mem::size_of::<FbFixScreenInfo>(),
                    );
                }
                0
            }
            IoctlRequest::FbIoPutVScreeninfo => {
                let mut info = FbVarScreenInfo::default();
                unsafe {
                    crate::syscall::copy_from_user(
                        &mut info as *mut FbVarScreenInfo as *mut u8,
                        arg as *const u8,
                        core::mem::size_of::<FbVarScreenInfo>(),
                    );
                }
                if info.bits_per_pixel != 32 {
                    return SyscallError::InvalidArg as isize;
                }
                let new_xres = info.xres as usize;
                let new_yres = info.yres as usize;
                if new_xres == 0 || new_yres == 0 {
                    return SyscallError::InvalidArg as isize;
                }
                let needs_update = {
                    let mut guard = RAW_GRAPHICS.lock();
                    let gfx = match guard.as_mut() {
                        Some(g) => g,
                        None => return SyscallError::IoError as isize,
                    };
                    if (new_xres, new_yres) != gfx.fb.resolution {
                        unsafe {
                            gfx.change_resolution((new_xres, new_yres));
                        }
                        true
                    } else {
                        false
                    }
                }; // guard dropped here, before change_qemu_resolution
                if needs_update {
                    change_qemu_resolution((new_xres, new_yres));
                }
                0
            }
            _ => SyscallError::OpNotSupp as isize,
        }
    }
}
