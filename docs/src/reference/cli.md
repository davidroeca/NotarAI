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

## notarai check

Deterministic, LLM-free drift detection. Reports coverage gaps, orphaned globs, changed files, overlapping coverage, circular `$ref` chains, and incomplete behaviors.

```sh
# Human-readable output (default)
notarai check

# JSON output
notarai check --format json

# Custom base branch
notarai check --base-branch develop

# Strict mode: promote all warnings to errors (useful for CI)
notarai check --strict
```

**Arguments:**

| Flag            | Required | Default | Description                                        |
| --------------- | -------- | ------- | -------------------------------------------------- |
| `--format`      | No       | `human` | Output format: `human` or `json`                   |
| `--base-branch` | No       | `main`  | Base branch for changed-file detection             |
| `--strict`      | No       | `false` | Promote all warnings to errors (zero-tolerance CI) |

**Checks performed:**

| Check                        | Severity | Description                                                     |
| ---------------------------- | -------- | --------------------------------------------------------------- |
| Coverage gaps                | Warning  | Tracked files not governed by any spec (minus excludes)         |
| Orphaned globs               | Error    | Artifact glob patterns matching zero files                      |
| Changed since reconciliation | Warning  | Governed files changed since last cache update                  |
| Overlapping coverage         | Warning  | Files governed by two or more specs                             |
| Circular `$ref` chains       | Error    | Cycles in `subsystems`, `applies`, or `dependencies` references |
| Behavior completeness        | Warning  | Behaviors missing a `given` or `then` field                     |

With `--strict`, all warning-severity findings are promoted to errors.

The check command never modifies files or the cache database.

**Exit codes:** `0` no error-severity findings, `1` errors found (including warnings promoted under `--strict`), `2` not initialized (`.notarai/` missing).

---

## notarai init

Set up NotarAI in a project. Running `init` again is safe: it always refreshes skills and the schema copy.

```sh
# Interactive prompt (defaults to claude)
notarai init

# Claude Code mode (explicit)
notarai init --agent claude

# Generic mode (any LLM agent)
notarai init --agent generic
```

**Arguments:**

| Flag      | Required | Description                                                         |
| --------- | -------- | ------------------------------------------------------------------- |
| `--agent` | No       | Agent type: `claude` or `generic`. Prompts interactively if omitted |

**Shared setup (both modes):**

1. Copies `notarai.spec.json` to `.notarai/notarai.spec.json` (always refreshed).
2. Writes `.notarai/README.md` with workflow instructions (always overwritten).
3. Appends `.notarai/.cache/` to `.gitignore`.
4. Writes `.mcp.json` registering `notarai mcp` as a local [MCP server](./mcp-server.md).

**Claude mode** (`--agent claude`):

5. Adds a **PostToolUse hook** to `.claude/settings.json` (command: `notarai hook validate`).
6. Copies `notarai-reconcile` and `notarai-bootstrap` skills to `.claude/skills/`.
7. Replaces the `## NotarAI` section in `CLAUDE.md` with a concise workflow description.

**Generic mode** (`--agent generic`):

5. Writes `AGENTS.md` with agent-agnostic NotarAI workflow documentation.
6. Writes `.notarai/reconcile-prompt.md` with a reconciliation prompt template containing `{{placeholders}}`.

**Exit codes:** `0` success, `1` error.

---

## notarai export-context

Export reconciliation context for any LLM agent. Outputs spec content, changed files, and diffs in a format suitable for feeding into a reconciliation prompt.

```sh
# Single spec, markdown output (default)
notarai export-context --spec .notarai/auth.spec.yaml

# All affected specs, JSON output
notarai export-context --all --format json

# Custom base branch
notarai export-context --spec .notarai/api.spec.yaml --base-branch develop
```

**Arguments:**

| Flag            | Required       | Default    | Description                           |
| --------------- | -------------- | ---------- | ------------------------------------- |
| `--spec`        | One of the two |            | Path to a single spec file            |
| `--all`         | One of the two |            | Export context for all affected specs |
| `--base-branch` | No             | `main`     | Base branch for diff                  |
| `--format`      | No             | `markdown` | Output format: `markdown` or `json`   |

Exactly one of `--spec` or `--all` is required.

**Markdown output** fills the bundled `reconcile-prompt.md` template with spec content, changed file list, and diff. Multiple specs are separated by `---`.

**JSON output** includes `spec_path`, `spec_name`, `spec_content`, `changed_files`, `diff`, `binary_changes`, and `file_categories`. A single spec produces an object; `--all` with multiple specs produces an array.

**Exit codes:** `0` success, `1` error (bad arguments, missing spec, git failure), `2` not initialized (`.notarai/` missing).

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

## notarai state

Manage the persistent reconciliation state file (`.notarai/reconciliation_state.json`). The state file records the last reconciliation timestamp, git hash, branch, and BLAKE3 fingerprints for all governed files and specs. It can be committed to the repo to give collaborators a baseline.

### notarai state show

Display the current reconciliation state.

```sh
notarai state show
```

Prints the timestamp, git hash, branch, and counts of tracked files and specs. Prints `No reconciliation state found.` if no state file exists.

**Exit codes:** `0` success, `1` error.

### notarai state reset

Delete the reconciliation state file, forcing the next reconciliation to treat everything as changed.

```sh
notarai state reset
```

Prints `Reconciliation state reset.` or `No reconciliation state to reset.` (if the file didn't exist).

**Exit codes:** `0` success, `1` error.

### notarai state snapshot

Build a new state snapshot from the current SQLite cache and save it to `.notarai/reconciliation_state.json`.

```sh
notarai state snapshot
```

Reads all entries from the cache, partitions them into file fingerprints and spec fingerprints, captures the current git HEAD and branch, and writes the result. This is the CLI equivalent of the [`snapshot_state` MCP tool](./mcp-server.md#snapshot_state).

**Exit codes:** `0` success, `1` error.

---

## notarai update

Check for and install updates.

```sh
# Check if an update is available
notarai update --check

# Update to the latest version
notarai update
```

**Arguments:**

| Flag      | Required | Description               |
| --------- | -------- | ------------------------- |
| `--check` | No       | Only check, don't install |

**Behavior:**

The command queries the GitHub API for the latest release, compares its version against the current binary, and prints the result. Without `--check`, it also attempts to install the update:

| Install method     | Detection                                  | Action                                       |
| ------------------ | ------------------------------------------ | -------------------------------------------- |
| **GitHub Release** | Binary is not in `.cargo/bin` or `target/` | Downloads and replaces the binary in place   |
| **cargo install**  | Binary path contains `.cargo/bin`          | Prints `cargo install notarai`               |
| **Dev build**      | Debug build or path contains `target/`     | Prints `cargo install --path crates/notarai` |

**Passive update hints:**

`notarai validate` and `notarai init` automatically check for updates in the background using a global cache with a 24-hour TTL and a 5-second network timeout. If a newer version is available, a one-line hint is printed to stderr. All errors are silently swallowed — the hint never interferes with normal output.

**Exit codes:** `0` success or up to date, `1` error or update failure.

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
