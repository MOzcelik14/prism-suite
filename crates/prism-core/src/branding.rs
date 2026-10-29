//! Core branding definitions for the prisM. Suite launcher and creative applications.

/// The display name for the suite as a whole (used in window title, installer, about dialog, docs).
pub const SUITE_DISPLAY_NAME: &str = "prisM. Suite";

/// The compact display name for the launcher application (used in taskbars, tray icons, system menus).
pub const LAUNCHER_DISPLAY_NAME: &str = "prisM.";

/// Current launcher version.
pub const SUITE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub organization / user URL.
pub const GITHUB_ORG_URL: &str = "https://github.com/MOzcelik14";

/// GitHub repository for the launcher itself.
pub const LAUNCHER_REPO_URL: &str = "https://github.com/MOzcelik14/prism-suite";

/// License identifier.
pub const SUITE_LICENSE: &str = "GPL-3.0-only";
