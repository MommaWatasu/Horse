use crate::{console::Console, horse_lib::fd::FileDescriptor, layer::LAYER_MANAGER};

fn write_to_console(buf: &[u8]) -> isize {
    if let Ok(s) = core::str::from_utf8(buf) {
        let mut console = Console::instance();
        if let Some(ref mut con) = *console {
            con.put_string(s);
        }
    } else {
        let mut console = Console::instance();
        if let Some(ref mut con) = *console {
            for &byte in buf {
                let mut char_buf = [0u8; 4];
                let c = byte as char;
                if let Some(s) = c.encode_utf8(&mut char_buf).get(..c.len_utf8()) {
                    con.put_string(s);
                }
            }
        }
    }
    LAYER_MANAGER.lock().as_mut().unwrap().draw();
    buf.len() as isize
}

pub struct StdoutDevice;

impl FileDescriptor for StdoutDevice {
    fn read(&self, _buf: &mut [u8]) -> isize {
        -22 // EINVAL
    }
    fn write(&self, buf: &[u8]) -> isize {
        write_to_console(buf)
    }
    fn close(&self) {}
}

pub struct StderrDevice;

impl FileDescriptor for StderrDevice {
    fn read(&self, _buf: &mut [u8]) -> isize {
        -22 // EINVAL
    }
    fn write(&self, buf: &[u8]) -> isize {
        write_to_console(buf)
    }
    fn close(&self) {}
}
