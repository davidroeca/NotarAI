# Reconciliation: {{spec_name}}

## Spec Content

```yaml
{ { spec_content } }
```

## Changed Files

{{changed_files}}

## Diff

```diff
{{diff}}
```

## Instructions

Review the diff above against the spec. For each behavior, constraint, and invariant:

1. Check whether the changes support or contradict it
2. Identify any drift between spec intent and implementation
3. Propose targeted updates to bring spec, code, and docs into alignment

The spec is the canonical tiebreaker when code and spec disagree.

Report findings in this format:

- **DRIFT**: Code has changed in ways not reflected in the spec
- **VIOLATED**: Code contradicts a spec constraint or invariant
- **UNSPECCED**: New code not covered by any spec behavior
- **STALE REF**: Spec references code or docs that no longer exist

If everything aligns, report: "No drift detected."
