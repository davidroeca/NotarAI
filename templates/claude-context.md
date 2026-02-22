## NotarAI

This project uses [NotarAI](https://github.com/davidroeca/NotarAI) for intent
reconciliation. Specs in `.notarai/` are the canonical source of truth for what
this project should do. The spec is always the tiebreaker when code and intent
diverge.

### Spec Format

Specs are YAML files (`.notarai/**/*.spec.yaml`) with these key fields:

- `intent` — plain-language description of purpose
- `behaviors` — Given/Then scenarios describing observable behavior
- `constraints` — rules the system must actively enforce
- `invariants` — conditions that must NEVER be true (highest priority)
- `artifacts` — glob patterns mapping spec to governed files (code, docs, tests,
  notebooks, configs)
- `decisions` — log of architectural choices with rationale

**Tiers**: `full` (intent + behaviors), `registered` (intent + artifacts only,
for utilities), or excluded via `exclude:` globs in the system spec.

### Slash Commands

- `/notarai-reconcile` — detect drift between specs and code after making changes
- `/notarai-bootstrap` — generate initial specs for an existing codebase via
  developer interview

### Workflow

Spec files in `.notarai/` are validated automatically when written or edited.
Run `/notarai-reconcile` after significant code changes. When code and spec
disagree, surface the conflict rather than silently resolving it — the spec is
the tiebreaker.
