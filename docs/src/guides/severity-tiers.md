# Severity Tiers

NotarAI classifies every check and lint finding into one of three severity tiers. Tiers help you prioritize: fix critical issues first, review drift during development, and handle housekeeping when convenient.

## Tier definitions

| Tier | Name             | Meaning                                              | Example                                                                                                                           |
| ---- | ---------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 1    | **Critical**     | Broken references or structural violations           | Orphaned glob (spec references deleted code), circular `$ref` cycle, schema version mismatch (L009), missing `$ref` target (L004) |
| 2    | **Drift**        | Code changed in ways that may not align with spec    | Files changed since last reconciliation, Tier 1 spec with no behaviors (L001), duplicate behavior names (L010)                    |
| 3    | **Housekeeping** | Documentation, style, or organizational misalignment | Coverage gaps, overlapping coverage, incomplete behaviors, stale decisions (L006), open questions (L007), broad globs (L008)      |

## Tier assignment

Each check type and lint rule is assigned a fixed tier:

**Critical**: `CircularRef`, `OrphanedGlob`, `L004`, `L009`

**Drift**: `ChangedSinceReconciliation`, `L001`, `L010`

**Housekeeping**: `CoverageGap`, `OverlappingCoverage`, `BehaviorIncomplete`, `L002`, `L003`, `L005`, `L006`, `L007`, `L008`

## Output

Human output groups findings by tier (critical first):

```
--- Critical (2 findings) ---
  Orphaned Globs (1 findings)
    error  : src/deleted/**/*.rs
             in .notarai/cli.spec.yaml
  Circular $ref Cycles (1 findings)
    error  : .notarai/a.spec.yaml
             Circular $ref chain: a -> b -> a

--- Drift (1 findings) ---
  Changed Since Last Reconciliation (1 findings)
    warning: src/lib.rs

--- Housekeeping (3 findings) ---
  Coverage Gaps (3 findings)
    warning: README.md
    warning: LICENSE
    warning: CONTRIBUTING.md

6 issues found (2 errors, 4 warnings).
```

JSON output includes a `tier` field on each finding:

```json
{
  "findings": [
    {
      "type": "orphaned_glob",
      "severity": "error",
      "tier": "critical",
      "spec_path": ".notarai/cli.spec.yaml",
      "file_path": null,
      "glob_pattern": "src/deleted/**/*.rs",
      "message": "Artifact glob matches no files: src/deleted/**/*.rs"
    }
  ],
  "summary": { "errors": 1, "warnings": 0 }
}
```

## Configuring CI thresholds

Create `.notarai/check.yaml` to control which tiers cause CI failures:

```yaml
# Fail the check if any critical or drift finding is present.
fail_on: drift

# Only show critical and drift findings (suppress housekeeping).
warn_on: drift
```

**`fail_on`**: The minimum tier that causes a non-zero exit code. Accepts `critical`, `drift`, or `housekeeping`. When set, any finding at or above (lower number) the configured tier causes exit code 1. Default behavior (when omitted): fail only on error-severity findings.

**`warn_on`**: The minimum tier to include in output. Tiers below this threshold are suppressed entirely. Default: `housekeeping` (show everything).

The `--strict` flag overrides `fail_on` and causes any finding at all to produce exit code 1.

## LLM reconciliation tiers

The `/notarai-reconcile` skill and `notarai export-context` prompt also use the three-tier classification. When the LLM evaluates semantic drift, it classifies each finding as:

- **Critical**: Code contradicts a spec constraint, invariant, or behavior.
- **Drift**: Code has changed in ways not reflected in the spec, or new code is not covered by any behavior.
- **Housekeeping**: Documentation references outdated information, or style/naming diverges from spec conventions.
