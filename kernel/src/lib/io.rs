use core::arch::asm;

#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let result: u8;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") result,
        options(nomem, nostack)
    );
    return result
}

#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
    let result: u16;
    asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") result,
        options(nomem, nostack)
    );
    return result
}

#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
    let result: u32;
    asm!(
        "in eax, dx",
        in("dx") port,
        out("eax") result,
        options(nomem, nostack)
    );
    return result
}

#[inline(always)]
pub unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack)
    );
}

#[inline(always)]
pub unsafe fn outw(port: u16, value: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, nostack)
    );
}

#[inline(always)]
pub unsafe fn outl(port: u16, value: u32) {
    asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
        options(nomem, nostack)
    );
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
