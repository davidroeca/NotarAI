# NotarAI — Claude Code Context

NotarAI is a continuous intent reconciliation tool that keeps specs (`.notarai/*.spec.yaml`), code, and docs aligned. The project dogfoods itself — its own `.notarai/` specs govern its own code and docs. The section at the very bottom of the document will be that self-referential usage doc.

## Schema Version

When bumping the schema version, update ALL of these consistently:

1. `notarai.spec.json` — `$id` URL and `schema_version` enum
2. Run `notarai schema-bump` in the repo to update `.notarai/notarai.spec.json` and all `.notarai/*.spec.yaml` files automatically
3. `commands/notarai-bootstrap.md` and `.claude/commands/notarai-bootstrap.md` — template `schema_version`
4. Any example snippets in `docs/` that show `schema_version`
5. Add a `decisions` entry to the system spec recording the rationale

## Tests

Tests use `cargo test`. Unit tests are inline `#[cfg(test)]` modules in source files. Integration tests in `tests/` use `assert_cmd` for CLI binary testing and `tempfile` for isolated init tests.

## Style

- **American English** throughout — variable names, function names, type names, comments, docs, and specs (e.g. "initialize" not "initialise", "color" not "colour").
- **QWERTY-typable characters in code and comments** — no em-dashes, curly quotes, or other characters requiring special input. Use `--` for em-dash, `->` for arrows, straight quotes. Non-typable characters are fine in user-facing printed strings where they aid readability.

## NotarAI

@.notarai/README.md
@.notarai/notarai.spec.json
