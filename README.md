# NotarAI

<div align="center">

<img src="img/NotarAI.png" alt="NotarAI logo" width="300" height="300">

_Intent captured. Drift witnessed._

[![crates.io](https://img.shields.io/crates/v/notarai)](https://crates.io/crates/notarai)
[![CI](https://github.com/davidroeca/NotarAI/actions/workflows/ci.yml/badge.svg)](https://github.com/davidroeca/NotarAI/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

</div>

NotarAI is a continuous intent reconciliation tool that keeps your specs, code, and documentation in sync as all three evolve. It uses LLMs as a bidirectional reconciliation engine, not just to generate code from specs, but to detect drift, surface conflicts, and propose updates across your entire artifact chain.

## The Problem

Before LLMs, code was the single source of truth. Documentation drifted, but it was clear which artifact was authoritative. Now, LLMs generate both code and documentation from natural language prompts, and either artifact can be the one that's "right." Intent lives in chat transcripts that disappear. Code, docs, and intent form a three-body problem where all three can diverge independently.

```mermaid
flowchart TD
    Intent["User Intent"] --> LLM
    Intent -.->|edits directly| Code
    Intent -.->|edits directly| Docs
    LLM -->|generates| Code
    LLM -->|generates| Docs
    Code <-..->|drift| Docs
```

## The Solution

NotarAI introduces a **spec file** -- a structured YAML document governed by a JSON Schema -- that captures user intent as the canonical source of truth. An LLM acts as the reconciliation engine, detecting drift between the spec and its governed artifacts and proposing updates for human review.

```mermaid
flowchart TD
    Intent -->|captured in| Spec
    Spec -->|read by| LLM
    LLM -->|derives| Code
    LLM -->|derives| Docs
    Code -.->|reconcile| Spec
    Docs -.->|reconcile| Spec
```

The spec is version-controlled, diffable, and composable. It serves the same role for LLM-assisted development that a contract serves in an API: a shared agreement about what should exist and why.

## Comparison to Spec-Driven Development (SDD)

Compare similarities and differences with SDD [here](https://davidroeca.github.io/NotarAI/background/comparison-to-sdd/).

### Complementary, not competing

NotarAI can work downstream of any SDD tool. Use Kiro or Spec Kit to generate your initial specs and code, then let NotarAI maintain alignment as the project evolves. The `.notarai/` spec format captures the same intent, behaviors, and constraints that SDD workflows produce. It just keeps watching after the initial generation is done.

## Spec Format

Specs are YAML files validated against a JSON Schema. The format uses progressive disclosure: a small set of required fields for minimum viability, with optional fields for precision as needed.

**Required fields:**

- `schema_version` -- pins the JSON Schema version
- `intent` -- natural language description of what the system should do
- `behaviors` -- structured Given/Then entries describing expected behavior
- `artifacts` -- glob patterns mapping the spec to the files it governs

**Optional fields:**

- `constraints` -- rules the system must follow
- `invariants` -- conditions that must never be violated
- `decisions` -- architectural decision log with date, choice, and rationale
- `open_questions` -- unresolved design questions
- `dependencies` -- references to other specs this one interacts with
- `sync_policy` -- configurable strategy for when and how reconciliation runs
- `notes` -- freeform hints for the LLM about implicit relationships

## How Reconciliation Works

The reconciliation engine detects three scenarios:

1. **Human edits code.** The engine detects that code has drifted from the spec and proposes spec and doc updates.

2. **Human edits spec.** The engine propagates the spec change to code and documentation.

3. **Conflict.** Code says one thing, the spec says another. The engine surfaces the disagreement and the human decides which is correct.

The system is always propose-and-approve, never auto-sync. Both humans and LLMs can edit everything; the spec is the tiebreaker.

## Coverage Model

Every file in the repo falls into one of three tiers:

- **Tier 1 (Full Spec)** -- Business logic, APIs, user-facing features. Full behavioral specification.
- **Tier 2 (Registered)** -- Utilities, config, sidecars. Intent and artifact mapping only, no behaviors required.
- **Tier 3 (Excluded)** -- Generated code, vendor deps, editor configs. Explicitly out of scope.

Files not covered by any tier are flagged as "unspecced" -- a lint warning, not a blocker.

## Getting Started

Spec files live in a `.notarai/` directory at the root of your repository:

```
project/
  .notarai/
    system.spec.yaml
    auth.spec.yaml
    billing.spec.yaml
    _shared/
      security.spec.yaml
  src/
  docs/
```

The top-level `system.spec.yaml` serves as the manifest, referencing subsystem specs via `$ref` and declaring exclusion patterns for Tier 3 files. Cross-cutting concerns (security, logging) are defined once in shared specs and applied across all subsystems.

## CLI Tool

The `notarai` CLI validates spec files against the JSON Schema and integrates with Claude Code via hooks.

### Installation

> [!NOTE]
> This tool is in early development and still needs to be pressure tested with larger repos.

```sh
# Quick install (Linux / macOS)
curl -fsSL https://raw.githubusercontent.com/davidroeca/NotarAI/main/scripts/install.sh | sh

# Or via cargo
cargo install notarai
```

<details>
<summary>Install from source</summary>

```sh
git clone https://github.com/davidroeca/NotarAI
cd NotarAI
cargo build --release
# Binary is at target/release/notarai
```

</details>

### Setup

Run `notarai init` in your project root. This does three things:

1. Adds a PostToolUse hook to `.claude/settings.json` so spec files are automatically validated when Claude Code writes or edits them.
2. Copies the `/notarai-reconcile` slash command to `.claude/commands/` for drift detection.
3. Copies the `/notarai-bootstrap` slash command to `.claude/commands/` for bootstrapping specs from an existing codebase.

```sh
notarai init
```

Running `init` again is safe -- it detects existing configuration and skips.

### Usage

```sh
# Validate all spec files in .notarai/
notarai validate

# Validate a specific file
notarai validate .notarai/auth.spec.yaml

# Validate a directory
notarai validate .notarai/subsystems/
```

Output is `PASS <file>` or `FAIL <file>` with an indented error list. Exit code is 0 if all files pass, 1 if any fail.

### Claude Code Integration

After running `notarai init`, spec files are validated automatically whenever Claude Code writes or edits a file in `.notarai/`. Invalid specs block the tool use with errors on stderr. Non-spec files are ignored silently.

Use the `/notarai-reconcile` slash command to detect drift between specs and code.
Use the `/notarai-bootstrap` slash command to bootstrap `.notarai/` specs for an existing codebase via a structured developer interview.

## Status

This project is in early development. What's implemented:

- Spec schema v0.4 (`notarai.spec.json`) with validation CLI
- `/notarai-reconcile` slash command for drift detection
- `/notarai-bootstrap` slash command for bootstrapping specs from an existing codebase via developer interview

Future goals include:

- Supporting other models and agentic ecosystems beyond Claude Code
- Minimizing token usage in spec-aware workflows
- Richer invariant elicitation and multi-repo support in the bootstrap flow
- Broader reconciliation techniques that don't balloon the context

## Inspirations

NotarAI doesn't come from nowhere. It synthesizes ideas from several established traditions:

- **[Cucumber](https://cucumber.io/) / [Gherkin](https://cucumber.io/docs/gherkin/):** The Given/Then behavior format in NotarAI specs is directly informed by BDD's structured scenario language, but kept in natural language rather than formal Gherkin syntax to lower the authoring barrier.
- **[Terraform](https://www.terraform.io/) and Infrastructure-as-Code:** The reconciliation model (declare desired state, detect drift from actual state, propose a plan to converge) is borrowed from IaC tools like Terraform, [Pulumi](https://www.pulumi.com/), and [CloudFormation](https://aws.amazon.com/cloudformation/). NotarAI's spec is a state file for intent, not infrastructure.
- **[JSON Schema](https://json-schema.org/) / [OpenAPI](https://www.openapis.org/):** The `$ref` composition model and the use of a JSON Schema to govern spec validity come directly from these standards.
- **[Design by Contract](https://en.wikipedia.org/wiki/Design_by_contract) (Eiffel):** The distinction between `constraints` (what the system enforces) and `invariants` (what must never be violated) echoes Eiffel's preconditions, postconditions, and class invariants.
- **[Architecture Decision Records](https://adr.github.io/):** The `decisions` field in the spec is a lightweight ADR log, capturing the _why_ alongside the _what_.

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.
