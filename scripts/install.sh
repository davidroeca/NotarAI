#!/bin/sh
# NotarAI installer — downloads the appropriate binary from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/davidroeca/NotarAI/main/scripts/install.sh | sh
#
# Environment variables:
#   VERSION      — release tag to install (default: latest)
#   INSTALL_DIR  — installation directory (default: /usr/local/bin)

set -e

REPO="davidroeca/NotarAI"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect OS
OS="$(uname -s)"
case "$OS" in
  Linux)  os="linux" ;;
  Darwin) os="macos" ;;
  *)
    echo "Error: unsupported OS: $OS" >&2
    exit 1
    ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)  arch="x86_64" ;;
  aarch64|arm64)  arch="aarch64" ;;
  *)
    echo "Error: unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

# Choose binary variant: musl for Linux (static), plain for macOS
if [ "$os" = "linux" ]; then
  binary="notarai-${arch}-linux-musl"
else
  binary="notarai-${arch}-macos"
fi

# Determine version
if [ -z "$VERSION" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')"
fi

if [ -z "$VERSION" ]; then
  echo "Error: could not determine latest version" >&2
  exit 1
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/${binary}"

echo "Installing notarai ${VERSION} (${binary}) to ${INSTALL_DIR}..."

# Download
tmpfile="$(mktemp)"
trap 'rm -f "$tmpfile"' EXIT

if ! curl -fsSL -o "$tmpfile" "$URL"; then
  echo "Error: download failed from $URL" >&2
  exit 1
fi

chmod +x "$tmpfile"

# Install
if [ -w "$INSTALL_DIR" ]; then
  mv "$tmpfile" "${INSTALL_DIR}/notarai"
else
  sudo mv "$tmpfile" "${INSTALL_DIR}/notarai"
fi

echo "notarai installed to ${INSTALL_DIR}/notarai"
notarai --version 2>/dev/null || true
