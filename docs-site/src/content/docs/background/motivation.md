---
title: Motivation
description: Why NotarAI exists and the problem it solves.
---

## The problem

With LLMs generating both code and documentation from natural language prompts, there's no authoritative representation of **intent** that persists across changes. Code and docs drift out of sync — and unlike the pre-LLM era where code was the single source of truth, now either artifact can be the one that's "right." This is the three-body problem: intent, code, and docs can all diverge.

## The idea

Introduce a **NotarAI spec** — a structured YAML document governed by a JSON Schema — that captures user intent as the canonical source of truth. An LLM acts as the reconciliation engine, keeping code and documentation in sync with the spec (and vice versa).

## Coverage model

Three tiers ensure every file in the repo is accounted for without over-specifying:

- **Tier 1 (Full Spec):** Business logic, APIs, user-facing features — full behaviors and constraints
- **Tier 2 (Registered):** Utility libs, sidecars, config — just intent + artifact mapping, no behaviors
- **Tier 3 (Excluded):** Generated code, vendor deps, editor configs — explicitly out of scope

Anything not covered by any tier is flagged as "unspecced" — a lint warning, not a blocker.

## Sync timing

Three strategies, configurable per spec via `sync_policy`:

- **Spec-first:** LLM updates spec before writing code (high friction, good for contracts)
- **Post-push reconciliation (recommended default):** Code changes land freely; at PR time, a CI hook triggers the LLM to propose spec + doc updates alongside the code diff — all reviewed and merged together
- **Ambient awareness (long-term target):** LLM reads the spec as context before coding (like CLAUDE.md), uses it to guide implementation, then reconciles at push

## Bootstrap

For existing codebases: ingest code + docs + commit history, then the LLM interviews the developer about goals and undocumented rules, drafts a spec with required fields only, and the user reviews and enriches. The spec accrues precision over time.

## Inspirations

See the [Inspirations](/NotarAI/docs/background/inspirations/) page.
