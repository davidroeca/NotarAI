# Drift Scoring

`notarai score` computes a numeric drift score for each spec so you can
prioritize reconciliation work. Scores are deterministic, produced
without LLM calls, and always informational (exit code 0).

## Score range and status

Each spec gets a score in `[0.0, 1.0]`:

| Range     | Status    | Meaning                             |
| --------- | --------- | ----------------------------------- |
| 0.0 - 0.3 | `healthy` | Spec is aligned with code and docs. |
| 0.3 - 0.6 | `review`  | Some drift signals; review soon.    |
| 0.6 - 1.0 | `overdue` | High drift; reconcile now.          |

The overall score is the mean across all specs.

## Signals

Six weighted signals contribute to each spec's score. All weights are
configurable via `.notarai/scoring.yaml`; defaults are shown below.

| Signal                      | Default weight | What it measures                                   |
| --------------------------- | -------------- | -------------------------------------------------- |
| `files_changed`             | 0.30           | Governed files changed since last reconciliation.  |
| `days_since_reconciliation` | 0.20           | Days since the reconciliation state was updated.   |
| `unresolved_decisions`      | 0.15           | Proposed (not yet accepted) decisions for spec.    |
| `orphaned_globs`            | 0.15           | Artifact glob patterns that match zero files.      |
| `open_questions`            | 0.10           | `open_questions` entries in the spec YAML.         |
| `unspecced_files`           | 0.10           | Tracked files in governed directories not covered. |

Each signal is normalized to `[0.0, 1.0]` before being multiplied by
its weight. The final score is clamped to 1.0.

## Usage

```sh
notarai score
notarai score --format json
notarai score --spec .notarai/cli.spec.yaml
```

### Human output

```
Spec                                          Score   Status
-----------------------------------------------------------------
.notarai/cli.spec.yaml                        0.42    review
.notarai/system.spec.yaml                     0.18    healthy

Overall: 0.30 (review)
```

### JSON output

```json
{
  "specs": [
    {
      "spec_path": ".notarai/cli.spec.yaml",
      "score": 0.42,
      "status": "review"
    },
    {
      "spec_path": ".notarai/system.spec.yaml",
      "score": 0.18,
      "status": "healthy"
    }
  ],
  "overall": { "score": 0.3, "status": "review" }
}
```

## Configuration

Create `.notarai/scoring.yaml` to override default weights:

```yaml
files_changed: 0.4
days_since_reconciliation: 0.1
unresolved_decisions: 0.2
orphaned_globs: 0.1
open_questions: 0.1
unspecced_files: 0.1
```

Weights do not need to sum to 1.0, but keeping them normalized makes
the resulting score easier to interpret.

## MCP integration

The `initialize` response from `notarai mcp` includes:

- `driftScore` -- overall score (0.0-1.0)
- `driftStatus` -- `healthy` / `review` / `overdue`
- `mostDrifted` -- spec path with the highest score

This lets agents quickly orient without running a separate command.
