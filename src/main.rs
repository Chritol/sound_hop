//! SoundHop: switch the Windows default audio devices from the tray or with
//! global hotkeys.
//!
//! Module overview (see docs/architecture.md for more):
//! - `app`       hidden main window, tray menu, hotkeys, switching logic
//! - `audio`     Windows Core Audio: list devices, set the default device
//! - `config`    the TOML settings file
//! - `hotkey`    hotkey text <-> Windows key codes
//! - `osd`       the small on-screen notice
//! - `settings`  the settings window
//! - `autostart` "Start with Windows"
//! - `ui`        small Win32 helpers

// Build as a GUI app: no console window pops up.
#![windows_subsystem = "windows"]
// On other systems only the unit-tested modules are built, so most of them look unused.
#![cfg_attr(not(windows), allow(dead_code))]

// The Windows-only parts are left out on other systems, so `cargo test` can
// run the platform-independent unit tests (config, hotkeys) anywhere.
#[cfg(windows)]
mod app;
#[cfg(windows)]
mod audio;
#[cfg(windows)]
mod autostart;
mod config;
mod hotkey;
#[cfg(windows)]
mod osd;
#[cfg(windows)]
mod settings;
#[cfg(windows)]
mod ui;

#[cfg(not(windows))]
fn main() {
    eprintln!("SoundHop only runs on Windows.");
}

#[cfg(windows)]
fn main() {
    if let Err(e) = app::run() {
        ui::error_box(None, &format!("SoundHop could not start:\n\n{}", e.message()));
    }
}
