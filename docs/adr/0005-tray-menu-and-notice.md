# 0005: Native tray menu and a self-drawn on-screen notice

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

The user needs one-click switching from the tray and short feedback after a hotkey switch, while keeping the
app as light as possible.

## Decision drivers

- Zero cost when idle; no extra processes.
- Feedback must be quick and must not pile up in the notification center.
- Must not steal keyboard focus (games, DAWs).

## Considered options

1. Native Win32 popup menu + a small self-drawn popup window for the notice
2. Windows toast notifications (WinRT `ToastNotificationManager`)
3. Tray balloon notifications (`Shell_NotifyIcon` with `NIF_INFO`)

## Decision outcome

Chosen option: **native popup menu and a self-drawn notice window**.

- One tray icon. Left or right click opens the menu: managed outputs and inputs (current one ticked,
  unavailable ones greyed out), a "Communications device" submenu, "Manage devices" (every Windows device,
  tick to manage), config, notifications and autostart toggles, exit.
- The notice is a small borderless, top-most, non-activating window at the bottom centre of the screen with
  the mouse, using Windows 11 rounded corners and the system light/dark theme. It is created when needed and
  destroyed after `notification_ms` (default 1.5 s), so it uses no resources while hidden.

### Consequences

- Good: no WinRT activation, no AppUserModelID or Start menu shortcut needed, nothing in the notification
  center, no focus stealing.
- Bad: we draw the notice ourselves (about 200 lines of GDI).

## Pros and cons of the options

### Toast notifications

- Good: native look.
- Bad: slow to appear, stack up in the notification center, need an AppUserModelID registration, and pull in
  WinRT. Silenced by Focus Assist.

### Balloon notifications

- Good: one API call.
- Bad: shown as toasts on Windows 10/11 with the same drawbacks.
