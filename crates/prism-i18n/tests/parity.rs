// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use fluent_bundle::FluentResource;
use std::collections::BTreeSet;

const EN_FTL: &str = include_str!("../locales/en.ftl");
const TR_FTL: &str = include_str!("../locales/tr.ftl");

fn extract_keys(ftl: &str) -> BTreeSet<String> {
    // Validate FTL grammar using FluentResource
    let _ = FluentResource::try_new(ftl.to_string()).expect("valid FTL syntax");

    let mut keys = BTreeSet::new();
    for line in ftl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = trimmed.split_once('=') {
            keys.insert(key.trim().to_string());
        }
    }
    keys
}

#[test]
fn test_strict_fluent_key_parity() {
    let en_keys = extract_keys(EN_FTL);
    let tr_keys = extract_keys(TR_FTL);

    assert!(
        !en_keys.is_empty(),
        "en.ftl should not be empty, found 0 keys"
    );
    assert!(
        !tr_keys.is_empty(),
        "tr.ftl should not be empty, found 0 keys"
    );

    let missing_in_tr: Vec<_> = en_keys.difference(&tr_keys).cloned().collect();
    let missing_in_en: Vec<_> = tr_keys.difference(&en_keys).cloned().collect();

    assert!(
        missing_in_tr.is_empty(),
        "Keys missing in tr.ftl: {:?}",
        missing_in_tr
    );
    assert!(
        missing_in_en.is_empty(),
        "Keys missing in en.ftl: {:?}",
        missing_in_en
    );

    assert_eq!(
        en_keys, tr_keys,
        "en.ftl and tr.ftl must have 100% identical keys"
    );
}

#[test]
fn test_turkish_characters_presence() {
    // Verify that Turkish characters are properly present in tr.ftl
    let turkish_chars = ['ç', 'ğ', 'ı', 'İ', 'ö', 'ş', 'ü'];
    for ch in turkish_chars {
        assert!(
            TR_FTL.contains(ch),
            "Expected tr.ftl to contain Turkish character '{}'",
            ch
        );
    }
}
