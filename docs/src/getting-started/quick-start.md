# Quick Start

## Initialize your project

Run `notarai init` in your project root:

```sh
notarai init
```

This does several things:

1. Copies `notarai.spec.json` to `.notarai/notarai.spec.json` so the schema is available for validation.
2. Writes `.notarai/README.md` with workflow instructions.
3. Writes `.notarai/reconcile-prompt.md` (reconciliation prompt template).
4. Writes `.notarai/bootstrap-prompt.md` (bootstrap prompt template).
5. Appends `.notarai/.cache/` to `.gitignore` so the hash cache DB is never committed.
6. Writes `.mcp.json` registering `notarai mcp` as a local MCP server, so [MCP-accelerated reconciliation](../reference/mcp-server.md) works out of the box.
7. Writes or section-merges `AGENTS.md` with a `## NotarAI` section describing the workflow.
8. For the Claude adapter: adds a **PostToolUse hook** to `.claude/settings.json` so spec files are automatically validated when Claude Code writes or edits them; copies reconcile and bootstrap skills to `.claude/skills/`; creates or section-merges `CLAUDE.md` as an `@AGENTS.md` pointer.

Running `init` again is safe: it always refreshes skills, templates, and the schema copy, and replaces the `## NotarAI` section in AGENTS.md (and adapter pointer files) with the current content.

## Create your first spec

Specs live in a `.notarai/` directory at the root of your repository:

```
project/
  .notarai/
    system.spec.yaml
    auth.spec.yaml
    billing.spec.yaml
    _shared/
      security.spec.yaml
  src/
  docs/
```

Here's a minimal spec:

```yaml
# .notarai/auth.spec.yaml
schema_version: '0.8'

intent: |
  Users can sign up, log in, and reset passwords.
  Sessions expire after 30 min of inactivity.

behaviors:
  - name: 'signup'
    given: 'valid email + password (>= 12 chars)'
    then: 'account created, welcome email sent'
  - name: 'login'
    given: 'valid credentials'
    then: 'JWT issued, session created'

artifacts:
  code:
    - path: 'src/auth/**'
      role: 'primary implementation'
  docs:
    - path: 'docs/auth.md'
```

## Validate specs

```sh
# Validate all spec files in .notarai/
notarai validate

# Validate a specific file
notarai validate .notarai/auth.spec.yaml

# Validate a directory
notarai validate .notarai/subsystems/
```

Output is `PASS <file>` or `FAIL <file>` with an indented error list. Exit code is 0 if all files pass, 1 if any fail.

## Update NotarAI

Check for and install updates:

```sh
notarai update
```

NotarAI will also print a hint when a newer version is available during `validate` or `init`.

## Bump schema version

When you upgrade to a new version of NotarAI, update all spec files with:

```sh
notarai schema-bump
```

This overwrites `.notarai/notarai.spec.json` with the bundled schema and updates the `schema_version` field in every `.notarai/*.spec.yaml` file.

## Bootstrap from an existing codebase

Use the `/notarai-bootstrap` skill in Claude Code to generate specs from your existing code via a structured developer interview.

## Check for drift

Run `notarai check` to detect structural drift without an LLM:

```sh
# See what's drifted
notarai check

# Strict mode for CI (any finding = exit code 1)
notarai check --strict
```

This reports coverage gaps, orphaned globs, changed files since last reconciliation, overlapping coverage, circular `$ref` chains, and incomplete behaviors. See the [CLI reference](../reference/cli.md#notarai-check) for details.

For automated PR checks, add the [GitHub Action](../reference/github-action.md) to your CI workflow.

## Reconcile with an LLM

Use the `/notarai-reconcile` skill in Claude Code to perform a full semantic reconciliation: detect drift, propose spec/code/doc updates, and walk through each finding interactively.

## Next steps

- **Existing codebase?** See the [Brownfield Adoption Guide](../guides/brownfield-adoption.md) for a step-by-step walkthrough of adding NotarAI to a project that already has code.
- **Not sure how much spec detail you need?** [Progressive Adoption](../guides/progressive-adoption.md) describes three maturity levels so you can start light and add depth where it matters.
