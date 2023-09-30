pub trait Storage {
    fn read(&mut self, buf: &mut [u8], lba: u32, nbytes: usize) -> u8;
    fn write(&mut self, buf: &[u8], lba: u32, nbytes: usize) -> u8;
}
