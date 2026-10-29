// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use base64::Engine;

use ed25519_dalek::{Signer, SigningKey};
use flate2::write::GzEncoder;
use flate2::Compression;
use prism_core::catalog::AppCatalog;
use prism_core::AppId;
use prism_install::{extract_archive, ArchiveError, InstallationManager};
use prism_net::{verify_ed25519_signature, verify_sha256sums, NetError, NetworkClient};
use tar::Builder;

/// Helper to generate a unique temporary test root directory.
fn create_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("prism-test-fixtures").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Helper to generate a valid in-memory .tar.gz archive fixture.
fn create_valid_tar_gz(app_binary: &str, version: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar = Builder::new(&mut encoder);

        // Add version.txt
        let mut header = tar::Header::new_gnu();
        header.set_size(version.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "version.txt", version.as_bytes())
            .unwrap();

        // Add binary
        let dummy_bin = b"#!/bin/sh\necho test\n";
        let mut bin_header = tar::Header::new_gnu();
        bin_header.set_size(dummy_bin.len() as u64);
        bin_header.set_mode(0o755);
        bin_header.set_cksum();
        tar.append_data(&mut bin_header, app_binary, &dummy_bin[..])
            .unwrap();

        tar.finish().unwrap();
    }
    encoder.finish().unwrap()
}

/// Helper to generate a malicious path traversal archive fixture.
fn create_malicious_path_traversal_archive() -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar = Builder::new(&mut encoder);
        let evil_payload = b"evil content";
        let mut header = tar::Header::new_gnu();
        header.set_size(evil_payload.len() as u64);
        header.set_mode(0o644);
        let name_bytes = b"../../escape.txt\0";
        header.as_mut_bytes()[..name_bytes.len()].copy_from_slice(name_bytes);
        header.set_cksum();
        tar.append(&header, &evil_payload[..]).unwrap();
        tar.finish().unwrap();
    }
    encoder.finish().unwrap()
}

#[test]
fn test_fixture_1_valid_install() {
    let test_dir = create_test_dir("valid_install");
    let mgr = InstallationManager::new(Some(test_dir.clone()));
    let catalog = AppCatalog::load_embedded();
    let trim = catalog.get(AppId::Trim).unwrap();

    let archive = create_valid_tar_gz("trim", "0.1.0");

    mgr.install_package(trim, "0.1.0", "trim-0.1.0-linux-x86_64.tar.gz", &archive, 2)
        .expect("Valid installation should succeed");

    // Check version folder exists
    let ver_dir = mgr.version_dir(trim, "0.1.0").unwrap();
    assert!(ver_dir.exists());

    // Check current symlink exists and points to 0.1.0
    let cur_link = mgr.current_link_path(trim).unwrap();
    assert!(cur_link.exists());
    let target = fs::read_link(&cur_link).unwrap();
    assert_eq!(target, ver_dir);

    // Check detected state is Installed
    let state = mgr.detect_state(trim);
    assert_eq!(state.installed_version(), Some("0.1.0"));

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_fixture_2_bad_hash() {
    let archive_bytes = create_valid_tar_gz("trim", "0.1.0");
    let sha256sums_content = "0000000000000000000000000000000000000000000000000000000000000000  trim-0.1.0-linux-x86_64.tar.gz\n";

    let result = verify_sha256sums(
        "trim-0.1.0-linux-x86_64.tar.gz",
        &archive_bytes,
        sha256sums_content,
    );
    assert!(result.is_err(), "Bad hash should be rejected");

    match result.unwrap_err() {
        NetError::ChecksumMismatch { expected, .. } => {
            assert!(expected.contains("0000000000000000"));
        }
        other => panic!("Expected ChecksumMismatch, got {:?}", other),
    }
}

#[test]
fn test_fixture_3_bad_signature() {
    let key1 = SigningKey::from_bytes(&[1u8; 32]);
    let key2 = SigningKey::from_bytes(&[2u8; 32]);
    let data = b"SHA256SUMS manifest";

    // Sign with key1
    let sig = key1.sign(data);
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

    // Verify against key2 (wrong key)
    let result = verify_ed25519_signature(data, &sig_b64, &key2.verifying_key());
    assert!(result.is_err(), "Wrong signature should be rejected");
}

#[test]
fn test_fixture_4_path_traversal_archive() {
    let test_dir = create_test_dir("path_traversal");
    let evil_archive = create_malicious_path_traversal_archive();

    let result = extract_archive("evil.tar.gz", &evil_archive, &test_dir);
    assert!(result.is_err(), "Path traversal archive must be rejected");

    match result.unwrap_err() {
        ArchiveError::PathTraversal(path) => {
            assert!(path.contains("escape.txt"));
        }
        other => panic!("Expected PathTraversal error, got {:?}", other),
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_fixture_5_interrupted_download() {
    let client = NetworkClient::new().unwrap();
    let cancel_flag = Arc::new(AtomicBool::new(true)); // Already cancelled

    let result = client
        .download_with_progress(
            "https://raw.githubusercontent.com/MOzcelik14/steM/main/Cargo.toml",
            Some(cancel_flag),
            |_down, _total, _prog| {},
        )
        .await;

    assert!(
        result.is_err(),
        "Interrupted download should return Cancelled"
    );
    match result.unwrap_err() {
        NetError::Cancelled => {}
        other => panic!("Expected Cancelled error, got {:?}", other),
    }
}

#[test]
fn test_fixture_6_rollback() {
    let test_dir = create_test_dir("rollback");
    let mgr = InstallationManager::new(Some(test_dir.clone()));
    let catalog = AppCatalog::load_embedded();
    let trim = catalog.get(AppId::Trim).unwrap();

    // 1. Install version 0.1.0
    let archive_v1 = create_valid_tar_gz("trim", "0.1.0");
    mgr.install_package(trim, "0.1.0", "trim-0.1.0.tar.gz", &archive_v1, 3)
        .unwrap();
    assert_eq!(mgr.detect_state(trim).installed_version(), Some("0.1.0"));

    // 2. Update to version 0.2.0
    let archive_v2 = create_valid_tar_gz("trim", "0.2.0");
    mgr.install_package(trim, "0.2.0", "trim-0.2.0.tar.gz", &archive_v2, 3)
        .unwrap();
    assert_eq!(mgr.detect_state(trim).installed_version(), Some("0.2.0"));
    assert_eq!(mgr.detect_previous_version(trim), Some("0.1.0".to_string()));

    // 3. Rollback to version 0.1.0
    let restored = mgr.rollback(trim).expect("Rollback should succeed");
    assert_eq!(restored, "0.1.0");

    // 4. Verify current version is 0.1.0
    assert_eq!(mgr.detect_state(trim).installed_version(), Some("0.1.0"));

    let _ = fs::remove_dir_all(&test_dir);
}
