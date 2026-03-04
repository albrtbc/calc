#!/bin/sh
set -e

REPO="albrtbc/calc"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect OS
case "$(uname -s)" in
    Linux*)  OS="linux" ;;
    Darwin*) OS="macos" ;;
    *)       echo "Unsupported OS: $(uname -s)"; exit 1 ;;
esac

# Detect architecture
case "$(uname -m)" in
    x86_64|amd64)  ARCH="amd64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *)             echo "Unsupported architecture: $(uname -m)"; exit 1 ;;
esac

ARTIFACT="calc-${OS}-${ARCH}"
echo "Downloading ${ARTIFACT}..."

# Get latest release tag
TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$TAG" ]; then
    echo "Error: could not find latest release"; exit 1
fi
echo "Latest release: ${TAG}"

URL="https://github.com/${REPO}/releases/download/${TAG}/${ARTIFACT}.tar.gz"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$URL" -o "${TMPDIR}/${ARTIFACT}.tar.gz"
tar xzf "${TMPDIR}/${ARTIFACT}.tar.gz" -C "$TMPDIR"

if [ -w "$INSTALL_DIR" ]; then
    mv "${TMPDIR}/calc" "${INSTALL_DIR}/calc"
else
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "${TMPDIR}/calc" "${INSTALL_DIR}/calc"
fi

echo "calc installed to ${INSTALL_DIR}/calc"
calc --version 2>/dev/null || true
