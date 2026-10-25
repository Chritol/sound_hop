<p align="center"><img src="res/sound_hop.png" width="96" alt="SoundHop icon"></p>

# SoundHop

Switch your Windows audio devices with one hotkey or one click, instead of digging through the sound settings.

SoundHop sits in the tray, uses a few MB of RAM and no CPU while idle, and switches the default playback or
recording device instantly. It is a single ~0.5 MB exe with no installer, no runtime and no admin rights.

> Status: early version (0.1.0). Feedback welcome.

## Features

- **One click**: click the tray icon and pick a device. The current one is ticked.
- **Global hotkeys**: one per device, plus one that cycles through your devices. Defaults: `Ctrl+Alt+1`,
  `Ctrl+Alt+2`, ... for outputs in the order you add them, `Ctrl+Alt+0` to cycle outputs,
  `Ctrl+Alt+Shift+1`, ... for inputs.
- **Only your devices**: you choose which devices SoundHop shows. Everything else stays hidden.
- **Outputs and inputs** are switched separately.
- **Communications device** is independent: choose per device whether it also becomes the "Default
  Communications Device" (used by Teams, Discord, ...), or set it from the tray submenu.
- **Unplugged devices are handled**: a device that disappears is shown as "not available" and SoundHop moves to
  the next available device in your list. When it comes back you can pick it again.
- **Short on-screen notice** after each switch (can be turned off). It never steals focus.
- **Start with Windows** option.
- **Settings window** for everything: devices, names, hotkeys, cycle order, options.

## Install

1. Download `sound_hop.exe` from the [latest release](https://github.com/Chritol/sound_hop/releases).
2. Put it somewhere permanent, e.g. `%LOCALAPPDATA%\Programs\SoundHop\`.
3. Run it. Windows SmartScreen may warn because the exe is not signed: "More info" then "Run anyway".
4. Click the tray icon (you may need to drag it out of the hidden icons area to keep it visible).

## Usage

1. On first start the settings window opens. Tick the devices you want on the **Output devices** and **Input
   devices** tabs. Each newly ticked device gets the next free hotkey. Rename them, change hotkeys or the
   cycle order there, then **Save**.
2. Pick a device from the tray menu, or press its hotkey.
3. Optional: **Start with Windows** on the **General** tab or in the tray menu.

Quick changes without the window: tray menu, **Manage devices**. To open the settings again: tray menu,
**Settings...**, or start `sound_hop.exe` again.

## Configuration

The settings window covers everything. For the curious, settings live in `%APPDATA%\SoundHop\config.toml`
and can also be edited by hand: **Edit config file** in the tray menu, save, then **Reload config file**
(changes are also picked up the next time the menu opens).

```toml
[general]
show_notification = true      # on-screen notice after a switch
notification_ms = 1500        # how long it stays
cycle_output_hotkey = "Ctrl+Alt+0"
cycle_input_hotkey = ""       # empty = no hotkey

[[outputs]]                   # in cycle order
id = "{0.0.0.00000000}.{...}" # Windows endpoint ID, filled in by "Manage devices"
name = "FiiO K5 Pro"          # shown in the menu and notice; rename freely
hotkey = "Ctrl+Alt+1"
set_default = true            # becomes the "Default Device"
set_communications = false    # also becomes the "Default Communications Device"

[[outputs]]
id = "{0.0.0.00000000}.{...}"
name = "Loudspeakers"
hotkey = "Ctrl+Alt+3"

[[inputs]]
id = "{0.0.1.00000000}.{...}"
name = "Scarlett 2i2"
hotkey = "Ctrl+Alt+Shift+1"
set_communications = true
```

Hotkey format: modifiers `Ctrl`, `Alt`, `Shift`, `Win` joined with `+`, then a key: `A`-`Z`, `0`-`9`,
`F1`-`F24`, `Num0`-`Num9`, `NumAdd`, `Space`, `Home`, `PageUp`, `MediaPlay`, `VolumeUp`, ... (see
[`src/hotkey.rs`](src/hotkey.rs)).

If the file has an error, SoundHop tells you, keeps it as `config.toml.broken` and starts with defaults.

## Troubleshooting

SoundHop writes a small log to `%APPDATA%\SoundHop\soundhop.log` (tray menu: **Open log file**). It lists
your devices, every hotkey and menu action, and what Windows reported after each switch. Attach it when
reporting a problem. It is capped at 256 KB.

## Build from source

```sh
# On Windows (MSVC toolchain)
cargo build --release              # target\release\sound_hop.exe

# Cross-compile from Linux (needs mingw-w64)
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu

cargo test                         # unit tests, runs on any OS
```

See [`docs/architecture.md`](docs/architecture.md) for how it works and
[`docs/adr/`](docs/adr/README.md) for why it is built this way.

## Roadmap

Nice-to-haves (not planned yet):

- Profiles: switch output and input together (e.g. "Recording" = Scarlett out + Scarlett in), with a hotkey.
- Tray icon that shows which device is active.
- Volume and mute hotkeys for the current device.
- Per-app output routing.
- Dark-mode settings window.
- Signed exe, winget / Scoop package.

## How it changes the default device

Windows has no public API for this. SoundHop uses the same undocumented `IPolicyConfig` interface as the
Windows sound settings, EarTrumpet and SoundSwitch. It has been stable since Windows 7, but a future Windows
update could break it. Details in [ADR 0003](docs/adr/0003-ipolicyconfig.md).

## License

Copyright (c) 2026 Chritol. SoundHop is free software under the [GNU GPL v3](LICENSE): you may use, change and
share it, also commercially, but anything you distribute that is based on it must be released under the same
license with its source code. Reasons in [ADR 0011](docs/adr/0011-license-gpl3.md).
