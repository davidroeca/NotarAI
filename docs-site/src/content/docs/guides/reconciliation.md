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

The reconciliation engine will:

1. Read all spec files in `.notarai/`
2. Examine the governed artifacts (code, docs, tests) listed in each spec's `artifacts` section
3. Compare the current state of artifacts against the spec's intent, behaviors, and constraints
4. Report any drift or conflicts found
5. Propose updates to bring everything back into alignment

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
