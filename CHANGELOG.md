# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Troubleshooting log at `%APPDATA%\SoundHop\soundhop.log` ("Open log file" in the tray menu), and a notice
  when a device has neither "default" nor "communications" ticked.
- Settings window: choose and order devices, rename them, set hotkeys with a hotkey box, pick default /
  communications roles, cycle hotkeys, autostart and notifications. Opens on first start, from the tray menu,
  or by starting the exe again.
- Tray icon with a menu to switch the default output and input device; the current device is ticked and
  unavailable devices are greyed out.
- "Manage devices" submenu to choose which Windows devices SoundHop shows.
- Global hotkeys per device and for cycling outputs and inputs; new devices get `Ctrl+Alt+1..9` (outputs) or
  `Ctrl+Alt+Shift+1..9` (inputs).
- Per-device choice of Default Device and/or Default Communications Device, plus a "Communications device"
  submenu.
- Automatic move to the next available managed device when the active one is unplugged or disabled.
- On-screen notice after a switch, following the Windows light/dark theme.
- Start with Windows option.
- TOML config at `%APPDATA%\SoundHop\config.toml` with safe writes and recovery from broken files.
- ADRs 0001-0009, README, architecture guide, agent instructions, GitHub Actions CI and release workflow.
