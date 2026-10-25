//! The settings file: `%APPDATA%\SoundHop\config.toml`.
//!
//! The settings window writes this file, but it is plain TOML, so it can also
//! be edited by hand. `#[derive(Serialize, Deserialize)]` generates the
//! reading and writing code, much like a Kotlin `@Serializable` data class.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Whole configuration. `#[serde(default)]` means a missing field gets its
/// default value instead of failing, so older or hand-written files keep
/// working.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    /// Playback devices SoundHop manages, in cycle order.
    pub outputs: Vec<DeviceEntry>,
    /// Recording devices SoundHop manages, in cycle order.
    pub inputs: Vec<DeviceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    /// Show the small on-screen notice after a switch.
    pub show_notification: bool,
    /// How long the notice stays visible, in milliseconds.
    pub notification_ms: u32,
    /// Hotkey that cycles through `outputs`. Empty means none.
    pub cycle_output_hotkey: String,
    /// Hotkey that cycles through `inputs`. Empty means none.
    pub cycle_input_hotkey: String,
}

impl Default for General {
    fn default() -> Self {
        Self {
            show_notification: true,
            notification_ms: 1500,
            cycle_output_hotkey: "Ctrl+Alt+0".into(),
            cycle_input_hotkey: String::new(),
        }
    }
}

/// One device the user chose to manage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceEntry {
    /// Windows endpoint ID, e.g. `{0.0.0.00000000}.{a1b2...}`. Stays the same
    /// when the device is unplugged and plugged back in.
    pub id: String,
    /// Name shown in the menu and the notice. Can be renamed by the user.
    pub name: String,
    /// e.g. `Ctrl+Alt+1`. Empty means no hotkey.
    pub hotkey: String,
    /// Make it the "Default Device" when picked.
    pub set_default: bool,
    /// Make it the "Default Communications Device" when picked.
    pub set_communications: bool,
}

impl Default for DeviceEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            hotkey: String::new(),
            set_default: true,
            set_communications: false,
        }
    }
}

pub fn dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("SoundHop")
}

pub fn path() -> PathBuf {
    dir().join("config.toml")
}

/// What happened when loading the file.
pub enum LoadResult {
    Loaded(Config),
    /// No file yet: first start.
    Missing,
    /// The file exists but could not be read. It was renamed to
    /// `config.toml.broken` so it is not lost, and defaults are used.
    Broken(String),
}

pub fn load() -> LoadResult {
    let path = path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return LoadResult::Missing,
        Err(e) => return LoadResult::Broken(e.to_string()),
    };
    // `match` on a Result is like a Kotlin `when` over a sealed class.
    match toml::from_str::<Config>(&text) {
        Ok(config) => LoadResult::Loaded(config),
        Err(e) => {
            let _ = std::fs::rename(&path, path.with_extension("toml.broken"));
            LoadResult::Broken(e.to_string())
        }
    }
}

/// Writes the file. It writes to a temporary file first and then renames it, so
/// a crash halfway through never leaves a half-written config behind.
pub fn save(config: &Config) -> Result<(), String> {
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let text = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    let header = "# SoundHop settings. Edited by the settings window; you can also edit it by hand\n\
                  # while SoundHop is closed.\n\n";
    let tmp = path().with_extension("toml.tmp");
    std::fs::write(&tmp, format!("{header}{text}")).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path()).map_err(|e| e.to_string())
}

impl DeviceEntry {
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            "Unnamed device"
        } else {
            self.name.trim()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut c = Config::default();
        c.outputs.push(DeviceEntry {
            id: "{0.0.0.00000000}.{abc}".into(),
            name: "FiiO K5 Pro".into(),
            hotkey: "Ctrl+Alt+1".into(),
            ..Default::default()
        });
        let text = toml::to_string_pretty(&c).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.outputs[0].name, "FiiO K5 Pro");
        assert!(back.outputs[0].set_default);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let c: Config = toml::from_str("[[outputs]]\nid = \"a\"\n").unwrap();
        assert!(c.general.show_notification);
        assert_eq!(c.general.cycle_output_hotkey, "Ctrl+Alt+0");
        assert!(c.outputs[0].set_default);
        assert!(!c.outputs[0].set_communications);
    }
}
