use crate::horse_lib::fd::FileDescriptor;

pub struct ZeroDevice;

impl FileDescriptor for ZeroDevice {
    fn read(&self, buf: &mut [u8]) -> isize {
        buf.fill(0);
        buf.len() as isize
    }
    fn write(&self, buf: &[u8]) -> isize {
        buf.len() as isize // discard
    }
    fn close(&self) {}
}
