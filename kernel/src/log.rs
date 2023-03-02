use core::fmt::Write;
use crate::LAYER_MANAGER;

static LOG_LEVEL_DISPLAY: [&str; 6] = ["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
pub static LOG_LEVEL: spin::Mutex<LogLevel> = spin::Mutex::new(LogLevel::Debug);

#[repr(usize)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        LOG_LEVEL_DISPLAY[*self as usize]
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export(local_inner_macro)]
macro_rules! log {
    (level: $level:expr, $fmt:expr) => {
        if $level <= $crate::_log_level() {
            $crate::print!("[ {} ]", $level.as_str());
            $crate::print!(core::concat!($fmt, "\n"));
        }
    };
    (level: $level:expr, $fmt:expr, $($arg:tt)*) => {
        if $level <= $crate::_log_level() {
            $crate::print!("[ {} ]", $level.as_str());
            $crate::print!(core::concat!($fmt, "\n"), $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! status_log {
    ($status:expr ,$fmt:expr) => {
        match $status {
            StatusCode::Success => {
                crate::print!("[ OK ]");
                crate::print!(core::concat!($fmt, "\n"))
            },
            _ => {
                crate::print!("[ Error ]");
                crate::print!(core::concat!($fmt, "\n"))
            }
        }
    };
    ($status:expr, $fmt:expr, $($arg:tt)*) => {
        match $status {
            StatusCode::Success => {
                crate::print!("[ OK ]");
                crate::print!(core::concat!($fmt, "\n"), $($arg)*)
            },
            _ => {
                crate::print!("[ Error ]");
                crate::print!(core::concat!($fmt, "\n"), $($arg)*)
            }
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::log!(level: $crate::LogLevel::Error, $($arg)*));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::log!(level: $crate::LogLevel::Warn, $($arg)*));
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::log!(level: $crate::LogLevel::Info, $($arg)*));
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::log!(level: $crate::LogLevel::Debug, $($arg)*));
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ($crate::log!(level: $crate::LogLevel::Trace, $($arg)*));
}

pub fn _print(args: core::fmt::Arguments) {
    let mut locked_console = crate::console::Console::instance();
    let console = locked_console.as_mut().unwrap();
    console.write_fmt(args).unwrap();
    unsafe { LAYER_MANAGER.get_mut().unwrap().draw(); }
}

pub fn _log_level() -> LogLevel {
    *LOG_LEVEL.lock()
}
