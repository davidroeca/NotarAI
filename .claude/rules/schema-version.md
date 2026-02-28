---
paths:
  - '.notarai/notarai.spec.json'
  - '.notarai/**/*.spec.yaml'
  - 'notarai.spec.json'
  - 'commands/notarai-bootstrap.md'
  - '.claude/commands/notarai-bootstrap.md'
  - 'docs/**/*.md'
---

## Schema Version

When bumping the schema version, update ALL of these consistently:

1. `notarai.spec.json` -- `$id` URL and `schema_version` enum
2. Run `notarai schema-bump` in the repo to update `.notarai/notarai.spec.json` and all `.notarai/*.spec.yaml` files automatically
3. `commands/notarai-bootstrap.md` and `.claude/commands/notarai-bootstrap.md` -- template `schema_version`
4. Any example snippets in `docs/` that show `schema_version`
5. Add a `decisions` entry to the system spec recording the rationale

> Schema versioning decisions are recorded in `system.spec.yaml`.
