use crate::horse_lib::fd::FileDescriptor;

pub struct NullDevice;

impl FileDescriptor for NullDevice {
    fn read(&self, _buf: &mut [u8]) -> isize {
        0 // EOF
    }
    fn write(&self, buf: &[u8]) -> isize {
        buf.len() as isize // discard
    }
    fn close(&self) {}
}
