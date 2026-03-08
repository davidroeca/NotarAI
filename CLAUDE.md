# NotarAI — Claude Code Context

NotarAI is a continuous intent reconciliation tool that keeps specs (`.notarai/*.spec.yaml`), code, and docs aligned. The project dogfoods itself — its own `.notarai/` specs govern its own code and docs. The section at the very bottom of the document will be that self-referential usage doc.

Project conventions (style, testing, schema versioning) live in `.claude/rules/`. See the [Specs vs Claude Rules](docs/src/guides/specs-vs-claude-rules.md) guide for when to use specs vs rules.

## NotarAI

Specs live in `.notarai/*.spec.yaml` and are the canonical source of truth.
Run `/notarai-reconcile` to detect drift between specs, code, and docs.
Run `notarai validate .notarai/` to validate specs manually.
The PostToolUse hook auto-validates any spec file you write or edit.
