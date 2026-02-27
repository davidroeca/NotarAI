# Reconciliation

## How reconciliation works

The reconciliation engine detects three scenarios:

### 1. Someone edits code

The engine detects that code has drifted from the spec and proposes spec and doc updates.

### 2. Someone edits spec

The engine propagates the spec change to code and documentation.

### 3. Conflict

Code says one thing, the spec says another. The engine surfaces the disagreement and the user decides which is correct.

The system is always **propose-and-approve**, never auto-sync. Both users and LLMs can edit everything; the spec is the tiebreaker.

## Using reconciliation

After running `notarai init`, use the `/notarai-reconcile` slash command in Claude Code to trigger a reconciliation pass.

The reconciliation engine uses the `notarai` MCP server to serve pre-filtered data, keeping context usage proportional to what actually changed:

1. Calls `list_affected_specs` to identify which specs govern changed files.
2. For each affected spec, calls `get_spec_diff` to get only the diff for files that spec governs. Files already reconciled (per the BLAKE3 hash cache) are skipped and listed in the `skipped` field. Pass `exclude_patterns` to suppress noisy files like lockfiles; pass `bypass_cache: true` to force a full diff without clearing the cache.
3. Loads any `applies` cross-cutting specs and merges their invariants and constraints into the analysis.
4. Notes any `dependencies` refs for ripple-effect analysis.
5. Calls `get_changed_artifacts` to get only doc artifacts that changed since the last reconciliation.
6. Reads only those files, analyzes drift against the spec's behaviors, constraints, and invariants.
7. Proposes targeted updates to bring spec, code, and docs back into alignment.
8. Calls `mark_reconciled` to update the hash cache for the next run.

See the [MCP Server reference](../reference/mcp-server.md) for full tool parameters and return shapes.

If the MCP server is unavailable, the command falls back to a manual flow using `git diff` directly.

## Automatic validation

After `notarai init`, spec files are validated automatically whenever Claude Code writes or edits a file in `.notarai/`. Invalid specs block the tool use with errors on stderr. Non-spec files are ignored silently.
