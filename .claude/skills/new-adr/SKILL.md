---
name: new-adr
description: Create a new Architecture Decision Record in docs/adr/ (MADR format) for SoundHop. Use when a change adds a dependency, a Windows API, changes the config format, user-visible behaviour or scope, or when asked to record a decision.
---

# New ADR

1. List `docs/adr/` and take the next free four-digit number.
2. Copy `docs/adr/template.md` to `docs/adr/NNNN-short-kebab-title.md`.
3. Fill every section:
   - Context: the problem in two or three sentences, with links to issues or PRs.
   - Decision drivers: the measurable constraints (footprint targets, no admin, ...).
   - Considered options: at least two real options, including "do nothing" if it was viable.
   - Decision outcome: the chosen option and why, tied to the drivers; good and bad consequences.
   - Pros and cons of each option that was not chosen.
   Use today's date and status `accepted` if the maintainer already agreed, otherwise `proposed`.
4. If it replaces an earlier decision, set the old ADR's status to `superseded by [NNNN](NNNN-....md)`.
   Do not change anything else in accepted ADRs.
5. Add a row to the table in `docs/adr/README.md`.
6. Mention the ADR in the PR description so the maintainer reviews it with the code.
