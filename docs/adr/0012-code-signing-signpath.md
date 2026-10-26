# 0012: Code signing through SignPath Foundation

- Status: accepted
- Date: 2026-10-26
- Deciders: Chris, Claude

## Context and problem statement

The unsigned 0.1.0 exe triggers SmartScreen, and Bitdefender quarantined it while downloading
(`Gen:Suspicious.Cloud`, a reputation check for new, unknown files). [ADR 0009](0009-build-and-release.md) left
signing as a nice-to-have; with real users hitting this, releases should be signed.

## Decision drivers

- Free or nearly free for a hobby project.
- Signing happens in CI from the public source, not on someone's PC, and no private key in the repository.
- Releases keep working before signing is approved.

## Considered options

1. SignPath Foundation (free for OSI-licensed open-source projects)
2. Azure Trusted Signing (about $10 a month, limited eligibility for individuals)
3. Buy an OV certificate (about $200-400 a year, key on a hardware token)

## Decision outcome

Chosen option: **SignPath Foundation**, because it is free for GPL-3.0 projects ([ADR 0011](0011-license-gpl3.md))
and signs directly from GitHub Actions with origin verification.

- The `release` job sends the exe uploaded by the `build` job to SignPath with
  `signpath/github-action-submit-signing-request` and publishes the signed exe.
- Every signing request needs a manual approval in SignPath; the job waits up to an hour.
- The step only runs when the repository variable `SIGNPATH_ORGANIZATION_ID` is set, so releases fall back to
  the unsigned exe until the project is accepted.
- The required [code signing policy](../code-signing.md) is linked from the README and every release.

### Consequences

- Good: signed releases, fewer antivirus and SmartScreen warnings; reputation builds on one certificate.
- Good: no secrets in the repository besides a SignPath API token in the repository secrets.
- Bad: acceptance is up to SignPath Foundation; the certificate is issued to SignPath Foundation, not to the
  maintainer, and the project must keep following their rules (OSI license, MFA, policy page, approvals).
- Bad: a new third-party GitHub Action in the release job.
- Neutral: signing does not remove SmartScreen warnings instantly; reputation still builds with downloads.

## Pros and cons of the options

### SignPath Foundation

- Good: free, CI-integrated, no key handling.
- Bad: application and approval process; rules to follow.

### Azure Trusted Signing

- Good: Microsoft-run, fast reputation.
- Bad: monthly cost, identity validation, eligibility limits for individuals.

### OV certificate

- Good: certificate in the maintainer's name.
- Bad: costly, hardware token, hard to use from CI.
