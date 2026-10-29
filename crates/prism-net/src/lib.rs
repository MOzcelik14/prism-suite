use ed25519_dalek::VerifyingKey;
use prism_core::error::PrismError;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Compile-time configurable Ed25519 public key for release signature verification.
/// In production, set `PRISM_PUBLIC_KEY=<hex-or-base64>` at build time.
pub const DEFAULT_ED25519_PUBLIC_KEY_HEX: &str = match option_env!("PRISM_PUBLIC_KEY") {
    Some(val) => val,
    None => "0000000000000000000000000000000000000000000000000000000000000000",
};

#[derive(Debug, Error)]
pub enum NetError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Checksum mismatch: expected {expected}, calculated {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Cryptographic signature verification failed: {0}")]
    SignatureInvalid(String),

    #[error("Invalid public key format: {0}")]
    InvalidPublicKey(String),
}

impl From<NetError> for PrismError {
    fn from(err: NetError) -> Self {
        match err {
            NetError::Http(e) => PrismError::Network(e.to_string()),
            NetError::ChecksumMismatch { .. } | NetError::SignatureInvalid(_) => {
                PrismError::Security(err.to_string())
            }
            NetError::InvalidPublicKey(msg) => PrismError::Security(msg),
        }
    }
}

/// Helper function to parse hex string into bytes without external hex crate.
pub fn parse_hex(hex_str: &str) -> Result<Vec<u8>, NetError> {
    let clean = hex_str.trim();
    if !clean.len().is_multiple_of(2) {
        return Err(NetError::InvalidPublicKey(
            "Hex string length must be even".to_string(),
        ));
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16)
                .map_err(|e| NetError::InvalidPublicKey(format!("Invalid hex character: {}", e)))
        })
        .collect()
}

/// Network client for fetching releases and update manifests.
pub struct NetworkClient {
    client: reqwest::Client,
}

impl NetworkClient {
    pub fn new() -> Result<Self, NetError> {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "prisM-Suite/",
                env!("CARGO_PKG_VERSION"),
                " (desktop)"
            ))
            .build()?;
        Ok(Self { client })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Verifies that data matches an expected SHA-256 hash (hex string).
    pub fn verify_sha256(data: &[u8], expected_hex: &str) -> Result<(), NetError> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual = format!("{:x}", hasher.finalize());
        if actual.eq_ignore_ascii_case(expected_hex.trim()) {
            Ok(())
        } else {
            Err(NetError::ChecksumMismatch {
                expected: expected_hex.to_string(),
                actual,
            })
        }
    }

    /// Parses the public key configured for release verification.
    pub fn public_key() -> Result<VerifyingKey, NetError> {
        let bytes = parse_hex(DEFAULT_ED25519_PUBLIC_KEY_HEX)?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| NetError::InvalidPublicKey("Public key must be 32 bytes".to_string()))?;

        VerifyingKey::from_bytes(&key_bytes).map_err(|e| NetError::InvalidPublicKey(e.to_string()))
    }
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new().expect("failed to create default NetworkClient")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_verification() {
        let data = b"hello prism suite";
        let expected = "1b8fdfe9172d93c30f2012f7bb6827d8113d75cf88ec5c2b41cac5b4b087b6af";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let calculated = format!("{:x}", hasher.finalize());
        assert_eq!(calculated, expected);

        assert!(NetworkClient::verify_sha256(data, expected).is_ok());
        assert!(NetworkClient::verify_sha256(data, "invalidchecksum").is_err());
    }

    #[test]
    fn test_parse_hex() {
        let bytes = parse_hex("deadbeef").unwrap();
        assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);
    }
}
