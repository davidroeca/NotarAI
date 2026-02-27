# NotarAI

> Intent captured. Drift witnessed.

NotarAI is a continuous intent reconciliation tool that keeps your specs, code, and documentation in sync as all three evolve. It uses LLMs as a bidirectional reconciliation engine: not just to generate code from specs, but to detect drift, surface conflicts, and propose updates across your entire artifact chain.

## What is NotarAI?

**Spec-anchored** -- Structured YAML specs capture intent as the canonical source of truth, validated by JSON Schema.

**Bidirectional** -- Detects drift in any direction (code, spec, or docs) and proposes aligned updates.

**Propose and approve** -- Never auto-syncs. All changes are proposed for human review.

**Composable** -- Specs reference each other via `$ref` for hierarchical and cross-cutting composition.

## Quick links

- [Installation](./getting-started/installation.md)
- [Quick Start](./getting-started/quick-start.md)
- [Spec Format Reference](./guides/spec-format.md)
- [CLI Commands](./reference/cli.md)
