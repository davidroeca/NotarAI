# NotarAI

NotarAI is a continuous intent reconciliation tool that keeps specs, code, and docs aligned.

## Specs

Specs live in `.notarai/*.spec.yaml` and are the canonical source of truth.
The JSON Schema is at `.notarai/notarai.spec.json`.

## Validation

Run `notarai validate .notarai/` to validate all specs against the schema.

## Drift Detection

Run `notarai check` for deterministic, LLM-free drift analysis:

- Coverage gaps (files not governed by any spec)
- Orphaned globs (spec patterns matching no files)
- Changed files since last reconciliation
- Overlapping coverage across specs

Run `notarai check --format json` for machine-readable output.

## Reconciliation

Run `notarai export-context --all --base-branch main --format markdown` to generate
a self-contained reconciliation prompt. Paste the output into your preferred LLM agent
to get a drift analysis and alignment recommendations.

For a single spec:

```
notarai export-context --spec .notarai/<name>.spec.yaml --base-branch main
```

## MCP Server

NotarAI includes an MCP server for tool-integrated reconciliation:

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

Available tools: `list_affected_specs`, `get_spec_diff`, `get_changed_artifacts`,
`mark_reconciled`, `clear_cache`, `snapshot_state`.
