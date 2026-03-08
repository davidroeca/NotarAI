---
paths:
  - '.notarai/notarai.spec.json'
  - '.notarai/**/*.spec.yaml'
  - 'notarai.spec.json'
  - 'skills/notarai-bootstrap/SKILL.md'
  - '.claude/skills/notarai-bootstrap/SKILL.md'
  - 'docs/**/*.md'
---

## Schema Version

When bumping the schema version, update ALL of these consistently:

1. `notarai.spec.json` -- `$id` URL and `schema_version` enum
2. Run `notarai schema-bump` in the repo to update `.notarai/notarai.spec.json` and all `.notarai/*.spec.yaml` files automatically
3. `skills/notarai-bootstrap/SKILL.md` and `.claude/skills/notarai-bootstrap/SKILL.md` -- template `schema_version`
4. Any example snippets in `docs/` that show `schema_version`
5. Add a `decisions` entry to the system spec recording the rationale

> Schema versioning decisions are recorded in `system.spec.yaml`.
