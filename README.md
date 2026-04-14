# NotarAI

<div align="center">

<img src="https://raw.githubusercontent.com/davidroeca/NotarAI/main/img/NotarAI.png" alt="NotarAI logo" width="300" height="300">

_Intent captured. Drift reconciled._

[![crates.io](https://img.shields.io/crates/v/notarai)](https://crates.io/crates/notarai)
[![CI](https://github.com/davidroeca/NotarAI/actions/workflows/ci.yml/badge.svg)](https://github.com/davidroeca/NotarAI/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

</div>

NotarAI is a continuous intent reconciliation tool that keeps your specs, code, and documentation in sync as all three evolve. It uses LLMs as a bidirectional reconciliation engine, not just to generate code from specs, but to detect drift, surface conflicts, and propose updates across your entire artifact chain.

## Why NotarAI?

- **Post-generation drift detection.** Most SDD tools help generate code from specs, then stop. NotarAI keeps watching after generation, catching drift as specs, code, and docs evolve independently.
- **Deterministic CI checks.** `notarai check` runs headless, LLM-free drift analysis in under 2 seconds. No API keys, no network access required.
- **Agent-agnostic.** `notarai export-context` produces self-contained reconciliation prompts any LLM can process (Claude, GPT, Gemini, local models). The MCP server speaks a standard protocol.
- **Structured YAML specs with JSON Schema validation.** Machine-readable specs, not Markdown. Enables deterministic tooling (lint, check, scoring) that freeform specs cannot support.
- **Propose-and-approve only.** Never auto-modifies code or specs. Every change is surfaced for human review.
- **Single static binary.** No runtime dependencies. Install and run.

## Getting started

The docsite has detailed [docs for getting started](https://notarai.dev/docs/getting-started/installation).

## Complementary, not competing

Use whatever tools you would like to create your codebase, then let NotarAI track alignment as the project evolves. The `.notarai/` spec format captures the same intent, behaviors, and constraints that SDD workflows produce. NotarAI works downstream of any SDD tool: use Spec Kit, OpenSpec, or Kiro to bootstrap your code, then install NotarAI to watch for drift.

For the full spec format reference, see the [Spec Format guide](https://notarai.dev/docs/guides/spec-format).

## Coverage Model

Every file in the repo falls into one of four tiers:

- **Tier 1 (Full Spec)** -- Business logic, APIs, user-facing features. Full behavioral specification.
- **Tier 2 (Registered)** -- Utilities, config, sidecars. Intent and artifact mapping only, no behaviors required.
- **Tier 3 (Excluded)** -- Generated code, vendor deps, editor configs. Explicitly out of scope.
- **Tier 4 (Derived)** -- Generated outputs tracked for staleness but not authored directly (build artifacts, compiled bundles).

Files not covered by any tier are flagged as "unspecced" -- a lint warning, not a blocker.

## Status

This project is in early development. What's implemented:

- Spec schema v0.8 (`notarai.spec.json`) with validation CLI
- `notarai check` -- deterministic drift detection (coverage gaps, orphaned globs, circular refs, behavior completeness, overlapping coverage, changed files)
- `notarai lint` -- deterministic spec quality linting (11 rules, configurable severity)
- `notarai export-context` -- agent-agnostic reconciliation prompt export
- `/notarai-reconcile` slash command for interactive semantic drift detection
- `/notarai-bootstrap` slash command for bootstrapping specs via developer interview
- BLAKE3+SQLite hash cache (`notarai cache`) to skip unchanged files during reconciliation
- MCP server (`notarai mcp`) with pre-filtered diffs and artifact lists
- GitHub Action for automated PR drift checks
- Self-update (`notarai update`) with install method detection

Future goals include:

- Drift scoring and severity tiers for prioritized remediation
- Decision tracking with audit trail
- Supporting other models and agentic ecosystems beyond Claude Code

## Inspirations

NotarAI draws from Cucumber/Gherkin, IaC tools like Terraform, JSON Schema, Design by Contract, and Architecture Decision Records. See the [Inspirations page](https://notarai.dev/docs/background/inspirations) for details.

## Contributing

See [CONTRIBUTING.md](https://notarai.dev/docs/contributing) for development setup, code style, and PR workflow.

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.
