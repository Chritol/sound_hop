//! Small helpers around raw Win32 calls, shared by the tray, the notification
//! and the settings window.
//!
//! Win32 talks in UTF-16 strings and integer messages. These helpers hide that
//! so the rest of the code can use normal Rust `&str` and `String`.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_ICONWARNING, MB_OK};

/// Converts a Rust string into a null-terminated UTF-16 buffer for Win32.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copies a Rust string into a fixed-size UTF-16 array (for struct fields like
/// `szTip`), cutting it off if it is too long.
pub fn copy_to_fixed(dst: &mut [u16], s: &str) {
    let mut n = 0;
    for (slot, ch) in dst.iter_mut().zip(s.encode_utf16()) {
        *slot = ch;
        n += 1;
    }
    let end = n.min(dst.len() - 1);
    dst[end] = 0;
}

pub fn hinstance() -> HINSTANCE {
    unsafe { GetModuleHandleW(None).map(|m| HINSTANCE(m.0)).unwrap_or_default() }
}

/// Scales a size given at 100% (96 DPI) to the screen's scale factor.
pub fn scale(v: i32) -> i32 {
    let dpi = unsafe { GetDpiForSystem() } as i32;
    v * dpi / 96
}

pub fn error_box(owner: Option<HWND>, text: &str) {
    let text = wide(text);
    unsafe {
        MessageBoxW(owner, PCWSTR(text.as_ptr()), w!("SoundHop"), MB_OK | MB_ICONERROR);
    }
}

pub fn warning_box(owner: Option<HWND>, text: &str) {
    let text = wide(text);
    unsafe {
        MessageBoxW(owner, PCWSTR(text.as_ptr()), w!("SoundHop"), MB_OK | MB_ICONWARNING);
    }
}

/// Reads a string from a null-terminated UTF-16 pointer owned by Windows.
pub fn from_wide_ptr(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0;
        while *p.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, len))
    }
}

/// `true` when Windows uses the light theme for the taskbar and flyouts.
pub fn system_uses_light_theme() -> bool {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    result.is_ok() && value != 0
}
