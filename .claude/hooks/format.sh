#!/bin/bash
if ! command -v jq &> /dev/null; then
  echo "jq is required but not installed" >&2
  exit 0  # exit 0 so Claude isn't blocked
fi

fp=$(jq -r '.tool_input.file_path')

case "$fp" in
  *.rs)             cargo fmt "$fp" ;;
  *.json|*.js|*.ts) biome format --write "$fp" ;;
  *.md)             npx prettier --write "$fp" ;;
esac
