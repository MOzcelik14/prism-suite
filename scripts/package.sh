#!/usr/bin/env bash
# Copyright (C) 2026 Murat Özçelik <mozcelik14>
# SPDX-License-Identifier: GPL-3.0-only
#
# Packages the prisM. Suite launcher into standardized release archives.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION="${1:-$(grep '^version' "$ROOT_DIR/Cargo.toml" | head -n1 | cut -d '"' -f2)}"
DIST_DIR="$ROOT_DIR/dist"

echo "==> Packaging prisM. Suite v$VERSION"
mkdir -p "$DIST_DIR"

# Build release binary
echo "==> Building release binary with Cargo..."
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        PKG_NAME="prism-${VERSION}-linux-${ARCH}"
        PKG_DIR="$DIST_DIR/${PKG_NAME}"
        rm -rf "$PKG_DIR"
        mkdir -p "$PKG_DIR"

        cp "$ROOT_DIR/target/release/prism" "$PKG_DIR/"
        cp "$ROOT_DIR/README.md" "$PKG_DIR/"
        cp "$ROOT_DIR/LICENSE" "$PKG_DIR/"

        TAR_FILE="$DIST_DIR/${PKG_NAME}.tar.gz"
        echo "==> Creating tarball $TAR_FILE..."
        tar -czf "$TAR_FILE" -C "$DIST_DIR" "$PKG_NAME"
        rm -rf "$PKG_DIR"
        echo "==> Packaged: $TAR_FILE"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        PKG_NAME="prism-${VERSION}-windows-${ARCH}"
        PKG_DIR="$DIST_DIR/${PKG_NAME}"
        rm -rf "$PKG_DIR"
        mkdir -p "$PKG_DIR"

        cp "$ROOT_DIR/target/release/prism.exe" "$PKG_DIR/"
        cp "$ROOT_DIR/README.md" "$PKG_DIR/"
        cp "$ROOT_DIR/LICENSE" "$PKG_DIR/"

        ZIP_FILE="$DIST_DIR/${PKG_NAME}.zip"
        echo "==> Creating zip $ZIP_FILE..."
        (cd "$DIST_DIR" && zip -r "${PKG_NAME}.zip" "$PKG_NAME")
        rm -rf "$PKG_DIR"
        echo "==> Packaged: $ZIP_FILE"
        ;;
    *)
        echo "Unsupported OS: $OS" >&2
        exit 1
        ;;
esac

# Generate checksums
"$SCRIPT_DIR/sign-release.sh" "$DIST_DIR" || true

echo "==> Packaging completed successfully in $DIST_DIR"
