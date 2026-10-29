#!/usr/bin/env bash
# Copyright (C) 2026 Murat Özçelik <mozcelik14>
# SPDX-License-Identifier: GPL-3.0-only
#
# Generates SHA256SUMS and signs it with Minisign (Ed25519) for prisM. Suite releases.

set -euo pipefail

TARGET_DIR="${1:-dist}"
KEY_FILE="${2:-${MINISIGN_SECRET_KEY_PATH:-prism.key}}"
PASSPHRASE="${MINISIGN_PASSPHRASE:-}"

if [[ ! -d "$TARGET_DIR" ]]; then
    echo "Error: Directory '$TARGET_DIR' does not exist." >&2
    exit 1
fi

echo "==> prisM. Suite Release Signer"
echo "Target directory : $TARGET_DIR"

# 1. Compute SHA-256 Checksums
echo "==> Generating SHA256SUMS..."
cd "$TARGET_DIR"

if command -v sha256sum >/dev/null 2>&1; then
    sha256sum *.tar.gz *.zip 2>/dev/null > SHA256SUMS || true
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 *.tar.gz *.zip 2>/dev/null > SHA256SUMS || true
else
    echo "Error: Neither sha256sum nor shasum is available." >&2
    exit 1
fi

if [[ ! -s "SHA256SUMS" ]]; then
    echo "Warning: No .tar.gz or .zip archive assets found in '$TARGET_DIR'." >&2
    exit 1
fi

echo "Generated checksums for:"
cat SHA256SUMS

# 2. Minisign Signature
if ! command -v minisign >/dev/null 2>&1; then
    echo "Warning: 'minisign' is not installed. SHA256SUMS generated, but skipping signing."
    echo "Install minisign (e.g. 'sudo apt install minisign' or 'cargo install minisign') to sign releases."
    exit 0
fi

if [[ ! -f "$KEY_FILE" ]] && [[ ! -f "../$KEY_FILE" ]]; then
    echo "Notice: Secret key '$KEY_FILE' not found. Skipping Minisign signature."
    exit 0
fi

RESOLVED_KEY="$KEY_FILE"
if [[ ! -f "$RESOLVED_KEY" ]]; then
    RESOLVED_KEY="../$KEY_FILE"
fi

echo "==> Signing SHA256SUMS with Minisign key '$RESOLVED_KEY'..."
rm -f SHA256SUMS.sig

if [[ -n "$PASSPHRASE" ]]; then
    echo "$PASSPHRASE" | minisign -S -s "$RESOLVED_KEY" -m SHA256SUMS -x SHA256SUMS.sig -p prism.pub 2>/dev/null || \
    echo "$PASSPHRASE" | minisign -S -s "$RESOLVED_KEY" -m SHA256SUMS -x SHA256SUMS.sig
else
    minisign -S -s "$RESOLVED_KEY" -m SHA256SUMS -x SHA256SUMS.sig
fi

echo "==> Successfully created SHA256SUMS.sig"
