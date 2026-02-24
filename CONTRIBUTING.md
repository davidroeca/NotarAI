# Contributing to NotarAI

Your interest in contributing to this project is appreciated. Below is a series of instructions that will hopefully remain up to date because this tool should help manage that. However, if you notice that the steps seem out of date or misaligned with current practices in the repo, an update to this document could be a and high-value first or second contribution to the project.

Note that the project's own spec drift is self-managed, so please get acquainted with the tool and make sure your contributions stay in sync.

## Development Setup

Install [Rust](https://www.rust-lang.org/tools/install) (stable toolchain) and [Node.js](https://nodejs.org/) (for prettier and docs-site).

```sh
git clone https://github.com/davidroeca/NotarAI.git
cd NotarAI
cargo build
npm ci
cargo install --path .
```

The last step installs the `notarai` binary to `~/.cargo/bin` so the Claude Code
hook (`notarai hook validate`) resolves correctly. Re-run it whenever you want
the installed binary to reflect your latest local changes.

## Making Changes

1. Create a branch from `main`
2. Make your changes
3. Run `cargo build` to verify compilation
4. Run `cargo test` to run the test suite
5. Run `cargo fmt --check` to verify formatting
6. Run `cargo clippy -- -D warnings` to check for lint issues
7. Use the `/notarai-reconcile` Claude Code command or use your favorite coding agent to follow these reconciliation instructions
8. Open a pull request

## Code Style

- Rust 2021 edition
- `cargo fmt` for Rust formatting
- `cargo clippy` for Rust lints
- `npx prettier --check .` for non-Rust file formatting (markdown, JSON, etc.)
- Functional style preferred over excessive use of structs with methods
- Core library lives in `src/core/` (not `src/lib/` due to Rust's reserved module name)

## Project Structure

See `CLAUDE.md` for a detailed layout and architectural constraints.

## Good First Contributions

These changes will drive broader adoption but are not yet a priority:

- Support other coding agents (e.g. Codex, Aider, Cline, OpenHands, Goose, opencode)
- Optimize/limit token usage with minimal quality loss

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
