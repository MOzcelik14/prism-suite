// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use crate::NetError;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// Verifies that data matches its entry in SHA256SUMS.
pub fn verify_sha256sums(
    archive_filename: &str,
    archive_bytes: &[u8],
    sha256sums_content: &str,
) -> Result<(), NetError> {
    let clean_target = archive_filename.trim();

    for line in sha256sums_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Format is "<hash>  <filename>" or "<hash> *<filename>" or "<hash> <filename>"
        let mut parts = trimmed.split_whitespace();
        let expected_hash = match parts.next() {
            Some(h) => h,
            None => continue,
        };
        let listed_filename = match parts.next() {
            Some(f) => f.trim_start_matches('*'),
            None => continue,
        };

        if listed_filename == clean_target {
            let mut hasher = Sha256::new();
            hasher.update(archive_bytes);
            let actual_hash = format!("{:x}", hasher.finalize());

            if actual_hash.eq_ignore_ascii_case(expected_hash) {
                return Ok(());
            } else {
                return Err(NetError::ChecksumMismatch {
                    expected: expected_hash.to_string(),
                    actual: actual_hash,
                });
            }
        }
    }

    Err(NetError::ChecksumMismatch {
        expected: format!("Entry for '{}' in SHA256SUMS", clean_target),
        actual: "Not found in manifest".to_string(),
    })
}

/// Extracts a 64-byte Ed25519 signature from either Minisign format or raw base64/hex string.
pub fn parse_signature_bytes(sig_str: &str) -> Result<[u8; 64], NetError> {
    let lines: Vec<&str> = sig_str
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    // Check Minisign format (Line 1: comment, Line 2: base64, ...)
    let candidate = if lines.len() >= 2 && lines[0].starts_with("untrusted comment") {
        lines[1]
    } else if !lines.is_empty() {
        lines[0]
    } else {
        return Err(NetError::SignatureInvalid("Empty signature".to_string()));
    };

    // Try base64 decoding
    if let Ok(decoded) = BASE64.decode(candidate) {
        if decoded.len() == 64 {
            let mut sig = [0u8; 64];
            sig.copy_from_slice(&decoded);
            return Ok(sig);
        } else if decoded.len() == 74 {
            // Minisign line 2: 2 bytes untrusted ID + 8 bytes key ID + 64 bytes signature
            let mut sig = [0u8; 64];
            sig.copy_from_slice(&decoded[10..74]);
            return Ok(sig);
        }
    }

    // Try hex decoding
    if candidate.len() == 128 {
        if let Ok(bytes) = (0..128)
            .step_by(2)
            .map(|i| u8::from_str_radix(&candidate[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
        {
            if bytes.len() == 64 {
                let mut sig = [0u8; 64];
                sig.copy_from_slice(&bytes);
                return Ok(sig);
            }
        }
    }

    Err(NetError::SignatureInvalid(
        "Could not parse 64-byte Ed25519 signature from signature file".to_string(),
    ))
}

/// Verifies that signature matches data signed by the specified public key.
pub fn verify_ed25519_signature(
    data: &[u8],
    sig_content: &str,
    public_key: &VerifyingKey,
) -> Result<(), NetError> {
    let sig_bytes = parse_signature_bytes(sig_content)?;
    let signature = Signature::from_bytes(&sig_bytes);

    public_key
        .verify(data, &signature)
        .map_err(|e| NetError::SignatureInvalid(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn test_verify_sha256sums() {
        let content = b"hello prism release";
        let manifest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  other.zip\n\
                        8ea8a912cfbf6b45d569a100173c322937a600fdec7ec365c79cba6c6805259b  test-0.1.0.tar.gz\n";

        // Calculated sha256 for "hello prism release"
        let mut hasher = Sha256::new();
        hasher.update(content);
        let actual = format!("{:x}", hasher.finalize());
        assert_eq!(
            actual,
            "8ea8a912cfbf6b45d569a100173c322937a600fdec7ec365c79cba6c6805259b"
        );

        assert!(verify_sha256sums("test-0.1.0.tar.gz", content, manifest).is_ok());
        assert!(verify_sha256sums("missing.tar.gz", content, manifest).is_err());
        assert!(verify_sha256sums("test-0.1.0.tar.gz", b"corrupted content", manifest).is_err());
    }

    #[test]
    fn test_verify_ed25519_minisign_and_raw() {
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let message = b"SHA256SUMS manifest content to sign";
        let signature = signing_key.sign(message);

        // 1. Raw base64 signature
        let raw_b64 = BASE64.encode(signature.to_bytes());
        assert!(verify_ed25519_signature(message, &raw_b64, &verifying_key).is_ok());

        // 2. Minisign format with 74-byte header
        let mut minisign_bytes = Vec::new();
        minisign_bytes.extend_from_slice(b"Ed");
        minisign_bytes.extend_from_slice(&[1u8; 8]); // Key ID
        minisign_bytes.extend_from_slice(&signature.to_bytes());
        let minisign_b64 = BASE64.encode(&minisign_bytes);

        let minisign_file = format!(
            "untrusted comment: signature from prism secret key\n{}\ntrusted comment: timestamp:1793262000\n{}",
            minisign_b64, minisign_b64
        );

        assert!(verify_ed25519_signature(message, &minisign_file, &verifying_key).is_ok());

        // 3. Tampered data fails
        assert!(
            verify_ed25519_signature(b"tampered message", &minisign_file, &verifying_key).is_err()
        );
    }
}
