//! A small log file for troubleshooting: `%APPDATA%\SoundHop\soundhop.log`.
//!
//! Every line is written and closed immediately, so nothing stays open or
//! buffered while SoundHop is idle. When the file passes 256 KB it is renamed
//! to `soundhop.old.log` and a new one is started, so it never grows large.

use std::io::Write;

use windows::Win32::System::SystemInformation::GetLocalTime;

use crate::config;

const MAX_BYTES: u64 = 256 * 1024;

pub fn path() -> std::path::PathBuf {
    config::dir().join("soundhop.log")
}

/// Appends one line with a timestamp. Errors are ignored: logging must never
/// break the app.
pub fn write(message: &str) {
    let path = path();
    let _ = std::fs::create_dir_all(config::dir());
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::rename(&path, config::dir().join("soundhop.old.log"));
    }
    let t = unsafe { GetLocalTime() };
    let line = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}  {message}\n",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    );
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// `log!("switched to {}", name)` works like `format!` and writes the result
/// to the log file. (A Rust macro: code that expands at compile time.)
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(&format!($($arg)*))
    };
}
