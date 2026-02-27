---
title: Installation
description: How to install the NotarAI CLI.
---

## Quick Install (Linux / macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/davidroeca/NotarAI/main/scripts/install.sh | sh
```

This detects your OS and architecture, downloads the appropriate binary from
GitHub Releases, and installs it to `~/.local/bin`. If that directory is not
in your `PATH`, the script will print a one-line export command to add it.

## From crates.io

If you have Rust installed:

```sh
cargo install notarai
```

## Manual Download

Download the binary for your platform from the
[latest release](https://github.com/davidroeca/NotarAI/releases/latest):

| Platform                      | Binary                       |
| ----------------------------- | ---------------------------- |
| Linux x86_64 (glibc)          | `notarai-x86_64-linux-gnu`   |
| Linux x86_64 (musl)           | `notarai-x86_64-linux-musl`  |
| Linux aarch64 (glibc)         | `notarai-aarch64-linux-gnu`  |
| Linux aarch64 (musl)          | `notarai-aarch64-linux-musl` |
| macOS x86_64                  | `notarai-x86_64-macos`       |
| macOS aarch64 (Apple Silicon) | `notarai-aarch64-macos`      |
| Windows x86_64                | `notarai-x86_64-windows.exe` |

Make the binary executable and move it to a directory in your `PATH`:

```sh
chmod +x notarai-*
mkdir -p ~/.local/bin
mv notarai-* ~/.local/bin/notarai
```

If `~/.local/bin` is not already in your `PATH`, add this to your shell profile
(`~/.bashrc`, `~/.zshrc`, etc.):

```sh
export PATH="$HOME/.local/bin:$PATH"
```

## From Source

```sh
git clone https://github.com/davidroeca/NotarAI
cd NotarAI
cargo build --release
# Binary is at target/release/notarai
```

## Requirements

- No runtime dependencies — NotarAI is a single static binary
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) for reconciliation features (optional for validation-only usage)
