pub trait Storage {
    fn read(&mut self, bytes: u32, lba: u32, buf: &mut [u8]) -> u8;
    fn write(&mut self, bytes: u32, lba: u32, buf: &[u8]) -> u8;
}
