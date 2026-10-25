//! "Start with Windows", using the per-user `Run` key in the registry.
//! Writing to HKEY_CURRENT_USER needs no admin rights.

use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

use crate::ui::wide;

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE: PCWSTR = w!("SoundHop");

fn command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\"", exe.display()))
}

/// `true` if Windows will start this copy of SoundHop at sign-in.
pub fn is_enabled() -> bool {
    let mut buf = [0u16; 1024];
    let mut size = std::mem::size_of_val(&buf) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            VALUE,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if result.is_err() {
        return false;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
    let current = String::from_utf16_lossy(&buf[..len]);
    // If the exe was moved, the old entry points somewhere else: treat as off.
    command().is_some_and(|c| c.eq_ignore_ascii_case(&current))
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    unsafe {
        if enabled {
            let cmd = wide(&command().ok_or("Could not find the SoundHop exe path")?);
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                VALUE,
                REG_SZ.0,
                Some(cmd.as_ptr() as *const _),
                (cmd.len() * 2) as u32,
            )
            .ok()
            .map_err(|e| e.message())
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE);
            Ok(())
        }
    }
}
