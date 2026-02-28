---
paths:
  - 'src/**/*.rs'
---

## Rust

- **Batch database operations** -- Wrap loops over DB calls in a single transaction with a prepared statement, rather than implicit per-call transactions.
- **Order by cost** -- When a function tries multiple strategies, put the cheapest one first (e.g., check a known path before scanning a directory).
- **Binary crate visibility** -- No external consumers exist, so `pub` only means cross-module. Gate test-only helpers with `#[cfg(test)]` instead of `pub`.
- **Avoid needless allocations** -- Prefer borrowing (`&str`, `&Value`) over `.to_string()` / `.cloned()` when the owned value isn't needed.
- **Avoid `.unwrap()` in production code** -- Use `?`, `.expect("reason")`, or combinators (`.unwrap_or`, `.map`). `.unwrap()` is acceptable in `#[test]` functions and `#[cfg(test)]` modules.

> Also enforced as constraints in `cli.spec.yaml` for retroactive drift detection.
