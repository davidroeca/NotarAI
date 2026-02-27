# NotarAI — Claude Code Context

NotarAI is a continuous intent reconciliation tool that keeps specs (`.notarai/*.spec.yaml`), code, and docs aligned. The project dogfoods itself — its own `.notarai/` specs govern its own code and docs. The next section below will be that self-referential usage doc.

## NotarAI

@.notarai/README.md
@.notarai/notarai.spec.json

## Build

```sh
cargo build --release    # → target/release/notarai
cargo test               # run all tests
npm ci                   # install prettier (for formatting hooks)
```

Spec files in `.notarai/` are validated automatically via the PostToolUse hook when written or edited. Rust files are auto-formatted with `rustfmt` and non-Rust files with `prettier` via PostToolUse hooks.

## Project Layout

```
src/
  main.rs                   # CLI entry point -- clap derive API
  commands/
    mod.rs
    validate.rs             # notarai validate [file|dir]
    init.rs                 # notarai init -- hook setup + command installation
    hook_validate.rs        # notarai hook validate -- PostToolUse stdin handler
    cache.rs                # notarai cache <status|clear>
    mcp.rs                  # notarai mcp -- JSON-RPC 2.0 MCP server
    schema_bump.rs          # notarai schema-bump -- version migration
  core/
    mod.rs
    validator.rs            # jsonschema-based YAML->JSON Schema validation
    schema.rs               # include_str! + OnceLock for bundled schema
    yaml.rs                 # serde_yaml_ng -> serde_json::Value conversion
    cache.rs                # BLAKE3+SQLite hash cache
    mcp_tools.rs            # MCP tool implementations
notarai.spec.json           # the JSON Schema all spec files are validated against
commands/                   # bundled slash command sources (copied by `notarai init`)
  notarai-reconcile.md
  notarai-bootstrap.md
templates/                  # bundled templates (written by `notarai init`)
  notarai-readme.md         # .notarai/README.md template ({{VERSION}} injected at write time)
.notarai/                   # this project's own specs
docs/                       # mdBook documentation source
  book.toml
  src/
tests/                      # integration tests (assert_cmd + tempfile)
package.json                # npm root (prettier for formatting hooks)
prettier.config.mjs         # prettier config for non-Rust files
target/                     # build output (gitignored)
```

## Key Architectural Constraints

- **clap derive API** — `main.rs` uses clap 4.x derive macros for argument parsing and help generation.
- **No bundler/runtime** — Rust compiles to a single static binary. All assets are embedded via `include_str!` at compile time.
- **Schema compiled once** — `schema.rs` parses and `validator.rs` compiles the jsonschema validator at process init via `OnceLock`. Don't move to per-call.
- **Module naming** — Core library lives in `src/core/` (not `src/lib/`) to avoid Rust's reserved `lib` module name.

## Schema Version

When bumping the schema version, update ALL of these consistently:

1. `notarai.spec.json` — `$id` URL and `schema_version` enum
2. Run `notarai schema-bump` in the repo to update `.notarai/notarai.spec.json` and all `.notarai/*.spec.yaml` files automatically
3. `commands/notarai-bootstrap.md` and `.claude/commands/notarai-bootstrap.md` — template `schema_version`
4. Any example snippets in `docs/` that show `schema_version`
5. Add a `decisions` entry to the system spec recording the rationale

## Slash Commands

`commands/` holds the **source of truth** for slash command prompts. `notarai init` always overwrites `.claude/commands/` with the bundled content on every run. When editing a command prompt, update `commands/<name>.md` (the source); the installed copy will be refreshed on the next `notarai init` run, or you can copy it manually to `.claude/commands/<name>.md` in the meantime.

## CLAUDE.md Generation

`notarai init` writes a `## NotarAI` section to the target project's `CLAUDE.md`. The section contains two `@` imports:

```
## NotarAI
@.notarai/README.md
@.notarai/notarai.spec.json
```

If a `## NotarAI` heading already exists, the section is replaced with the current content (not skipped). The schema is copied to `.notarai/notarai.spec.json` and the README template is written to `.notarai/README.md` on every run (always overwrite).

## Tests

Tests use `cargo test`. Unit tests are inline `#[cfg(test)]` modules in source files. Integration tests in `tests/` use `assert_cmd` for CLI binary testing and `tempfile` for isolated init tests.

## Style

- **American English** throughout — code, comments, docs, and specs (e.g. "initialize" not "initialise").
- **ASCII in code and comments** — use `--` for dashes, `->` for arrows, plain quotes. Non-ASCII is fine in user-facing printed strings where it aids readability.
