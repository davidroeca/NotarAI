# Quick Start

## Initialize your project

Run `notarai init` in your project root:

```sh
notarai init
```

This does several things:

1. Adds a **PostToolUse hook** to `.claude/settings.json` so spec files are automatically validated when Claude Code writes or edits them.
2. Copies the `/notarai-reconcile` slash command to `.claude/commands/` for drift detection.
3. Copies the `/notarai-bootstrap` slash command to `.claude/commands/` for bootstrapping specs from an existing codebase.
4. Copies `notarai.spec.json` to `.notarai/notarai.spec.json` so Claude has the schema available in every session.
5. Writes `.notarai/README.md` with workflow instructions.
6. Replaces the `## NotarAI` section in `CLAUDE.md` with `@`-imports of the schema and README.
7. Appends `.notarai/.cache/` to `.gitignore` so the hash cache DB is never committed.
8. Writes `.mcp.json` registering `notarai mcp` as a local MCP server, so [MCP-accelerated reconciliation](../reference/mcp-server.md) works out of the box.

Running `init` again is safe: it always refreshes slash commands and the schema copy, and replaces the `## NotarAI` section in CLAUDE.md with the current content.

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
schema_version: '0.5'

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

## Bump schema version

When you upgrade to a new version of NotarAI, update all spec files with:

```sh
notarai schema-bump
```

This overwrites `.notarai/notarai.spec.json` with the bundled schema and updates the `schema_version` field in every `.notarai/*.spec.yaml` file.

## Bootstrap from an existing codebase

Use the `/notarai-bootstrap` slash command in Claude Code to generate specs from your existing code via a structured developer interview.

## Detect drift

Use the `/notarai-reconcile` slash command in Claude Code to detect drift between specs and code, and propose aligned updates.
