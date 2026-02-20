# How NotarAI Differs from Spec-Driven Development

Spec-driven development (SDD) has emerged as a major pattern for AI-assisted coding, but the term covers several distinct approaches. [Birgitta Böckeler's taxonomy](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html) identifies three levels:

- **Spec-first:** Write a spec, generate code, discard or ignore the spec afterward.
- **Spec-anchored:** Keep the spec around for ongoing maintenance, but how it stays current is left vague.
- **Spec-as-source:** The spec *replaces* code as the primary artifact. Humans never touch code directly.

Most SDD tools ([Kiro](https://kiro.dev/), [Spec Kit](https://github.com/github/spec-kit), [OpenSpec](https://github.com/Fission-AI/OpenSpec)) are spec-first in practice: they help you go from intent → plan → tasks → code, but once the code exists, the spec quietly goes stale. [Tessl](https://docs.tessl.io/) is exploring spec-as-source, where code is generated from specs and marked "DO NOT EDIT", but this sacrifices the flexibility of direct code editing.

**NotarAI occupies the gap that Böckeler's taxonomy identifies but no current tool fills: spec-anchored with automated maintenance.** The spec persists for the lifetime of the feature, and an LLM reconciliation engine actively keeps it aligned with code and docs as all three evolve.

## SDD tools solve the cold-start problem. NotarAI solves the entropy problem.

SDD tools help you *write* specs. NotarAI helps you *keep them true*.

- A developer adds a feature → NotarAI detects the spec doesn't account for it and proposes an update
- A team lead updates the spec → NotarAI propagates the change to code and docs
- Code contradicts a spec constraint → NotarAI flags the conflict and asks the human to decide

The spec isn't just a blueprint. It's a **witness**, a living contract the LLM continuously verifies against reality.

## Landscape comparison

| Tool | SDD Level | Direction | Spec Lifespan | Brownfield Support |
|---|---|---|---|---|
| [**Kiro**](https://kiro.dev/) | Spec-first | Spec → code | Change request | Limited |
| [**Spec Kit**](https://github.com/github/spec-kit) | Spec-first (aspires to anchored) | Spec → code | Branch / change request | Limited |
| [**Tessl**](https://docs.tessl.io/) | Spec-as-source | Spec → code (human edits spec only) | Feature lifetime | Reverse-engineering CLI |
| [**OpenSpec**](https://github.com/Fission-AI/OpenSpec) | Spec-first | Spec → code | Change request | Limited |
| [**Semcheck**](https://semcheck.ai/) | Compliance checking | Spec → code (one-way check) | Ongoing | Yes |
| **NotarAI** | Spec-anchored + active reconciliation | Spec → code, code → spec, spec → docs, docs → spec | Feature lifetime | Bootstrap flow with LLM interview |

## Core Concepts

### The spec as a living contract

A NotarAI spec (`.notarai/*.spec.yaml`) captures **intent** in a structured, versionable format: what the system should do, what behaviors it exhibits, what constraints it must follow, and which files it governs. The spec sits between human intent and machine-generated artifacts, serving as the canonical source of truth that both humans and LLMs can read, write, and reason about.

### Bidirectional reconciliation

Everyone can edit everything: humans edit code, LLMs edit code, humans edit specs, LLMs propose spec updates. NotarAI's reconciliation engine detects when any artifact drifts from the others and proposes aligned updates. The system is always **propose-and-approve**, never auto-sync.

### Three-tier coverage

Not every file needs a behavioral spec, but nothing should be invisible:

- **Tier 1 (Full Spec):** Business logic, APIs, user-facing features with intent, behaviors, and constraints
- **Tier 2 (Registered):** Utility libs, sidecars, config with intent and artifact mapping only
- **Tier 3 (Excluded):** Generated code, vendor deps, build output, explicitly out of scope

Anything not covered by any tier is flagged as unspecced, a lint warning that prompts you to assign or exclude it.

### Composable specs

Specs reference each other via `$ref` for hierarchical composition (`subsystems`) and cross-cutting concerns (`applies`). A security spec defined once is validated everywhere. A top-level `system.spec.yaml` serves as the manifest for the entire repo.

---

## Quick Example

```yaml
# .notarai/auth.spec.yaml
schema_version: "0.3"

intent: |
  Users can sign up, log in, and reset passwords.
  Sessions expire after 30 min of inactivity.

behaviors:
  - name: "signup"
    given: "valid email + password (>= 12 chars)"
    then: "account created, welcome email sent"
  - name: "login"
    given: "valid credentials"
    then: "JWT issued, session created"
  - name: "session_timeout"
    given: "30 min inactivity"
    then: "session invalidated"

constraints:
  - "rate limit: 5 login attempts per minute per IP"

invariants:
  - "no plaintext passwords stored anywhere"

artifacts:
  code:
    - path: "src/auth/**"
      role: "primary implementation"
  docs:
    - path: "docs/auth.md"
  tests:
    - path: "tests/auth/**"

sync_policy:
  on_code_change: "propose_spec_update"
  on_spec_change: "update_code_and_docs"
  on_doc_change: "propose_spec_update"
```

When a developer, direct LLM agent update, or SDD tool adds a Google OAuth endpoint to `src/auth/`, NotarAI detects that:

1. The spec has no behavior covering OAuth login
2. The rate limiting constraint may not apply to the new auth path
3. The docs don't mention the new login method

It proposes updates to all three (spec, code if the constraint needs enforcement, and docs) as part of the same PR.
