# Lint Rules

`notarai lint` checks spec quality beyond JSON Schema conformance. Each rule has a stable ID, a default severity, and a description. Rules can be configured per-project via `.notarai/lint.yaml`.

## Rules

| Rule | Default Severity | Description                                                                                                        |
| ---- | ---------------- | ------------------------------------------------------------------------------------------------------------------ |
| L001 | error            | Tier 1 (full) spec has zero behaviors. A full spec should describe at least one behavior.                          |
| L002 | warning          | Behavior missing `given` field. The trigger condition is unspecified.                                              |
| L003 | warning          | Behavior missing `then` field. The expected outcome is unspecified.                                                |
| L004 | error            | `$ref` target file does not exist on disk. A subsystem, applies, or dependency reference points to a missing file. |
| L005 | warning          | Circular `$ref` dependency detected. Spec reference chains form a cycle.                                           |
| L006 | warning          | Decision older than 90 days with no `rationale`. Stale decisions without context lose value over time.             |
| L007 | info             | Spec has `open_questions`. Consider resolving before reconciliation.                                               |
| L008 | warning          | Artifact glob is `**/*` (overly broad). Likely matches unintended files.                                           |
| L009 | error            | `schema_version` does not match the bundled schema. Run `notarai schema-bump` to update.                           |
| L010 | warning          | Duplicate behavior names within a spec. Each behavior should have a unique name.                                   |
| L011 | error            | Cross-cutting spec (`cross_cutting: true`) referenced from another spec's `subsystems`. Move it to `applies`.      |

## Usage

```sh
# Human-readable output (default)
notarai lint

# JSON output for CI
notarai lint --format json
```

Exit codes:

- `0`: No error-severity findings.
- `1`: At least one error-severity finding.
- `2`: `.notarai/` directory not found.

## Configuration

Create `.notarai/lint.yaml` to customize rule behavior:

```yaml
rules:
  L006:
    severity: info # downgrade from warning
    decision_age_days: 180 # custom threshold (default: 90)
  L007:
    enabled: false # disable rule entirely
  L008:
    severity: error # promote to error
```

Each rule supports:

- **enabled** (bool): Set to `false` to disable the rule. Default: `true`.
- **severity** (string): Override the default severity. Values: `error`, `warning`, `info`.
- Rule-specific parameters (e.g., `decision_age_days` for L006).

## Integration with `notarai check`

Lint rules run automatically as part of `notarai check`. Findings from L002, L003, and L005 are deduplicated against their check equivalents (BehaviorIncomplete and CircularRef). All other lint errors count as check errors and affect the exit code.

## JSON Output Format

```json
{
  "findings": [
    {
      "rule_id": "L001",
      "severity": "error",
      "spec_path": ".notarai/auth.spec.yaml",
      "message": "Tier 1 (full) spec has zero behaviors: .notarai/auth.spec.yaml"
    }
  ],
  "summary": {
    "errors": 1,
    "warnings": 0,
    "infos": 0
  }
}
```

Rule IDs are stable across versions and will never be renumbered.
