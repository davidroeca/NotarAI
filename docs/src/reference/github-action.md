# GitHub Action

NotarAI provides a composite GitHub Action that runs `notarai check` on pull requests and posts a summary comment. No Rust toolchain is required on the runner.

## Setup

Add a workflow file to your repository:

```yaml
# .github/workflows/notarai.yml
name: NotarAI Check
on:
  pull_request:
    branches: [main]

permissions:
  contents: read
  pull-requests: write

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: davidroeca/NotarAI/crates/notarai-action@v0.7.0
```

The action downloads the `notarai` binary from GitHub Releases, runs the check, and posts (or updates) a PR comment with the results.

## Inputs

| Input         | Default  | Description                                        |
| ------------- | -------- | -------------------------------------------------- |
| `version`     | `latest` | NotarAI version to install                         |
| `base-branch` | `main`   | Branch to diff against for changed-file detection  |
| `strict`      | `false`  | Promote all warnings to errors (fail on any drift) |
| `comment`     | `true`   | Post a PR comment with findings                    |

## What it does

1. **Detects platform**: determines runner OS and architecture.
2. **Downloads binary**: fetches the matching `notarai` release binary from GitHub Releases.
3. **Runs check**: executes `notarai check --format json --base-branch <base-branch>` (with `--strict` if enabled).
4. **Posts comment**: renders findings into a Markdown comment grouped by type, with collapsible details.
5. **Sets exit code**: fails the step if any error-severity findings are present.

## PR comment

The comment includes a summary line and collapsible sections for each finding type:

```
## NotarAI Drift Check

113 finding(s) | 0 error(s) | 113 warning(s) -- 113 warning(s)

> Orphaned globs (1)
>   - Artifact glob matches no files: src/legacy/** (in legacy.spec.yaml)

> Coverage gaps (3)
>   - File not governed by any spec: src/new_module.rs
>   - ...
```

Re-runs update the existing comment in place rather than posting duplicates. The comment is identified by a `<!-- notarai-action -->` HTML marker.

## Strict mode

Use `strict: 'true'` to fail the check on any finding, not just errors:

```yaml
- uses: davidroeca/NotarAI/crates/notarai-action@v0.7.0
  with:
    strict: 'true'
```

This is useful for repositories that want zero-tolerance drift detection in CI.

## Uninitialized repositories

If the repository does not have a `.notarai/` directory, the action posts a comment noting that NotarAI is not initialized and exits successfully (does not fail the workflow).

## Requirements

- Runs on `ubuntu-latest` (Linux x86_64 or aarch64).
- No Rust toolchain required on the runner.
- Requires `pull-requests: write` permission for posting comments.
- Uses `GITHUB_TOKEN` (automatic) for downloading releases and posting comments.
