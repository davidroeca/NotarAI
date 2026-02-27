---
title: MCP Server
description: Reference for NotarAI's MCP server — tool definitions, parameters, return shapes, and cache semantics.
---

NotarAI includes a built-in [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server that serves pre-filtered diffs and change data to the reconciliation engine. This keeps context usage proportional to what actually changed rather than the full repository.

## Setup

`notarai init` writes an `.mcp.json` file to the project root that registers the MCP server:

```json
{
  "mcpServers": {
    "notarai": {
      "type": "stdio",
      "command": "notarai",
      "args": ["mcp"]
    }
  }
}
```

Claude Code reads this file and starts the server automatically. No manual configuration needed.

## Protocol

- **Transport:** stdio (stdin/stdout)
- **Format:** JSON-RPC 2.0, one message per line
- **Execution:** synchronous (no async runtime)
- **Protocol version:** `2024-11-05`

## Tools

### list_affected_specs

Identify which specs govern files that changed on the current branch relative to a base branch.

**Parameters:**

| Parameter     | Type   | Required | Description                             |
| ------------- | ------ | -------- | --------------------------------------- |
| `base_branch` | string | Yes      | Branch to diff against (e.g., `"main"`) |

**Returns:**

```json
{
  "changed_files": ["src/auth.rs", "src/main.rs"],
  "affected_specs": [
    {
      "spec_path": ".notarai/cli.spec.yaml",
      "behaviors": [...],
      "constraints": [...],
      "invariants": [...]
    }
  ]
}
```

Each affected spec includes its `behaviors`, `constraints`, and `invariants` so the reconciliation engine has the context to evaluate drift without additional file reads.

---

### get_spec_diff

Get the git diff filtered to files governed by a specific spec. Uses the hash cache to skip files that haven't changed since the last reconciliation.

**Parameters:**

| Parameter          | Type     | Required | Description                                                                                |
| ------------------ | -------- | -------- | ------------------------------------------------------------------------------------------ |
| `spec_path`        | string   | Yes      | Relative path to the spec file                                                             |
| `base_branch`      | string   | Yes      | Branch to diff against                                                                     |
| `exclude_patterns` | string[] | No       | Glob patterns to exclude via git `:(exclude)` pathspecs (e.g., `["Cargo.lock", "*.lock"]`) |
| `bypass_cache`     | boolean  | No       | If `true`, diff all governed files regardless of cache state. Defaults to `false`          |

**Returns:**

```json
{
  "diff": "unified diff of non-spec governed files...",
  "files": ["src/auth.rs"],
  "skipped": ["src/utils.rs"],
  "excluded": ["Cargo.lock"],
  "spec_changes": [
    {
      "path": ".notarai/cli.spec.yaml",
      "content": "full file content..."
    }
  ],
  "system_spec": {
    "path": ".notarai/system.spec.yaml",
    "content": "full file content..."
  }
}
```

| Field          | Description                                                                                                          |
| -------------- | -------------------------------------------------------------------------------------------------------------------- |
| `diff`         | Unified diff output for non-spec artifact files only                                                                 |
| `files`        | Non-spec files included in the diff                                                                                  |
| `skipped`      | Non-spec files whose BLAKE3 hash matched the cache (already reconciled)                                              |
| `excluded`     | Patterns passed via `exclude_patterns`                                                                               |
| `spec_changes` | Array of `{path, content}` for each governed `.notarai/**/*.spec.yaml` file that changed                             |
| `system_spec`  | The system spec (the spec with a `subsystems` key) — included whenever `spec_changes` is non-empty; `null` otherwise |

**Why full content for spec files?**

Spec files express intent, not implementation. The reconciliation engine needs the complete spec to evaluate drift — diff hunks showing only changed lines lack the context to determine whether behavior is still satisfied. Returning full content also avoids the ambiguity of partial context when the spec is the source of truth.

**Spec deduplication:** If the system spec itself changed, it appears in `spec_changes` with full content and `system_spec` contains only `{path}` (a reference) to avoid duplicating the content.

**Cache behavior:**

- Files whose on-disk BLAKE3 hash matches the cached hash are listed in `skipped` (for artifact files) or omitted from `spec_changes` (for spec files).
- A cold or absent cache causes all governed files to be included. This is a safe fallback that ensures nothing is missed.
- `bypass_cache: true` forces a full diff without destroying the cache (useful for re-checking everything).

---

### get_changed_artifacts

Get artifact files governed by a spec that have changed since the last cache update. Useful for identifying which docs or other artifacts need review during reconciliation.

**Parameters:**

| Parameter       | Type   | Required | Description                                                     |
| --------------- | ------ | -------- | --------------------------------------------------------------- |
| `spec_path`     | string | Yes      | Relative path to the spec file                                  |
| `artifact_type` | string | No       | Filter by artifact type (e.g., `"docs"`, `"code"`, `"configs"`) |

**Returns:**

```json
{
  "changed_artifacts": ["docs/auth.md", "docs/api-reference.md"]
}
```

Only files whose content differs from the cached hash are included. If no `artifact_type` is specified, all artifact types are checked.

---

### mark_reconciled

Update the hash cache after reconciliation is complete. Call this at the end of a reconciliation pass so that subsequent runs skip files that haven't changed.

**Parameters:**

| Parameter | Type     | Required | Description                  |
| --------- | -------- | -------- | ---------------------------- |
| `files`   | string[] | Yes      | Relative file paths to cache |

**Returns:**

```json
{
  "updated": 5
}
```

Files are hashed with BLAKE3 and stored with their relative paths as cache keys. Non-existent files are silently skipped.

---

### clear_cache

Delete the reconciliation cache database, forcing the next `get_spec_diff` call to diff all governed files.

**Parameters:** None.

**Returns:**

```json
{
  "cleared": true
}
```

Returns `true` if the database was deleted, `false` if it didn't exist.

## Cache semantics

The cache is a SQLite database at `.notarai/.cache/notarai.db` with a single table:

```sql
file_cache(path TEXT PRIMARY KEY, blake3_hash TEXT, updated_at INTEGER)
```

**Key details:**

- **Hash algorithm:** [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) — fast cryptographic hash.
- **Path format:** MCP tools use **relative paths** as cache keys. The CLI `cache update`/`cache changed` commands use canonical absolute paths. These are separate namespaces — seed the MCP cache via `mark_reconciled`, not `notarai cache update`.
- **Cold cache:** When the cache is empty or absent, `get_spec_diff` diffs all governed files. This is the safe default.
- **Cache location:** `.notarai/.cache/` is gitignored by `notarai init` so the cache is never committed.

## Error codes

| Code     | Meaning                                                   |
| -------- | --------------------------------------------------------- |
| `-32700` | Parse error (malformed JSON)                              |
| `-32601` | Method not found                                          |
| `-32602` | Invalid params (missing required parameter)               |
| `-32603` | Internal error (git failure, file I/O, cache unavailable) |
