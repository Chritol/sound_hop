# 0008: MVP scope and what comes next

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

The brief asks for a minimal MVP (hotkeys, tray, switching) with nice-to-haves listed rather than built. The
clarifying questions added: a settings window usable by anyone, separate input switching, an independent
communications device, and graceful handling of disappearing devices.

## Decision outcome

**MVP (0.1.0)**

- Switch output and input devices from the tray and with global hotkeys; cycle hotkeys.
- Manage devices from the tray; communications device submenu.
- On-screen notice; fallback when a device disappears; start with Windows.
- Settings in a hand-editable TOML file.

**Next (planned, own PR)**

- Settings window: manage devices, rename them, edit hotkeys with a hotkey box, communications options,
  general options. Needed because the app should be easy to use for anyone without editing a file.

**Nice-to-haves (not planned yet)** are listed in the README roadmap, e.g. profiles (switch output + input
together), per-app routing, volume hotkeys, per-device tray icons, installer/winget package.

### Consequences

- Good: a usable tool quickly, with the riskiest part (`IPolicyConfig`, notifications, fallback) tested first.
- Bad: until the settings window lands, renaming devices and changing hotkeys means editing the config file.
