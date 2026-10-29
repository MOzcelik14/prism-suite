// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

/// Unique identifier for an application within the prisM. Suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppId {
    Trim,
    Stem,
    Beam,
}

impl AppId {
    pub const ALL: [AppId; 3] = [AppId::Trim, AppId::Stem, AppId::Beam];

    pub fn as_str(&self) -> &'static str {
        match self {
            AppId::Trim => "trim",
            AppId::Stem => "stem",
            AppId::Beam => "beam",
        }
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metadata and catalog entry for an application in the suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: AppId,
    pub display_name: String,
    pub category: String,
    pub tagline_key: String,
    pub description_key: String,
    pub runtime_notes_key: Option<String>,
    pub repository: String,
    pub default_branch: String,
    pub primary_binary_name: String,
    pub accent_color: (u8, u8, u8),
}

/// The collection of all applications in the suite catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCatalog {
    pub apps: Vec<AppEntry>,
}

impl AppCatalog {
    /// Loads the embedded suite catalog compiled into the binary.
    pub fn load_embedded() -> Self {
        const CATALOG_DATA: &str = include_str!("catalog.ron");
        ron::from_str(CATALOG_DATA).expect("embedded catalog.ron must be valid")
    }

    /// Finds an app by its AppId.
    pub fn get(&self, id: AppId) -> Option<&AppEntry> {
        self.apps.iter().find(|app| app.id == id)
    }

    /// Returns a slice of all apps in the catalog.
    pub fn all(&self) -> &[AppEntry] {
        &self.apps
    }
}

impl Default for AppCatalog {
    fn default() -> Self {
        Self::load_embedded()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_catalog_loads_and_contains_all_apps() {
        let catalog = AppCatalog::load_embedded();
        assert_eq!(catalog.apps.len(), 3);

        let trim = catalog.get(AppId::Trim).expect("triM. must be in catalog");
        assert_eq!(trim.display_name, "triM.");
        assert_eq!(trim.repository, "https://github.com/MOzcelik14/triM");
        assert_eq!(trim.default_branch, "master");

        let stem = catalog.get(AppId::Stem).expect("steM. must be in catalog");
        assert_eq!(stem.display_name, "steM.");
        assert_eq!(stem.repository, "https://github.com/MOzcelik14/steM");
        assert_eq!(stem.default_branch, "main");

        let beam = catalog.get(AppId::Beam).expect("beaM. must be in catalog");
        assert_eq!(beam.display_name, "beaM.");
        assert_eq!(beam.repository, "https://github.com/MOzcelik14/beaM");
        assert_eq!(beam.default_branch, "master");
    }
}
