use alloc::string::String;

pub fn bytes2str(bytes: &[u8]) -> String {
    return String::from_utf8(bytes.to_vec()).unwrap();
}

pub fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    let data = unsafe { data as *const T as *const u8 };
    let mut sum: u8 = 0;
    for i in 0..bytes {
        sum.wrapping_add(unsafe { *data.wrapping_add(i) });
    }
    return sum;
}

pub fn negative(x: u32) -> u32 {
    if x != 0 {
        return 0;
    } else {
        return 1;
    }
}