---
title: CLI Commands
description: Complete reference for all NotarAI CLI commands, flags, and exit codes.
---

NotarAI is distributed as a single static binary with no runtime dependencies. All commands use the `notarai` prefix.

## notarai validate

Validate spec files against the JSON Schema.

```sh
# Validate all specs in .notarai/ (default)
notarai validate

# Validate a specific file
notarai validate .notarai/auth.spec.yaml

# Validate a directory
notarai validate .notarai/subsystems/
```

**Arguments:**

| Argument | Required | Description                                            |
| -------- | -------- | ------------------------------------------------------ |
| `path`   | No       | File or directory to validate. Defaults to `.notarai/` |

**Behavior:**

- **Single file**: validates against the schema, prints `PASS` or `FAIL` with indented errors.
- **Directory**: recursively finds all `.spec.yaml` files and validates each.
- **No specs found**: exits 0 with a warning on stderr.
- **Stale schema warning**: if `.claude/notarai.spec.json` exists but its `$id` differs from the bundled schema, prints a warning suggesting `notarai init` to update.

**Exit codes:** `0` all files pass, `1` any file fails.

---

## notarai init

Set up NotarAI in a project. Running `init` again is safe — it detects existing configuration and skips.

```sh
notarai init
```

**What it does (7 steps):**

1. Adds a **PostToolUse hook** to `.claude/settings.json` so spec files are automatically validated when Claude Code writes or edits them (command: `notarai hook validate`).
2. Copies `notarai-reconcile.md` to `.claude/commands/` for drift detection.
3. Copies `notarai-bootstrap.md` to `.claude/commands/` for bootstrapping specs from an existing codebase.
4. Copies `notarai.spec.json` to `.claude/notarai.spec.json` so Claude has the schema available in every session (always refreshed to keep current).
5. Appends a `## NotarAI` section to `CLAUDE.md` with workflow instructions and an `@`-import of the schema. If the section already exists but differs from the bundled template, prints a warning suggesting manual review.
6. Appends `.notarai/.cache/` to `.gitignore` so the hash cache DB is never committed.
7. Writes `.mcp.json` registering `notarai mcp` as a local [MCP server](/NotarAI/docs/reference/mcp-server/).

**Exit codes:** `0` success, `1` error.

---

## notarai hook validate

PostToolUse hook handler — validates spec files when Claude Code writes or edits them.

```sh
# Called automatically by Claude Code, not typically invoked manually
notarai hook validate
```

Reads PostToolUse JSON from stdin. If the file path matches `.notarai/**/*.spec.yaml`, reads the file from disk and validates it. Invalid specs block the tool use with errors on stderr.

**Behavior:**

| Stdin                                      | Result                                    |
| ------------------------------------------ | ----------------------------------------- |
| Spec file path (`.notarai/**/*.spec.yaml`) | Validates; exits 1 with errors if invalid |
| Non-spec file path                         | Exits 0 silently                          |
| Invalid JSON or missing file               | Exits 0 silently (graceful degradation)   |

**Exit codes:** `0` valid or non-spec file, `1` invalid spec.

---

## notarai cache

BLAKE3 + SQLite hash cache for tracking file changes between reconciliation runs. The cache database lives at `.notarai/.cache/notarai.db`.

### notarai cache status

Show cache status — database path, entry count, and newest entry timestamp.

```sh
notarai cache status
```

Creates an empty database if none exists.

**Exit codes:** `0` success, `1` error.

### notarai cache update

Hash files with BLAKE3 and store results in the cache.

```sh
# Update specific files
notarai cache update src/auth.rs src/main.rs

# Update from stdin (one path per line)
echo "src/auth.rs" | notarai cache update
```

**Arguments:**

| Argument   | Required | Description                                        |
| ---------- | -------- | -------------------------------------------------- |
| `paths...` | No       | Files to hash and cache. Reads from stdin if empty |

Paths are resolved to canonical absolute paths before storage. Prints `Updated N file(s)` on success.

**Exit codes:** `0` success, `1` error.

### notarai cache changed

Print paths whose content has changed since the last cache update.

```sh
# Check specific files
notarai cache changed src/auth.rs src/main.rs

# Check from stdin (one path per line)
echo "src/auth.rs" | notarai cache changed
```

**Arguments:**

| Argument   | Required | Description                               |
| ---------- | -------- | ----------------------------------------- |
| `paths...` | No       | Files to check. Reads from stdin if empty |

Prints changed file paths one per line. Files absent from the cache are treated as changed. Empty output means nothing changed.

**Exit codes:** `0` always (even if files changed).

### notarai cache clear

Delete the cache database.

```sh
notarai cache clear
```

Prints `Cache cleared` or `Cache not initialized` (if the DB didn't exist). No-op if the file does not exist.

**Exit codes:** `0` success, `1` error.

---

## notarai mcp

Start a synchronous JSON-RPC 2.0 MCP server over stdio. Typically configured automatically by `notarai init` rather than invoked manually.

```sh
notarai mcp
```

The server reads JSON-RPC messages line-by-line from stdin and writes responses to stdout. It exits cleanly on stdin EOF.

**Protocol:** JSON-RPC 2.0 over stdio (synchronous, no async runtime).

**Setup:** `notarai init` writes `.mcp.json` to the project root, which Claude Code reads to auto-start the server:

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

See the [MCP Server reference](/NotarAI/docs/reference/mcp-server/) for the full tool API, parameters, and return shapes.

**Exit codes:** `0` on stdin EOF.
