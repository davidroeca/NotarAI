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

The docsite has detailed [docs for getting started](http://localhost:4321/NotarAI/docs/getting-started/installation/).

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

NotarAI doesn't come from nowhere. It synthesizes ideas from several established traditions:

- **[Cucumber](https://cucumber.io/) / [Gherkin](https://cucumber.io/docs/gherkin/):** The Given/Then behavior format in NotarAI specs is directly informed by BDD's structured scenario language, but kept in natural language rather than formal Gherkin syntax to lower the authoring barrier.
- **[Terraform](https://www.terraform.io/) and Infrastructure-as-Code:** The reconciliation model (declare desired state, detect drift from actual state, propose a plan to converge) is borrowed from IaC tools like Terraform, [Pulumi](https://www.pulumi.com/), and [CloudFormation](https://aws.amazon.com/cloudformation/). NotarAI's spec is a state file for intent, not infrastructure.
- **[JSON Schema](https://json-schema.org/) / [OpenAPI](https://www.openapis.org/):** The `$ref` composition model and the use of a JSON Schema to govern spec validity come directly from these standards.
- **[Design by Contract](https://en.wikipedia.org/wiki/Design_by_contract) (Eiffel):** The distinction between `constraints` (what the system enforces) and `invariants` (what must never be violated) echoes Eiffel's preconditions, postconditions, and class invariants.
- **[Architecture Decision Records](https://adr.github.io/):** The `decisions` field in the spec is a lightweight ADR log, capturing the _why_ alongside the _what_.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and PR workflow.

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.
