# 0004: TOML settings file in %APPDATA%

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

SoundHop stores managed devices, hotkeys and options. In the MVP the file is edited by hand; later the settings
window ([0008](0008-mvp-scope.md)) writes it.

## Decision drivers

- Readable and editable by hand, with comments.
- Robust: a typo must not lose the user's settings.
- No admin rights, per-user.

## Considered options

1. TOML
2. JSON
3. Registry

## Decision outcome

Chosen option: **TOML at `%APPDATA%\SoundHop\config.toml`**, read and written with `serde` + `toml`.

- Every field has a default (`#[serde(default)]`), so partial files work.
- Devices are identified by their Windows endpoint ID, which stays the same across unplug/replug and reboots,
  plus a display name the user can change.
- Writes go to a temp file that is then renamed, so a crash never leaves a half-written file.
- If the file cannot be parsed it is renamed to `config.toml.broken` and the user is told, instead of
  silently overwriting it.
- The app reloads the file when the tray menu opens and its modification time changed, and via
  "Reload config file". No file watcher thread.

### Consequences

- Good: human-friendly, supports comments, standard in the Rust world.
- Bad: comments written by hand are lost when SoundHop rewrites the file (e.g. after adding a device).

## Pros and cons of the options

### JSON

- Good: universally known.
- Bad: no comments, trailing-comma errors are easy to make by hand.

### Registry

- Good: no file handling.
- Bad: hard to edit, back up or share; not portable.
