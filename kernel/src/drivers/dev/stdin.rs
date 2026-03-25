use crate::horse_lib::fd::FileDescriptor;

pub struct StdinDevice;

impl FileDescriptor for StdinDevice {
    fn read(&self, _buf: &mut [u8]) -> isize {
        // Keyboard integration not yet implemented
        0 // EOF
    }
    fn write(&self, _buf: &[u8]) -> isize {
        -22 // EINVAL
    }
    fn close(&self) {}
}
