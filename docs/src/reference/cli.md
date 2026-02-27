# CLI Commands

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
- **Stale schema warning**: if `.notarai/notarai.spec.json` exists but its `$id` differs from the bundled schema, prints a warning suggesting `notarai init` to update.

**Exit codes:** `0` all files pass, `1` any file fails.

---

## notarai init

Set up NotarAI in a project. Running `init` again is safe: it always refreshes slash commands and the schema copy.

```sh
notarai init
```

**What it does:**

1. Adds a **PostToolUse hook** to `.claude/settings.json` so spec files are automatically validated when Claude Code writes or edits them (command: `notarai hook validate`).
2. Copies `notarai.spec.json` to `.notarai/notarai.spec.json` so Claude has the schema available (always refreshed to keep current).
3. Writes `.notarai/README.md` with workflow instructions (always overwritten).
4. Copies `notarai-reconcile.md` and `notarai-bootstrap.md` to `.claude/commands/` (always overwritten to stay in sync with the binary).
5. Replaces the `## NotarAI` section in `CLAUDE.md` with `@`-imports of the schema and README. Appends if the section is absent.
6. Appends `.notarai/.cache/` to `.gitignore` so the hash cache DB is never committed.
7. Writes `.mcp.json` registering `notarai mcp` as a local [MCP server](./mcp-server.md).

**Exit codes:** `0` success, `1` error.

---

## notarai schema-bump

Update the schema version across all specs in the project.

```sh
notarai schema-bump
```

Detects the schema version in `.notarai/notarai.spec.json` (if it exists) and compares it to the bundled schema. If they differ:

1. Overwrites `.notarai/notarai.spec.json` with the bundled schema.
2. Updates the `schema_version` field in every `.notarai/*.spec.yaml` file.
3. Validates all updated specs and reports any failures.

If versions already match, prints "Already at current schema version" and exits 0.

**Exit codes:** `0` success or already current, `1` validation error after update.

---

## notarai hook validate

PostToolUse hook handler. Validates spec files when Claude Code writes or edits them.

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

Show cache status: database path, entry count, and newest entry timestamp.

```sh
notarai cache status
```

Creates an empty database if none exists.

**Exit codes:** `0` success, `1` error.

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

See the [MCP Server reference](./mcp-server.md) for the full tool API, parameters, and return shapes.

**Exit codes:** `0` on stdin EOF.
