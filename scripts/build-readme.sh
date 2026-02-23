#!/usr/bin/env bash
# Builds a npm-compatible README with mermaid diagrams rendered as SVGs.
# GitHub renders mermaid natively, but npm does not — this script converts
# mermaid code blocks to inline SVG image references for the published package.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cp "$ROOT_DIR/README.md" "$ROOT_DIR/dist/README.md"

npx --yes mmdc -i "$ROOT_DIR/dist/README.md" -o "$ROOT_DIR/dist/README.md" --outputFormat svg

echo "README built with rendered mermaid diagrams in dist/"
