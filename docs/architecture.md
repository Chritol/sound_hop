# Architecture

SoundHop is one Rust binary with no GUI framework. It talks to Windows directly through Microsoft's
[`windows`](https://crates.io/crates/windows) crate. Decisions behind this are in [`adr/`](adr/README.md).

## Modules

| File | Responsibility |
|------|----------------|
| `src/main.rs` | Entry point; shows an error box if startup fails. |
| `src/app.rs` | Hidden main window, message loop, tray icon and menu, hotkeys, switching and fallback logic. |
| `src/audio.rs` | Core Audio: list devices, read/set the default device (`IPolicyConfig`), device change notifications. |
| `src/config.rs` | The TOML settings file: types, load, safe save. Platform independent, unit tested. |
| `src/hotkey.rs` | `"Ctrl+Alt+1"` text to/from Windows key codes. Platform independent, unit tested. |
| `src/settings.rs` | The settings window (tabs, device lists, hotkey boxes); exists only while open. |
| `src/osd.rs` | The on-screen notice window (created on demand, self-destroys). |
| `src/log.rs` | Troubleshooting log (`log!` macro), size-capped, written line by line. |
| `src/autostart.rs` | "Start with Windows" via `HKCU\...\Run`. |
| `src/ui.rs` | Small helpers: UTF-16 strings, DPI scaling, message boxes, theme detection. |
| `build.rs`, `res/` | Embeds icon, manifest and version info into the exe. |

## How it runs

```
            ┌────────────── UI thread (the only thread we own) ───────────────┐
 hotkey ──► │ WM_HOTKEY ─┐                                                    │
 tray   ──► │ WM_TRAY ───┼─► wndproc (app.rs) ─► App::perform ─► audio.rs ────┼─► IPolicyConfig
 device ──► │ WM_DEVICES_CHANGED ─► 300 ms timer ─► App::on_devices_changed   │   (Windows audio)
 change     │                                         │                       │
 (Windows   │                                         └─► osd::show (notice)  │
  thread)   └─────────────────────────────────────────────────────────────────┘
```

- `app::run` creates a hidden window and sits in `GetMessageW`. Windows only wakes the thread when a message
  arrives, so idle CPU is zero and there is no polling.
- **Hotkeys**: `RegisterHotKey` per configured hotkey. Windows sends `WM_HOTKEY` with the ID; `App.hotkeys`
  maps ID to an `Action`.
- **Tray**: `Shell_NotifyIconW` with `NOTIFYICON_VERSION_4`. A click sends `WM_TRAY`; `show_menu` builds a
  fresh popup menu each time (so it always reflects the current devices) and runs the chosen `Action`.
- **Device changes**: `audio::DeviceWatcher` implements `IMMNotificationClient`. Windows calls it on its own
  thread, so it only posts `WM_DEVICES_CHANGED` to the window. The UI thread waits 300 ms for the burst of
  notifications to settle, then runs `on_devices_changed`.
- **Fallback** (`on_devices_changed`): for each flow (output/input) and role (default/communications),
  if the device SoundHop last saw in that role is a managed device that is no longer available, switch to the
  next available managed device in list order. Otherwise remember Windows' current device.
- **Switching**: `Audio::set_default` calls `IPolicyConfig::SetDefaultEndpoint` with `eConsole` +
  `eMultimedia` (Default Device) or `eCommunications`.
- **Config**: loaded at start; reloaded when the menu opens and the file's modification time changed, or via
  "Reload config file". Every change made from the menu is saved immediately.
- **Settings window**: `settings::open` receives a snapshot (config, devices, current defaults) and edits it in
  memory. Hotkeys are unregistered while it is open and the tray icon only brings it forward. On close it posts
  `WM_SETTINGS_CLOSED` (with "saved" in `wParam`); the app reloads the config and re-registers hotkeys. The
  message loop passes messages through `IsDialogMessageW` for Tab/Enter/Esc.
- **Single instance**: a named mutex. A second start posts a message to the first, which opens the settings.

## Error handling

- Nothing in normal operation panics. Win32 failures return `Result` and are shown in the notice
  ("Could not switch", "Hotkey problem") or a message box (config file problems).
- A broken config file is renamed to `config.toml.broken` before defaults are used.
- `panic = "abort"` in release: if a real bug panics, the process ends instead of continuing in a bad state.

## Reading the Rust code

A short guide for Kotlin and C# developers. Most of the code is Win32 calls, which read the same in any
language; these are the Rust-specific parts.

| Rust | Kotlin / C# equivalent |
|------|------------------------|
| `Option<T>` (`Some(x)` / `None`) | `T?` / `Nullable<T>`. `opt?` in a function returning `Option` is like `?: return null`. |
| `Result<T, E>` (`Ok` / `Err`) | Checked exceptions as values. `expr?` returns the error to the caller, like re-throwing. |
| `match x { ... }` | `when (x)` / `switch` expression, but the compiler forces you to cover every case. |
| `enum Action { Select(Flow, usize), ... }` | Kotlin `sealed class` with data-class subclasses. |
| `struct` + `impl` | A class: data in `struct`, methods in `impl`. |
| `trait` | `interface`. `impl Trait for Type` = "Type implements Trait". |
| `&x` / `&mut x` | Pass by reference (read-only / mutable). The compiler checks no one else changes `x` meanwhile. |
| Ownership, `.clone()` | Values have one owner; `clone()` makes an explicit copy where both sides need one. |
| `Drop` | `IDisposable.Dispose()` / `use {}`, but called automatically when the value goes out of scope. |
| `thread_local!` + `RefCell` | A field only the UI thread touches. `RefCell` checks at runtime that it is not borrowed twice. |
| `unsafe { ... }` | Code the compiler cannot check, here only Win32/COM calls. Like `unsafe` in C# with pointers. |
| `#[derive(Serialize, Deserialize)]` | `@Serializable` (kotlinx) / `System.Text.Json` attributes. |
| `#[implement(IFoo)]` / `#[interface("guid")]` | Implementing / declaring a COM interface, like `[ComVisible]` / `[ComImport, Guid(...)]`. |
| `w!("text")` | A compile-time UTF-16 string literal for Win32 (`LPCWSTR`). |
| `cargo build` / `cargo test` | `./gradlew build` / `dotnet test`. `Cargo.toml` is the `build.gradle` / `.csproj`. |

## Footprint budget

| Metric | Target | Notes |
|--------|--------|-------|
| Exe size | < 1 MB | About 0.55 MB for 0.1.0 (MinGW build). |
| Idle RAM | < 10 MB | To be measured on Windows 11 (Task Manager, "Memory (active private working set)"). |
| Idle CPU | 0 % | No timers or polling while idle; only Windows messages wake the thread. |
| Hotkey to switch | < 100 ms | `SetDefaultEndpoint` is synchronous and fast. |
