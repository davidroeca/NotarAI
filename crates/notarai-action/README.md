# NotarAI Drift Check Action

A GitHub Action that runs [NotarAI](https://github.com/davidroeca/NotarAI)
deterministic drift detection on pull requests and posts a summary comment.

## Usage

```yaml
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
      - uses: davidroeca/NotarAI/crates/notarai-action@v0.6.0
```

## Inputs

| Input         | Default  | Description                     |
| ------------- | -------- | ------------------------------- |
| `version`     | `latest` | NotarAI version to install      |
| `base-branch` | `main`   | Branch to diff against          |
| `strict`      | `false`  | Promote warnings to errors      |
| `comment`     | `true`   | Post a PR comment with findings |

## What it does

1. Downloads the `notarai` binary matching the runner platform from GitHub
   Releases.
2. Runs `notarai check --format json` to detect drift.
3. Posts (or updates) a PR comment summarizing findings.
4. Fails the workflow step if any error-severity findings are present.

## PR comment

The action posts a collapsible Markdown comment grouped by finding type.
Re-runs update the existing comment in place (identified by a
`<!-- notarai-action -->` HTML marker).

## Requirements

- Runs on `ubuntu-latest` (Linux x86_64 or aarch64).
- No Rust toolchain needed on the runner.
- Requires `pull-requests: write` permission for comment posting.
