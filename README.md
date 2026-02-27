# NotarAI

<div align="center">

<img src="img/NotarAI.png" alt="NotarAI logo" width="300" height="300">

_Intent captured. Drift witnessed._

[![crates.io](https://img.shields.io/crates/v/notarai)](https://crates.io/crates/notarai)
[![CI](https://github.com/davidroeca/NotarAI/actions/workflows/ci.yml/badge.svg)](https://github.com/davidroeca/NotarAI/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

</div>

NotarAI is a continuous intent reconciliation tool that keeps your specs, code, and documentation in sync as all three evolve. It uses LLMs as a bidirectional reconciliation engine, not just to generate code from specs, but to detect drift, surface conflicts, and propose updates across your entire artifact chain.

## Getting started

The docsite has detailed [docs for getting started](https://davidroeca.github.io/NotarAI/docs/getting-started/installation/).

## Spec: complementary, not competing

NotarAI can work downstream of any SDD tool. Use Kiro or Spec Kit to generate your initial specs and code, then let NotarAI maintain alignment as the project evolves. The `.notarai/` spec format captures the same intent, behaviors, and constraints that SDD workflows produce. It just keeps watching after the initial generation is done.

For the full reference, check [here](https://davidroeca.github.io/NotarAI/docs/guides/spec-format/).

## Coverage Model

Every file in the repo falls into one of three tiers:

- **Tier 1 (Full Spec)** -- Business logic, APIs, user-facing features. Full behavioral specification.
- **Tier 2 (Registered)** -- Utilities, config, sidecars. Intent and artifact mapping only, no behaviors required.
- **Tier 3 (Excluded)** -- Generated code, vendor deps, editor configs. Explicitly out of scope.

Files not covered by any tier are flagged as "unspecced" -- a lint warning, not a blocker.

## Status

This project is in early development. What's implemented:

- Spec schema v0.4 (`notarai.spec.json`) with validation CLI
- `/notarai-reconcile` slash command for drift detection
- `/notarai-bootstrap` slash command for bootstrapping specs from an existing codebase via developer interview
- BLAKE3+SQLite hash cache (`notarai cache`) to skip unchanged files during reconciliation
- MCP server (`notarai mcp`) that serves pre-filtered diffs and artifact lists, keeping reconciliation context proportional to what changed

Future goals include:

- Supporting other models and agentic ecosystems beyond Claude Code
- Richer invariant elicitation and multi-repo support in the bootstrap flow

## Inspirations

NotarAI draws from Cucumber/Gherkin, IaC tools like Terraform, JSON Schema, Design by Contract, and Architecture Decision Records. See the [Inspirations page](https://davidroeca.github.io/NotarAI/docs/background/inspirations/) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and PR workflow.

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.
