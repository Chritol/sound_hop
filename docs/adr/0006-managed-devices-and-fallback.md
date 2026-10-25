# 0006: Explicitly managed devices, independent roles, and fallback when a device disappears

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

Windows lists many endpoints (monitors with speakers, virtual devices, unused inputs). The user only wants a
few in SoundHop. Devices get switched off or unplugged, and the "Default Communications Device" is used
differently by different people.

## Decision drivers

- Only show what the user chose.
- Never crash or get stuck when a device disappears.
- Communications device must be controllable separately (e.g. calls always on headphones).

## Considered options

For device selection: (a) show all active devices, (b) explicit list chosen by the user.
For the communications role: (a) always follow the default device, (b) per-device setting plus a separate
menu.
For disappearing devices: (a) let Windows pick, (b) move to the next managed device that is available.

## Decision outcome

- **Explicit list**: outputs and inputs are separate lists in the config, in cycle order. Devices are added
  and removed from "Manage devices" in the tray menu.
- **Independent communications role**: each managed device has `set_default` (default `true`) and
  `set_communications` (default `false`). The "Communications device" submenu sets only that role.
- **Fallback**: SoundHop remembers which device it last saw in each role. When Windows reports a device change
  (via `IMMNotificationClient`, debounced by 300 ms) and that device is a managed one that is no longer
  available, SoundHop switches that role to the next available managed device in list order. If none is
  available, Windows' own choice stays. The missing device stays in the list, greyed out as "not available",
  and can be selected again as soon as Windows reports it back.
- Separate output and input switching, each with its own hotkeys and cycle hotkey.

### Consequences

- Good: menus stay short; unplugging the FiiO moves you to the next device you actually use.
- Good: no polling; reacts only to Windows notifications.
- Bad: devices must be added once by the user.
