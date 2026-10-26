# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

Released `sound_hop.exe` files are built from this repository's source by GitHub Actions on GitHub-hosted
runners, and signed only after a team member approves each release in SignPath. Nothing is signed from a local
machine.

## Team roles

| Role | Members |
|------|---------|
| Authors (trusted committers) | [Chritol](https://github.com/Chritol) |
| Reviewers (review changes from others) | [Chritol](https://github.com/Chritol) |
| Approvers (approve every signing request) | [Chritol](https://github.com/Chritol) |

All members use multi-factor authentication for GitHub and SignPath.

## Privacy

SoundHop does not transfer any information to other networked systems. It has no network code at all. Its
settings and log stay on your PC in `%APPDATA%\SoundHop\`.

## System changes

SoundHop changes the Windows default audio device only when you pick a device or press one of your hotkeys.
"Start with Windows" adds an entry to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` only when you turn it
on. Uninstall steps are in the [README](../README.md#uninstall).

## Maintainer setup

How the signing is wired up, for the maintainer.

1. SignPath project `sound_hop` with the GitHub.com trusted build system linked, the artifact configuration from
   [`.signpath/artifact-configuration.xml`](../.signpath/artifact-configuration.xml) and a signing policy
   `release-signing` that requires manual approval.
2. Repository settings, **Secrets and variables, Actions**:
   - Secret `SIGNPATH_API_TOKEN`: API token of a SignPath user with submitter permission.
   - Variables `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG` (`sound_hop`) and
     `SIGNPATH_SIGNING_POLICY_SLUG` (`release-signing`).
3. Pushing a `v*` tag builds the exe, sends it to SignPath and waits up to an hour for the approval; the release
   then contains the signed exe. While the variables are not set, releases contain the unsigned exe.
