# 0002: Tech stack: Rust with direct Win32 calls

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

SoundHop must stay resident all day, so footprint matters more than development speed. It needs the Windows
Core Audio APIs, the undocumented `IPolicyConfig` COM interface (see [0003](0003-ipolicyconfig.md)), global
hotkeys, a tray icon and, later, a settings window. The maintainer writes Kotlin and C# daily, has some C++
experience and no Rust experience.

## Decision drivers

- Idle RAM under 10 MB, no CPU use when idle, single small exe, no runtime to install.
- Easy access to COM, including a hand-declared undocumented interface.
- Maintainable by the maintainer and by AI agents; low risk of crashes from memory bugs.
- Simple build and CI for Windows.

## Considered options

1. Rust with the official `windows` crate (raw Win32, no GUI framework)
2. C++ with raw Win32 (or WIL helpers)
3. C# with .NET NativeAOT
4. AutoHotkey v2

## Decision outcome

Chosen option: **Rust with the `windows` crate**, because it matches C++ on footprint while removing the
memory-safety bugs that are the main maintenance risk of a long-running C++ tray app, and Microsoft maintains
the bindings for every Win32 and COM API, including macros to declare our own COM interfaces.

The maintainer's unfamiliarity with Rust is handled by keeping the code close to plain Win32 (which looks the
same in every language), by comments that compare Rust concepts to Kotlin and C#, and by a short guide in
[`docs/architecture.md`](../architecture.md#reading-the-rust-code).

### Consequences

- Good: release exe is about 0.5 MB with no dependencies besides Windows itself. Expected idle RAM is
  a few MB.
- Good: `unsafe` is limited to the Win32 calls; the rest of the logic is checked by the compiler.
- Good: can be cross-compiled from Linux (`x86_64-pc-windows-gnu`) as well as built with MSVC on Windows.
- Bad: steeper learning curve for the maintainer than C#.
- Bad: no GUI designer; the settings window is built in code.

## Pros and cons of the options

The sizes below are typical figures for small tray apps built with each stack, not measurements of SoundHop
except where stated.

### Rust + `windows` crate

- Good: about 0.5 MB exe (measured for this MVP), a few MB idle RAM, no runtime.
- Good: `#[interface]` / `#[implement]` macros declare or implement COM interfaces, so `IPolicyConfig` is
  about 15 lines. Reference counting is automatic (like `ComPtr`).
- Good: memory safety and `Result`-based error handling prevent the typical crash classes.
- Bad: new language for the maintainer; compile times are longer than C++ for a first build.

### C++ / Win32

- Good: smallest possible exe (100-200 KB) and RAM; `IPolicyConfig` is a header with a vtable declaration.
- Good: the maintainer has some C++ experience.
- Bad: manual memory and COM lifetime management; a dangling pointer in a tray app shows up as a random crash
  hours later. WIL/`ComPtr` help but do not remove the risk.
- Bad: build setup (CMake/MSBuild, vcpkg) is heavier than `cargo build`.

### C# with NativeAOT

- Good: the maintainer's strongest language; excellent tooling.
- Good: `[GeneratedComInterface]` supports hand-declared COM interfaces under AOT.
- Bad: WinForms and WPF are not supported by NativeAOT, so the tray and windows would still be raw Win32
  calls through P/Invoke (CsWin32): the same code as in Rust, with more ceremony.
- Bad: the GC heap and runtime put idle RAM around 10-20 MB, which misses the target; exe is several MB.

### AutoHotkey v2

- Good: hotkeys and a tray menu in a few lines; fastest to prototype.
- Bad: `IPolicyConfig` must be called through `ComCall` with a raw vtable index and no type checking; a
  mistake fails silently or crashes.
- Bad: a settings window, device notifications and solid error handling get awkward quickly; weak tooling
  for tests and CI.
- Bad: compiled AHK exes are often flagged by antivirus software, which hurts an open-source download.

## IPolicyConfig across the stacks

Every stack calls the same undocumented COM interface, so the risk that a Windows update breaks it is the same
for all four. The difference is how a break would show up: Rust, C++ and C# declare the interface with types,
so a change is fixed in one declaration; AutoHotkey uses magic vtable indices. See [0003](0003-ipolicyconfig.md).
