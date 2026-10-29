# Shared Preferences Specification (`shared.ron`)

## Overview
**prisM. Suite** manages suite-wide preferences across its family of creative applications:
- **triM.** (video editor)
- **steM.** (audio workstation)
- **beaM.** (photo editor)

When a user modifies a global suite setting (such as changing the active language between English and Türkçe in the launcher settings), the launcher updates a centralized, machine-readable preferences file.

---

## File Location
The file is located in the user's standard configuration directory:

- **Linux**: `~/.config/prism/shared.ron`
- **Windows**: `%APPDATA%\prism\shared.ron`

---

## File Format & Schema

The file is encoded in **Rusty Object Notation (RON)** format.

```ron
(
    // The active IETF BCP 47 language code chosen in the launcher ("en" or "tr")
    language: "tr",

    // Current version of the prisM. launcher that wrote the preferences
    launcher_version: "0.1.0",
)
```

### Fields:
| Field | Type | Description |
|---|---|---|
| `language` | `String` | Two-letter lowercase language code (`"en"`, `"tr"`). Child apps can inspect this field on startup to align their UI language with the suite. |
| `launcher_version` | `String` | Semantic version string of the launcher application. |

---

## Child Application Integration Guidelines

Each application (`triM.`, `steM.`, `beaM.`) can optionally read this file during initialization before falling back to system locale:

```rust
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
struct SharedSuitePrefs {
    language: String,
    launcher_version: String,
}

fn detect_suite_language() -> Option<String> {
    directories::ProjectDirs::from("com", "prism", "prism")
        .map(|dirs| dirs.config_dir().join("shared.ron"))
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|content| ron::from_str::<SharedSuitePrefs>(&content).ok())
        .map(|prefs| prefs.language)
}
```
