// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use crate::catalog::AppId;
use serde::{Deserialize, Serialize};

/// Target operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetOs {
    Linux,
    Windows,
    MacOs,
}

impl TargetOs {
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            TargetOs::Linux
        }
        #[cfg(target_os = "windows")]
        {
            TargetOs::Windows
        }
        #[cfg(target_os = "macos")]
        {
            TargetOs::MacOs
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            TargetOs::Linux
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TargetOs::Linux => "linux",
            TargetOs::Windows => "windows",
            TargetOs::MacOs => "macos",
        }
    }
}

/// Target CPU architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetArch {
    X86_64,
    Aarch64,
}

impl TargetArch {
    pub fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            TargetArch::X86_64
        }
        #[cfg(target_arch = "aarch64")]
        {
            TargetArch::Aarch64
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            TargetArch::X86_64
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x86_64",
            TargetArch::Aarch64 => "aarch64",
        }
    }
}

/// Metadata describing a specific installable binary asset within a release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub os: TargetOs,
    pub arch: TargetArch,
    pub filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub download_url: String,
}

/// Metadata describing an app release version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppReleaseManifest {
    pub app_id: AppId,
    pub version: String,
    pub release_date: String,
    pub notes: String,
    pub assets: Vec<ReleaseAsset>,
}
