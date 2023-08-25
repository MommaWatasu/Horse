use core::arch::asm;

extern "C" {
    pub fn inb(addr: u16) -> u8;
    pub fn inw(addr: u16) -> u16;
    pub fn inl(addr: u16) -> u32;
    pub fn outb(addr: u16, value: u8);
    pub fn outw(addr: u16, value: u16);
    pub fn outl(addr: u16, value: u32);
}

pub unsafe fn insb(port: u16, buffer: &mut [u8], count: u32) {
    for i in 0..count {
        buffer[i as usize] = inb(port);
    }
}

pub unsafe fn insw(port: u16, buffer: &mut [u16], count: u32) {
    for i in 0..count {
        buffer[i as usize] = inw(port);
    }
}

pub unsafe fn insl(port: u16, buffer: &mut [u32], count: u32) {
    for i in 0..count {
        buffer[i as usize] = inl(port);
    }
}

pub unsafe fn outsb(port: u16, buffer: &[u8], count: u32) {
    for i in 0..count {
        outb(port, buffer[i as usize]);
    }
}

pub unsafe fn outsw(port: u16, buffer: &[u16], count: u32) {
    for i in 0..count {
        outw(port, buffer[i as usize]);
    }
}

pub unsafe fn outsl(port: u16, buffer: &[u32], count: u32) {
    for i in 0..count {
        outl(port, buffer[i as usize]);
    }
}

//TODO: implement ioremap