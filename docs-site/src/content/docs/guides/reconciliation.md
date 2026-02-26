---
title: Reconciliation
description: How NotarAI detects and resolves drift between specs, code, and docs.
---

## How reconciliation works

The reconciliation engine detects three scenarios:

### 1. Human edits code

The engine detects that code has drifted from the spec and proposes spec and doc updates.

### 2. Human edits spec

The engine propagates the spec change to code and documentation.

### 3. Conflict

Code says one thing, the spec says another. The engine surfaces the disagreement and the human decides which is correct.

The system is always **propose-and-approve**, never auto-sync. Both humans and LLMs can edit everything; the spec is the tiebreaker.

## Using reconciliation

After running `notarai init`, use the `/notarai-reconcile` slash command in Claude Code to trigger a reconciliation pass.

The reconciliation engine uses the `notarai` MCP server to serve pre-filtered data, keeping context usage proportional to what actually changed:

1. Calls `list_affected_specs` to identify which specs govern changed files
2. For each affected spec, calls `get_spec_diff` to get only the diff for files that spec governs
3. Calls `get_changed_artifacts` to get only doc artifacts that changed since the last reconciliation (using the hash cache to skip unchanged files)
4. Reads only those files, analyzes drift against the spec's behaviors, constraints, and invariants
5. Proposes targeted updates to bring spec, code, and docs back into alignment
6. Calls `mark_reconciled` to update the hash cache for next run

If the MCP server is unavailable, the command falls back to a manual flow using `git diff` and `notarai cache changed`.

## Sync policies

Each spec can declare a `sync_policy` that controls reconciliation behavior:

```yaml
sync_policy:
  on_code_change: 'propose_spec_update'
  on_spec_change: 'update_code_and_docs'
  on_doc_change: 'propose_spec_update'
```

Available strategies:

- `propose_spec_update` — suggest changes to the spec to match reality
- `update_code_and_docs` — propagate spec changes to governed artifacts

## Automatic validation

After `notarai init`, spec files are validated automatically whenever Claude Code writes or edits a file in `.notarai/`. Invalid specs block the tool use with errors on stderr. Non-spec files are ignored silently.
