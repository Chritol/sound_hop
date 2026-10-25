# AGENTS.md

Instructions for AI coding agents (Claude Code, GitHub Copilot, others) and human contributors.

## Project

SoundHop is a tiny Windows 11 tray app that switches the default audio output/input device with global hotkeys
or one click. Rust, raw Win32 via the `windows` crate, no GUI framework. Read
[`docs/architecture.md`](docs/architecture.md) before changing code and [`docs/adr/`](docs/adr/README.md) to
understand why things are built this way.

## Commands

```sh
cargo test                                               # unit tests (config, hotkeys); runs on any OS
cargo fmt                                                # format (max width 120, see rustfmt.toml)
cargo clippy --all-targets -- -D warnings                # lint; CI fails on any warning
cargo build --release                                    # on Windows -> target/release/sound_hop.exe
cargo build --release --target x86_64-pc-windows-gnu     # cross-compile from Linux (needs mingw-w64)
cargo clippy --release --target x86_64-pc-windows-gnu    # lint the Windows code from Linux
```

On Linux, plain `cargo build`/`clippy` only check the platform-independent modules; always also run the
`--target x86_64-pc-windows-gnu` variants. The app cannot be run in a Linux container: say so in the PR and ask
the maintainer to test on Windows.

## Rules

1. **Every significant decision gets an ADR** in `docs/adr/` (MADR format, next free number, copy
   `template.md`, add it to `docs/adr/README.md`). Significant = new dependency, new Windows API, config format
   change, user-visible behaviour that is hard to change later, scope changes. Accepted ADRs are not rewritten;
   supersede them with a new one.
2. **Keep docs current in the same PR**: `CHANGELOG.md` (`[Unreleased]` section) for user-visible changes,
   `README.md` for usage/config changes, `docs/architecture.md` for structural changes.
3. **Footprint is a feature**: no polling loops, timers that run while idle, background threads, or heavy
   dependencies. Target: idle RAM < 10 MB, idle CPU 0 %, exe < 1 MB. Justify any new crate in an ADR.
4. **No admin rights, no services, no installers required.**
5. **Never crash on device changes**: devices can disappear at any time. Handle `Err` from Win32/COM by
   showing a notice or skipping, never `unwrap()` on Windows calls.
6. **Keep the MVP minimal**: new ideas go to the README roadmap ("Nice-to-haves") unless the maintainer asked
   for them.
7. **Explain Rust for the maintainer**: the maintainer writes Kotlin/C# and is new to Rust. Prefer plain,
   explicit code over clever idioms, and add a short comment when a Rust-specific construct is not obvious
   (see the table in `docs/architecture.md`).

## Conventions

- `unsafe` only around Win32/COM calls, as small as practical.
- Win32 string conversion through `ui::wide` / `ui::from_wide_ptr`; sizes in 96-DPI units through
  `ui::scale`.
- UI state lives in `thread_local!` + `RefCell` and is only touched on the UI thread through `with_app`.
  Never hold the borrow across a modal call (`TrackPopupMenuEx`, `MessageBoxW`); Windows re-enters `wndproc`
  during those.
- Callbacks from Windows threads (e.g. `IMMNotificationClient`) only `PostMessageW` to the main window.
- Everything the user sees is plain English, short, and says what to do next.
- Commits: imperative summary line (`Add cycle hotkey for inputs`), body explains why.

## Layout

```
src/            app.rs audio.rs config.rs hotkey.rs osd.rs autostart.rs ui.rs main.rs
res/            icon, manifest, .rc (embedded by build.rs)
docs/           architecture.md, adr/
.github/        CI (build, test, release on tag v*), copilot-instructions.md
.claude/skills/ new-adr skill
```
