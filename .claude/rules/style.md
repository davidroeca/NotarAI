---
paths:
  - 'src/**/*.rs'
  - 'docs/**/*.md'
  - '.notarai/**/*.spec.yaml'
  - 'CLAUDE.md'
  - 'README.md'
  - 'CONTRIBUTING.md'
---

## Style

- **American English** throughout -- variable names, function names, type names, comments, docs, and specs (e.g. "initialize" not "initialise", "color" not "colour").
- **QWERTY-typable characters in code and comments** -- no em-dashes, curly quotes, or other characters requiring special input. Use `--` for em-dash, `->` for arrows, straight quotes. Non-typable characters are fine in user-facing printed strings where they aid readability.

> Also enforced by `style.spec.yaml` for retroactive drift detection. See the [Specs vs Claude Rules](../../../docs/src/guides/specs-vs-claude-rules.md) guide for why both exist.
