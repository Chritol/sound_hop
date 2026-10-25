# 0011: License: GPL-3.0 instead of MIT

- Status: accepted
- Date: 2026-10-25
- Deciders: Chritol

## Context and problem statement

The repository started under MIT. MIT lets anyone take SoundHop, close the source and sell it as their own
product. The maintainer wants everyone to be able to use SoundHop freely, but not to have others sell a closed
copy of it. The license has to be settled before the first release (0.1.0).

## Decision drivers

- Free to use for everyone, including at work.
- No closed, repackaged copies of SoundHop.
- Stay real open source (OSI-approved), so free code signing for open-source projects (for example SignPath
  Foundation) and package managers (winget, Scoop) remain possible.
- Compatible with the dependencies (`windows`, `windows-core`, `serde`, `toml`, `embed-resource`, all
  MIT/Apache-2.0).

## Considered options

1. Keep MIT
2. GPL-3.0
3. A non-commercial license (PolyForm Noncommercial 1.0.0)

## Decision outcome

Chosen option: **GPL-3.0** (`GPL-3.0-only`), because it keeps SoundHop open source while requiring anyone who
distributes it, changed or not, to publish the source under the same license. Selling a closed copy is not
possible, and selling an open copy makes little sense when the original is free.

`-only` rather than `-or-later`: the maintainer owns all the code and can relicense later if needed, without
depending on future license versions.

### Consequences

- Good: SoundHop and every version based on it stay open source.
- Good: Still OSI-approved, so open-source code signing and package managers stay available.
- Bad: Selling the unchanged GPL program is still technically allowed (with source). Accepted, because nobody has
  a reason to pay for what is free on GitHub.
- Bad: Code contributed by others is GPL-3.0 too, so a later license change needs their consent. Contributions
  are accepted under GPL-3.0.
- Neutral: Commits published before this change remain available under MIT to anyone who copied them.

## Pros and cons of the options

### Keep MIT

- Good: Simplest, most permissive.
- Bad: Anyone may close and sell it.

### GPL-3.0

- Good: Open source, prevents closed copies.
- Bad: Does not forbid selling outright.

### PolyForm Noncommercial

- Good: Forbids commercial use and selling.
- Bad: Not open source; also forbids use at work; rules out open-source code signing and makes package
  managers and contributions harder.
