# 0009: Build, CI and releases

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

Development partly happens in Linux containers (AI agents), while the app only runs on Windows. Users should
download a ready exe.

## Decision outcome

- `windows` crate dependencies are Windows-only (`cfg(windows)`), so `cargo test` runs the platform-independent
  unit tests (config, hotkey parsing) on any OS.
- Agents cross-compile with `cargo build --release --target x86_64-pc-windows-gnu` (needs MinGW-w64).
- GitHub Actions on `windows-latest` (MSVC toolchain) runs `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test` and a release build for every push and PR, and uploads the exe as an artifact.
- Pushing a tag `vX.Y.Z` builds the exe and attaches it to a GitHub release.
- Release profile is size-optimised (`opt-level = "s"`, LTO, `panic = "abort"`, stripped).
- The icon, manifest (common controls v6, DPI awareness, `asInvoker`) and version info are embedded by
  `build.rs` with `embed-resource`.

### Consequences

- Good: every change is built and tested on real Windows; downloads need no install.
- Bad: the exe is not code-signed, so SmartScreen warns on first start. Signing is a nice-to-have.
