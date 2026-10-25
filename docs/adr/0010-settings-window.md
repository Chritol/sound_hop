# 0010: Settings window built from standard Win32 controls

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

SoundHop should be easy to use for anyone ([0008](0008-mvp-scope.md)). Renaming devices, changing hotkeys and
reordering the cycle list required editing `config.toml`. We need a settings window without giving up the
footprint goals ([0002](0002-tech-stack.md)).

## Decision drivers

- No RAM or CPU cost while the window is closed.
- Native look on Windows 11, keyboard accessible (Tab, Enter, Esc).
- No new dependencies.

## Considered options

1. Standard Win32 controls created in code (tab control, list view with check boxes, hotkey box)
2. A Rust GUI crate (egui, iced, Slint, native-windows-gui)
3. A dialog resource template (`.rc` DIALOGEX)

## Decision outcome

Chosen option: **standard Win32 controls created in code** (`src/settings.rs`).

- Tabs: Output devices, Input devices, General.
- Device tabs: every Windows device plus managed devices that are currently missing ("Not available"). Tick
  to manage, Move up/down for the cycle order, and a "Selected device" box for name, hotkey and the
  default/communications choice. The cycle hotkey sits on the same tab.
- Hotkeys use the Windows hotkey box (`msctls_hotkey32`). Combinations without Ctrl or Alt are turned into
  Ctrl+Alt+key so a single letter can never block typing. A hotkey the box cannot show (e.g. `Win+...` written
  by hand) is kept unless the user changes it.
- Global hotkeys are switched off while the window is open, so pressing a combination sets it instead of
  switching devices. While the window is open, clicking the tray icon brings it to the front instead of
  showing the menu, so the two cannot make conflicting changes.
- Nothing is applied until Save. Save rejects duplicate hotkeys with a message naming both uses.
- First start and starting the exe a second time open the window.
- The tray menu keeps "Manage devices" for quick changes, as requested.

### Consequences

- Good: about 30 KB larger exe, zero cost when closed, native controls and accessibility for free.
- Good: no new dependency.
- Bad: layout is done by hand in code (positions in 96-DPI units, scaled at runtime).
- Bad: the window follows the classic light control theme; a dark-mode window is a nice-to-have.

## Pros and cons of the options

### Rust GUI crate

- Good: easier layout and theming.
- Bad: egui/iced/Slint add 2-10 MB to the exe and often keep a GPU context alive; native-windows-gui is
  unmaintained. All conflict with the footprint goals.

### Dialog resource template

- Good: layout outside the code.
- Bad: the dialog editor is Visual Studio only, and dynamic parts (tabs, list contents) still need code.
