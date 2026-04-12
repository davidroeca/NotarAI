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

The skill is a thin orchestrator that delegates context assembly to the `notarai export-context` CLI command:

1. Determines a baseline (from `.notarai/reconciliation_state.json` if available, or asks for a base branch).
2. Runs `notarai export-context --all --base-branch <baseline> --format markdown` to gather per-spec reconciliation blocks containing spec content and changed-file lists.
3. For small changesets (10 or fewer changed files), analyzes all specs inline. For larger changesets, spawns one parallel sub-agent per spec.
4. Reads changed files, runs `git diff` per file, and evaluates each behavior, constraint, and invariant against the changes.
5. Notes `applies` cross-cutting specs and `dependencies` refs for ripple-effect analysis.
6. Produces a structured report (DRIFT / VIOLATED / UNSPECCED / STALE REF findings).
7. Walks through findings interactively, proposing exact changes for approval.
8. Calls `mark_reconciled` (via MCP or CLI) to update the hash cache, then snapshots reconciliation state.

The [MCP server](../reference/mcp-server.md) is used for `mark_reconciled` and `snapshot_state` when available, with CLI fallbacks (`notarai state snapshot`) when it is not.

For non-Claude agents, run `notarai export-context` directly and paste the output into your agent's prompt. See the [CLI reference](../reference/cli.md#notarai-export-context) for details.

## Automatic validation

After `notarai init`, spec files are validated automatically whenever Claude Code writes or edits a file in `.notarai/`. Invalid specs block the tool use with errors on stderr. Non-spec files are ignored silently.
