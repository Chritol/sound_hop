# 0007: Global hotkeys with RegisterHotKey

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

Every managed device and both cycle actions can have a global hotkey.

## Decision drivers

- Zero cost between key presses.
- No admin rights, no interference with other apps' input.

## Considered options

1. `RegisterHotKey`
2. A low-level keyboard hook (`SetWindowsHookEx(WH_KEYBOARD_LL)`)

## Decision outcome

Chosen option: **`RegisterHotKey`** with `MOD_NOREPEAT`. Windows delivers `WM_HOTKEY` to our hidden window;
nothing runs per key press otherwise.

Defaults: `Ctrl+Alt+0` cycles outputs. When a device is added, it gets the first free `Ctrl+Alt+1..9`
(outputs) or `Ctrl+Alt+Shift+1..9` (inputs). All hotkeys are editable in the config file (later in the settings
window) and can be empty. Every action also works from the tray menu, so hotkeys are optional.

If a hotkey is invalid or already taken by another program, the notice says so instead of failing silently.

### Consequences

- Good: no CPU cost, no hook DLL, no antivirus suspicion.
- Bad: cannot override hotkeys that another program registered first; the user must pick another one.

## Pros and cons of the options

### Low-level keyboard hook

- Good: can capture any key combination.
- Bad: our code runs on every key press system-wide, adds latency if we are slow, looks like a keylogger to
  security tools.
