---
title: Spec Format Reference
description: Complete reference for the NotarAI spec YAML format.
---

Specs are YAML files validated against a JSON Schema (`notarai.spec.json`). The format uses progressive disclosure: a small set of required fields for minimum viability, with optional fields for precision as needed.

## Required fields

### `schema_version`

Pins the JSON Schema version. Current version: `"0.4"`.

```yaml
schema_version: '0.4'
```

### `intent`

Natural language description of what the system or feature should do.

```yaml
intent: |
  Users can sign up, log in, and reset passwords.
  Sessions expire after 30 min of inactivity.
```

### `behaviors`

Structured Given/Then entries describing expected behavior. Each behavior has a `name`, a `given` condition, and a `then` outcome.

```yaml
behaviors:
  - name: 'signup'
    given: 'valid email + password (>= 12 chars)'
    then: 'account created, welcome email sent'
  - name: 'session_timeout'
    given: '30 min inactivity'
    then: 'session invalidated'
```

### `artifacts`

Glob patterns mapping the spec to the files it governs. Organized by category (`code`, `docs`, `tests`).

```yaml
artifacts:
  code:
    - path: 'src/auth/**'
      role: 'primary implementation'
  docs:
    - path: 'docs/auth.md'
  tests:
    - path: 'tests/auth/**'
```

## Optional fields

### `constraints`

Rules the system must follow.

```yaml
constraints:
  - 'rate limit: 5 login attempts per minute per IP'
  - 'passwords must be >= 12 characters'
```

### `invariants`

Conditions that must never be violated.

```yaml
invariants:
  - 'no plaintext passwords stored anywhere'
  - 'all API responses include request-id header'
```

### `decisions`

Architectural decision log with date, choice, and rationale.

```yaml
decisions:
  - date: '2025-01-15'
    choice: 'JWT over session cookies'
    rationale: 'Stateless auth simplifies horizontal scaling'
```

### `open_questions`

Unresolved design questions.

```yaml
open_questions:
  - 'Should we support OAuth providers beyond Google?'
  - "What's the session timeout for mobile clients?"
```

### `dependencies`

References to other specs this one interacts with.

```yaml
dependencies:
  - $ref: 'billing.spec.yaml'
    relationship: 'auth gates billing endpoints'
```

### `sync_policy`

Configurable strategy for when and how reconciliation runs.

```yaml
sync_policy:
  on_code_change: 'propose_spec_update'
  on_spec_change: 'update_code_and_docs'
  on_doc_change: 'propose_spec_update'
```

### `notes`

Freeform hints for the LLM about implicit relationships.

```yaml
notes: |
  The auth module shares a rate limiter with the API gateway.
  Session storage is Redis in production, in-memory in dev.
```

## Coverage tiers

Every file in the repo falls into one of three tiers:

- **Tier 1 (Full Spec)** — Business logic, APIs, user-facing features. Full behavioral specification.
- **Tier 2 (Registered)** — Utilities, config, sidecars. Intent and artifact mapping only, no behaviors required.
- **Tier 3 (Excluded)** — Generated code, vendor deps, editor configs. Explicitly out of scope.

Files not covered by any tier are flagged as "unspecced" — a lint warning, not a blocker.

## Composition

Specs compose via `$ref` (borrowed from JSON Schema/OpenAPI):

- `subsystems` — hierarchical references (system → services)
- `applies` — cross-cutting specs (e.g., security, logging) that apply to all subsystems

A top-level `system.spec.yaml` serves as the manifest, referencing subsystem specs and declaring exclusion patterns for Tier 3 files.
