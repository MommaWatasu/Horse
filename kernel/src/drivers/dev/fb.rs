use crate::horse_lib::fd::FileDescriptor;
use crate::graphics::RAW_GRAPHICS;

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
        crate::debugcon::write_dec(format as u32 as u64);
        return META_LEN as isize
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
}
