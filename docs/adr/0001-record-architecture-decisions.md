# 0001: Record architecture decisions

- Status: accepted
- Date: 2026-10-25
- Deciders: Chris, Claude

## Context and problem statement

SoundHop is built with AI coding agents and reviewed by one maintainer. Agents start every session without
memory, and humans forget why things are the way they are. We need a lightweight, durable record of why each
significant choice was made.

## Decision drivers

- Agents and new contributors must be able to find the reasoning without asking.
- Low ceremony: writing a record should take minutes.

## Considered options

1. No records, reasoning lives in PR descriptions.
2. Architecture Decision Records in the repo, using [MADR](https://adr.github.io/madr/).
3. A wiki page.

## Decision outcome

Chosen option: **ADRs in `docs/adr/` using MADR**, because they live next to the code, are reviewed in the
same PR as the change, and every agent reads the repo anyway.

Rules:

- Every significant decision gets an ADR: new dependency, new Windows API, file format, UX behaviour that is
  hard to change later, scope cuts.
- Files are numbered `NNNN-kebab-title.md`; copy [`template.md`](template.md).
- ADRs are not edited after acceptance except for status. A changed decision gets a new ADR that supersedes
  the old one.
- [`README.md`](README.md) in this folder lists all ADRs.

### Consequences

- Good: the "why" survives across sessions, agents and contributors.
- Bad: small extra step in PRs that change architecture.

## Pros and cons of the options

### No records

- Good: zero effort.
- Bad: PR descriptions are hard to find later and agents do not read them.

### Wiki

- Good: easy to edit.
- Bad: not versioned with the code, not reviewed, invisible to agents working in the repo.
