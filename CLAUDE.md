# NotarAI — Claude Code Context

NotarAI is a continuous intent reconciliation tool that keeps specs (`.notarai/*.spec.yaml`), code, and docs aligned. The project dogfoods itself — its own `.notarai/` specs govern its own code and docs. The next section below will be that self-referential usage doc.

## NotarAI

This project uses [NotarAI](https://github.com/davidroeca/NotarAI) for intent
reconciliation. Specs in `.notarai/` are the canonical source of truth for what
this project should do. The spec is always the tiebreaker when code and intent
diverge.

### Spec Format

Specs are YAML files (`.notarai/**/*.spec.yaml`) with these key fields:

- `intent` — plain-language description of purpose
- `behaviors` — Given/Then scenarios describing observable behavior
- `constraints` — rules the system must actively enforce
- `invariants` — conditions that must NEVER be true (highest priority)
- `artifacts` — glob patterns mapping spec to governed files (code, docs, tests,
  notebooks, configs)
- `decisions` — log of architectural choices with rationale

**Tiers**: `full` (intent + behaviors), `registered` (intent + artifacts only,
for utilities), or excluded via `exclude:` globs in the system spec.

### Slash Commands

- `/notarai-reconcile` — detect drift between specs and code after making changes
- `/notarai-bootstrap` — generate initial specs for an existing codebase via
  developer interview

### Workflow

Spec files in `.notarai/` are validated automatically when written or edited.
Run `/notarai-reconcile` after significant code changes. When code and spec
disagree, surface the conflict rather than silently resolving it — the spec is
the tiebreaker.

The full spec schema (field definitions, required fields, valid enums) is at:

@.claude/notarai.spec.json

## Build

```sh
npm run build        # tsc → dist/
```

Spec files in `.notarai/` are validated automatically via the PostToolUse hook when written or edited.

## Project Layout

```
src/
  bin.ts                  # CLI entry point — raw process.argv routing, no framework
  commands/
    validate.ts           # notarai validate [file|dir]
    init.ts               # notarai init — hook setup + command installation
    hook-validate.ts      # notarai hook validate — PostToolUse stdin handler
  lib/
    validator.ts          # AJV-based YAML→JSON Schema validation
    schema.ts             # loads notarai.spec.json at module init
    yaml.ts               # js-yaml wrapper returning discriminated union
notarai.spec.json         # the JSON Schema all spec files are validated against
commands/                 # bundled slash command sources (copied by `notarai init`)
  notarai-reconcile.md
  notarai-bootstrap.md
templates/                # bundled templates (written by `notarai init`)
  claude-context.md
.notarai/                 # this project's own specs
dist/                     # build output (gitignored)
```

## Key Architectural Constraints

- **No CLI framework** — `bin.ts` uses raw `process.argv`. Keep it that way.
- **No bundler** — TypeScript compiles directly to `dist/` and runs on Node.js. `module: "nodenext"` in tsconfig requires `.js` extensions on all local imports (even for `.ts` source files).
- **AJV ESM interop** — AJV ships as CJS. The double-cast at `validator.ts:11` (`new (Ajv as unknown as typeof Ajv.default)`) is intentional; don't simplify it.
- **Schema loaded once** — `schema.ts` loads and `validator.ts` compiles the AJV validator at module init. This is a constraint in the spec; don't move it to per-call.

## Schema Version

When bumping the schema version, update ALL of these consistently:

1. `notarai.spec.json` — `$id` URL and `schema_version` enum
2. All specs in `.notarai/` — `schema_version` field
3. `commands/notarai-bootstrap.md` + `.claude/commands/notarai-bootstrap.md` — template `schema_version`
4. Any example snippets in `docs-site/` that show `schema_version`
5. Add a `decisions` entry to the system spec recording the rationale

## Slash Commands

`commands/` holds the **source of truth** for slash command prompts. `notarai init` copies them to the target project's `.claude/commands/`. When editing a command prompt, update **both** copies:

- `commands/<name>.md` (source)
- `.claude/commands/<name>.md` (local installed copy)

## CLAUDE.md Generation

`notarai init` writes a `## NotarAI` section to the target project's `CLAUDE.md` (appending to an existing file, or creating it) and copies `notarai.spec.json` to `.claude/notarai.spec.json`. The CLAUDE.md template (`templates/claude-context.md`) includes an `@.claude/notarai.spec.json` import so Claude auto-loads the schema in every conversation. The CLAUDE.md operation is idempotent — if a `## NotarAI` heading is already present (matched as a line-anchored heading, not inline text), init skips it. The schema copy always overwrites to keep it current.

## No Tests Yet

There is no test suite. When adding test coverage, check with the user before choosing a framework.
