# 0003: Use the undocumented IPolicyConfig interface to change the default device

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

Windows has documented APIs to list audio devices and read the default device (`IMMDeviceEnumerator`), but no
documented API to change it. SoundHop's core feature is changing it.

## Decision drivers

- Must work without admin rights.
- Must be instant (< 100 ms).
- Should be what other well-known tools rely on, so a Windows change would be noticed and fixed widely.

## Considered options

1. `IPolicyConfig` (CLSID `870af99c-171d-4f9e-af0d-e63df40c2bc9`, IID `f8679f50-850a-41cf-9c72-430f290290c8`)
2. Scripting the Sound control panel (UI automation)
3. Per-app routing via `IAudioPolicyConfigFactory` (Windows 10+ "App volume and device preferences")

## Decision outcome

Chosen option: **`IPolicyConfig`**. It is what the Windows Sound settings use internally, and what EarTrumpet,
SoundSwitch, NirSoft SoundVolumeView and AudioDeviceCmdlets use. It has been unchanged since Windows 7, needs
no elevation and switches in a few milliseconds.

`SetDefaultEndpoint` is called with `eConsole` and `eMultimedia` for "Default Device", and with
`eCommunications` for "Default Communications Device".

### Consequences

- Good: instant switching, no admin rights, no UI flicker.
- Bad: a future Windows update could change or remove the interface. All Win32 calls to it are in one place
  (`src/audio.rs`, module `policy`), so a fix touches one declaration. Errors are reported in the on-screen
  notice instead of crashing.

## Pros and cons of the options

### UI automation of the Sound control panel

- Good: only documented APIs.
- Bad: slow (opens a window), visible, breaks with every UI redesign and language.

### IAudioPolicyConfigFactory

- Good: per-app routing is a nice feature.
- Bad: also undocumented, changed between Windows versions, and solves a different problem. Listed as a
  nice-to-have in the README.
