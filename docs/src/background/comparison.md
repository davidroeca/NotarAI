# How NotarAI Compares

This page provides a fair, practical comparison of NotarAI against the major spec-driven development tools available today. For the conceptual positioning of NotarAI within SDD taxonomy, see [Comparison to SDD](./comparison-to-sdd.md).

## At a Glance

| Dimension                | NotarAI                         | Spec Kit            | OpenSpec       | Intent         | Kiro             |
| ------------------------ | ------------------------------- | ------------------- | -------------- | -------------- | ---------------- |
| **Focus**                | Continuous reconciliation       | Generative workflow | Proposal-based | Living specs   | IDE-integrated   |
| **Agent support**        | Claude Code + any via export    | 14+ agents          | 20+ agents     | Proprietary    | Claude only      |
| **Spec format**          | Structured YAML + JSON Schema   | Markdown            | Markdown       | Proprietary    | EARS notation    |
| **CI integration**       | `notarai check` + GitHub Action | Manual              | Manual         | Built-in       | Built-in         |
| **Deterministic checks** | Yes (LLM-free)                  | No                  | No             | Partial        | Partial          |
| **Cost**                 | Free / OSS                      | Free / OSS          | Free / OSS     | $60-200/mo     | Free tier + paid |
| **Brownfield**           | `/notarai-bootstrap` interview  | Manual              | Delta markers  | Context Engine | Limited          |

## Where NotarAI fits

Most SDD tools solve the **cold-start problem**: turning intent into code. NotarAI solves the **entropy problem**: keeping specs, code, and docs aligned as all three evolve independently after the initial generation.

This makes NotarAI complementary to, not competitive with, tools like Spec Kit and OpenSpec. You can use Spec Kit to generate your initial codebase, then install NotarAI to watch for drift as the project evolves.

## Key differentiators

**Post-generation drift detection.** NotarAI keeps watching after the initial generation is done. When code changes but the spec stays the same (or the reverse), NotarAI surfaces the conflict and proposes updates.

**Deterministic CI checks.** `notarai check` runs without network access, API keys, or LLM calls. It completes in under 2 seconds and produces structured JSON for CI consumption. No other SDD tool offers a fully deterministic, headless drift analysis.

**Structured specs with schema validation.** Specs are machine-readable YAML validated against a JSON Schema. This enables deterministic tooling (lint, check, scoring) that Markdown-based specs cannot support.

**Agent-agnostic reconciliation.** `notarai export-context` produces self-contained prompts that any LLM can process. The MCP server speaks a standard protocol. You are not locked into a single agent ecosystem.

**Propose-and-approve only.** NotarAI never auto-modifies code or specs. Every change is surfaced for human review. The spec is the tiebreaker when code and spec disagree, but the human decides what to do about it.

## When to choose NotarAI

NotarAI is a good fit when:

- You already have a codebase and want to add spec coverage incrementally
- You want CI to catch spec drift automatically, without LLM calls
- You care about structured, machine-readable specs rather than freeform Markdown
- You want to use multiple LLM agents without lock-in

NotarAI may not be the right choice when:

- You need a tool that generates entire codebases from specs (use Spec Kit, OpenSpec, or Kiro instead, then add NotarAI for ongoing maintenance)
- Your team prefers freeform Markdown specs without schema constraints
- You need a fully proprietary, managed solution with built-in billing

## Using NotarAI alongside other tools

NotarAI's `.notarai/` spec format captures the same intent, behaviors, and constraints that other SDD tools produce. A typical combined workflow:

1. Use Spec Kit or OpenSpec to bootstrap your initial codebase from specs
2. Run `notarai init` and `notarai-bootstrap` to create `.notarai/` specs from the existing code
3. Use `notarai check` in CI and `/notarai-reconcile` during development
4. Continue using your preferred generation tool for new features; NotarAI watches everything

See [Brownfield Adoption](../guides/brownfield-adoption.md) for a step-by-step guide.
