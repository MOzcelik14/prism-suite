#!/usr/bin/env bash
# Copyright (C) 2026 Murat Özçelik <mozcelik14>
# SPDX-License-Identifier: GPL-3.0-only
#
# Packages the prisM. Suite launcher into standardized release archives.
# Usage: ./scripts/package.sh [format]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION="${VERSION:-$(grep '^version' "$ROOT_DIR/Cargo.toml" | head -n1 | cut -d '"' -f2)}"
DIST_DIR="$ROOT_DIR/dist"
FORMAT="${1:-all}"

echo "==> Packaging prisM. Suite v$VERSION (target: $FORMAT)"
mkdir -p "$DIST_DIR"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        # Ensure release binary is compiled
        if [[ ! -f "$ROOT_DIR/target/release/prism" ]]; then
            echo "==> Building release binary with Cargo..."
            cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml" -p prism-app
        fi

        case "$FORMAT" in
            deb|rpm|appimage|tarball)
                bash "$SCRIPT_DIR/package-linux.sh" "$FORMAT"
                ;;
            all)
                echo "==> Building standard Linux distribution formats..."
                bash "$SCRIPT_DIR/package-linux.sh" tarball
                if command -v dpkg-deb >/dev/null 2>&1; then
                    bash "$SCRIPT_DIR/package-linux.sh" deb
                fi
                if command -v rpmbuild >/dev/null 2>&1; then
                    bash "$SCRIPT_DIR/package-linux.sh" rpm
                fi
                ;;
            *)
                echo "Unknown format for Linux: $FORMAT (valid: deb, rpm, appimage, tarball, all)" >&2
                exit 1
                ;;
        esac
        ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "==> Building release binary with Cargo..."
        cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml" -p prism-app

        PKG_NAME="prism-${VERSION}-windows-${ARCH}"
        PKG_DIR="$DIST_DIR/${PKG_NAME}"
        rm -rf "$PKG_DIR"
        mkdir -p "$PKG_DIR"

        cp "$ROOT_DIR/target/release/prism.exe" "$PKG_DIR/"
        cp "$ROOT_DIR/README.md" "$PKG_DIR/"
        cp "$ROOT_DIR/LICENSE" "$PKG_DIR/"
        cp -r "$ROOT_DIR/assets" "$PKG_DIR/"

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

# Generate checksums if helper is present
if [[ -f "$SCRIPT_DIR/sign-release.sh" ]]; then
    "$SCRIPT_DIR/sign-release.sh" "$DIST_DIR" || true
fi

echo "==> Packaging finished in $DIST_DIR"
