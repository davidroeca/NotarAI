---
paths:
  - 'src/**/*.rs'
  - 'tests/**/*.rs'
---

## Tests

Tests use `cargo test`. Unit tests are inline `#[cfg(test)]` modules in source files. Integration tests in `tests/` use `assert_cmd` for CLI binary testing and `tempfile` for isolated init tests.

> Test artifacts are governed by `cli.spec.yaml`.
