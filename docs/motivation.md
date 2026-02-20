# NotarAI — Motivation and Concept

## The Problem

With LLMs generating both code and documentation from natural language prompts, there's no authoritative representation of **intent** that persists across changes. Code and docs drift out of sync — and unlike the pre-LLM era where code was the single source of truth, now either artifact can be the one that's "right." This is the three-body problem: intent, code, and docs can all diverge.

## The Idea

Introduce a **NotarAI spec** — a structured YAML document governed by a JSON Schema — that captures user intent as the canonical source of truth. An LLM acts as the reconciliation engine, keeping code and documentation in sync with the spec (and vice versa).

## Spec Format

YAML with progressive disclosure: a small set of required fields for minimum viability, and optional fields for precision as needed.

**Required fields:**
- `schema_version` — pins the JSON Schema version
- `intent` — natural language description of what the system/feature should do
- `behaviors` — structured Given/Then entries (BDD-adjacent, but in natural language)
- `artifacts` — glob patterns mapping the spec to the code and doc files it governs

**Optional fields:**
- `constraints` — rules the system must follow (e.g., "passwords >= 12 chars")
- `invariants` — things that must never be true (e.g., "no plaintext passwords in DB")
- `decisions` — architectural decision log with date, choice, and rationale
- `open_questions` — unresolved design questions
- `dependencies` — references to other specs this one interacts with
- `sync_policy` — configurable strategy for when/how the LLM reconciles
- `notes` — freeform hints for the LLM about implicit relationships

## Coverage Model

Three tiers ensure every file in the repo is accounted for without over-specifying:

- **Tier 1 (Full Spec):** Business logic, APIs, user-facing features — full behaviors and constraints
- **Tier 2 (Registered):** Utility libs, sidecars, config — just intent + artifact mapping, no behaviors
- **Tier 3 (Excluded):** Generated code, vendor deps, editor configs — explicitly out of scope

Anything not covered by any tier is flagged as "unspecced" — a lint warning, not a blocker.

## Composition

Specs compose via `$ref` (borrowed from JSON Schema/OpenAPI):

- `subsystems` — hierarchical references (system → services)
- `applies` — cross-cutting specs (e.g., security, logging) that apply to all subsystems
- A top-level `system.spec.yaml` serves as the manifest, including an `exclude` pattern for Tier 3 files
- Specs will be stored in a `.notarai` directory

## Sync Timing

Three strategies, configurable per spec via `sync_policy`:

- **Spec-first:** LLM updates spec before writing code (high friction, good for contracts)
- **Post-push reconciliation (recommended default):** Code changes land freely; at PR time, a CI hook triggers the LLM to propose spec + doc updates alongside the code diff — all reviewed and merged together
- **Ambient awareness (long-term target):** LLM reads the spec as context before coding (like CLAUDE.md), uses it to guide implementation, then reconciles at push

## Reconciliation Lifecycle

Three scenarios the system handles:

1. **Human edits code** → LLM detects drift → proposes spec + doc updates → human approves
2. **Human edits spec** → LLM propagates changes to code + docs → human reviews as a single PR
3. **Conflict** (code says X, spec says Y) → LLM surfaces the disagreement with options → human decides intent → LLM propagates

Core principle: everyone (human and LLM) can edit everything. The system is propose-and-approve, never auto-sync.

## Bootstrap

For existing codebases: ingest code + docs + commit history → LLM interviews the developer about goals and undocumented rules → draft spec with required fields only → human reviews and enriches → activate sync engine. The spec accrues precision over time.

## Strategic Arc

- **Phase 1:** Open-source spec schema + CLI + CI hooks for software development. Revenue via consulting. Goal: prove the pattern, build community.
- **Phase 2:** Managed sync engine as SaaS (hosted CI bot, drift dashboard, bootstrap wizard). Per-repo/per-seat pricing.
- **Phase 3:** Generalize beyond software — the same spec shape (intent → behaviors → constraints → artifacts) applied to presentations, reports, courses, marketing, and other LLM-mediated creation. The spec becomes a domain-agnostic "version-controlled intent" layer.

## Current Status

The spec schema (`notarai.spec.json`) and CLI validator are implemented. See the [README](../README.md) for installation and usage. Open questions include broadening reconciliation techniques and supporting agentic ecosystems beyond Claude Code.
