// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;
use std::process::ExitCode;

use prism_core::branding::{SUITE_DISPLAY_NAME, SUITE_VERSION};
use prism_core::catalog::{AppCatalog, AppEntry, AppId};
use prism_core::config::LauncherConfig;
use prism_core::shared::SharedPreferences;
use prism_core::state::{AppInstallState, ReleaseInfo};
use prism_install::InstallationManager;
use prism_net::{verify_ed25519_signature, verify_sha256sums, NetworkClient};
use semver::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Help,
    Version,
    List { offline: bool },
    Install { app: AppId },
    Update { app: Option<AppId> },
    Launch { app: AppId, file: Option<PathBuf> },
    Recent,
}

/// Parses an application identifier case-insensitively (`trim`, `stem`, `beam`, `dram`, with or without trailing `.`).
pub fn parse_app_id(raw: &str) -> Result<AppId, String> {
    match raw.trim().trim_end_matches('.').to_ascii_lowercase().as_str() {
        "trim" => Ok(AppId::Trim),
        "stem" => Ok(AppId::Stem),
        "beam" => Ok(AppId::Beam),
        "dram" => Ok(AppId::Dram),
        other => Err(format!(
            "Unknown app '{other}'. Expected one of: trim, stem, beam, dram"
        )),
    }
}

/// Parses CLI arguments (excluding argv[0]).
pub fn parse_args<I, S>(args: I) -> Result<CliCommand, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let items: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    let Some(cmd) = items.first().map(String::as_str) else {
        return Ok(CliCommand::Help);
    };

    match cmd {
        "-h" | "--help" | "help" => Ok(CliCommand::Help),
        "-V" | "--version" | "version" => Ok(CliCommand::Version),
        "list" => {
            let mut offline = false;
            for arg in &items[1..] {
                match arg.as_str() {
                    "--offline" => offline = true,
                    other => return Err(format!("Unknown flag for 'list': {other}")),
                }
            }
            Ok(CliCommand::List { offline })
        }
        "install" => {
            if items.len() != 2 {
                return Err("Usage: prism install <trim|stem|beam|dram>".to_string());
            }
            Ok(CliCommand::Install {
                app: parse_app_id(&items[1])?,
            })
        }
        "update" => match items.get(1).map(String::as_str) {
            None | Some("--all") if items.len() <= 2 => Ok(CliCommand::Update { app: None }),
            Some(target) if items.len() == 2 => Ok(CliCommand::Update {
                app: Some(parse_app_id(target)?),
            }),
            _ => Err("Usage: prism update [trim|stem|beam|dram|--all]".to_string()),
        },
        "launch" => {
            if items.len() < 2 || items.len() > 3 {
                return Err("Usage: prism launch <trim|stem|beam|dram> [file]".to_string());
            }
            let app = parse_app_id(&items[1])?;
            let file = items.get(2).map(PathBuf::from);
            Ok(CliCommand::Launch { app, file })
        }
        "recent" => {
            if items.len() != 1 {
                return Err("Usage: prism recent".to_string());
            }
            Ok(CliCommand::Recent)
        }
        other => Err(format!(
            "Unknown command '{other}'. Run 'prism --help' for usage."
        )),
    }
}

fn print_help() {
    println!(
        "{SUITE_DISPLAY_NAME} v{SUITE_VERSION}\n\n\
         USAGE:\n  \
           prism                         Launch graphical hub\n  \
           prism list [--offline]        List applications and installed/latest versions\n  \
           prism install <app>           Download, verify, and install an application\n  \
           prism update [app | --all]    Update an application or all installed apps\n  \
           prism launch <app> [file]     Launch an application (optionally with a project file)\n  \
           prism recent                  List recently opened projects\n  \
           prism --version               Print version information\n  \
           prism --help                  Show this help message\n\n\
         APPS:\n  \
           trim, stem, beam, dram"
    );
}

/// Runs the CLI command specified by process arguments and returns an exit code.
pub fn run_from_args() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse_args(args) {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("Error: {err}");
            return ExitCode::from(2);
        }
    };

    match execute(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn execute(command: CliCommand) -> Result<(), String> {
    match command {
        CliCommand::Help => {
            print_help();
            Ok(())
        }
        CliCommand::Version => {
            println!("{SUITE_DISPLAY_NAME} v{SUITE_VERSION}");
            Ok(())
        }
        CliCommand::Recent => {
            let catalog = AppCatalog::load_embedded();
            let prefs = SharedPreferences::load();
            if prefs.recent_projects.is_empty() {
                println!("No recent projects recorded.");
                return Ok(());
            }
            for item in &prefs.recent_projects {
                let app_name = catalog
                    .get(item.app_id)
                    .map(|e| e.display_name.as_str())
                    .unwrap_or(item.app_id.as_str());
                let exists_marker = if item.path.exists() { "" } else { " [missing]" };
                println!(
                    "{:<7} {} ({}){}",
                    app_name,
                    item.name,
                    item.path.display(),
                    exists_marker
                );
            }
            Ok(())
        }
        CliCommand::Launch { app, file } => {
            let catalog = AppCatalog::load_embedded();
            let config = LauncherConfig::load();
            let install_mgr = InstallationManager::new(config.custom_install_dir.clone());
            let entry = catalog
                .get(app)
                .ok_or_else(|| format!("App '{}' not found in catalog", app))?;

            let pid = if let Some(ref project_path) = file {
                if !project_path.exists() {
                    return Err(format!(
                        "Project file does not exist: {}",
                        project_path.display()
                    ));
                }
                let abs_path = std::fs::canonicalize(project_path)
                    .unwrap_or_else(|_| project_path.clone());
                let arg = abs_path.to_string_lossy().into_owned();
                let pid = install_mgr
                    .launch_app_with_args(entry, &[&arg])
                    .map_err(|e| format!("Failed to launch {}: {e}", entry.display_name))?;
                let mut prefs = SharedPreferences::load();
                prefs.record_recent_project(app, abs_path);
                let _ = prefs.save();
                pid
            } else {
                install_mgr
                    .launch_app(entry)
                    .map_err(|e| format!("Failed to launch {}: {e}", entry.display_name))?
            };

            println!("Launched {} (PID {})", entry.display_name, pid);
            Ok(())
        }
        CliCommand::List { offline } => run_list(offline),
        CliCommand::Install { app } => run_install(app),
        CliCommand::Update { app } => run_update(app),
    }
}

fn installed_version(install_mgr: &InstallationManager, entry: &AppEntry) -> Option<String> {
    match install_mgr.detect_state(entry) {
        AppInstallState::Installed { version, .. } => Some(version),
        _ => None,
    }
}

fn build_runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to initialize async runtime: {e}"))
}

fn run_list(offline: bool) -> Result<(), String> {
    let catalog = AppCatalog::load_embedded();
    let config = LauncherConfig::load();
    let install_mgr = InstallationManager::new(config.custom_install_dir.clone());
    let client = NetworkClient::new().map_err(|e| format!("Network client error: {e}"))?;
    let rt = if offline { None } else { Some(build_runtime()?) };

    println!("{:<8} {:<14} {:<14} LATEST", "ID", "APP", "INSTALLED");
    for entry in catalog.all() {
        let installed = installed_version(&install_mgr, entry)
            .map(|v| format!("v{v}"))
            .unwrap_or_else(|| "-".to_string());
        let latest = if let Some(ref runtime) = rt {
            match runtime.block_on(client.discover_release(entry, config.release_channel, false)) {
                Ok(Some(rel)) => format!("v{}", rel.version),
                Ok(None) => "-".to_string(),
                Err(_) => "error".to_string(),
            }
        } else {
            "(offline)".to_string()
        };
        println!(
            "{:<8} {:<14} {:<14} {}",
            entry.id.as_str(),
            entry.display_name,
            installed,
            latest
        );
    }
    Ok(())
}

fn install_from_release(
    rt: &tokio::runtime::Runtime,
    client: &NetworkClient,
    install_mgr: &InstallationManager,
    entry: &AppEntry,
    release: &ReleaseInfo,
    keep_count: usize,
) -> Result<(), String> {
    if install_mgr.is_running(entry, None) {
        return Err(format!(
            "{} is currently running. Close it before installing or updating.",
            entry.display_name
        ));
    }

    println!(
        "Downloading {} v{} ({})...",
        entry.display_name, release.version, release.asset_filename
    );
    let archive_bytes = rt
        .block_on(client.download_with_progress(&release.asset_download_url, None, |_, _, _| {}))
        .map_err(|e| format!("Download failed: {e}"))?;

    if let Some(ref checksums_url) = release.checksums_url {
        if let Ok(manifest_content) = rt.block_on(client.fetch_text(checksums_url)) {
            verify_sha256sums(&release.asset_filename, &archive_bytes, &manifest_content)
                .map_err(|e| format!("Checksum verification failed: {e}"))?;

            if let Some(ref sig_url) = release.signature_url {
                if let Ok(sig_content) = rt.block_on(client.fetch_text(sig_url)) {
                    if let Ok(pubkey) = NetworkClient::public_key() {
                        verify_ed25519_signature(
                            manifest_content.as_bytes(),
                            &sig_content,
                            &pubkey,
                        )
                        .map_err(|e| format!("Signature verification failed: {e}"))?;
                    }
                }
            }
        }
    }

    let target_ver = release.version.to_string();
    println!("Installing {} v{}...", entry.display_name, target_ver);
    install_mgr
        .install_package(
            entry,
            &target_ver,
            &release.asset_filename,
            &archive_bytes,
            keep_count,
        )
        .map_err(|e| format!("Installation failed: {e}"))?;

    println!("Installed {} v{}", entry.display_name, target_ver);
    Ok(())
}

fn run_install(app_id: AppId) -> Result<(), String> {
    let catalog = AppCatalog::load_embedded();
    let config = LauncherConfig::load();
    let install_mgr = InstallationManager::new(config.custom_install_dir.clone());
    let entry = catalog
        .get(app_id)
        .ok_or_else(|| format!("App '{}' not found in catalog", app_id))?;

    let rt = build_runtime()?;
    let client = NetworkClient::new().map_err(|e| format!("Network client error: {e}"))?;
    let release = rt
        .block_on(client.discover_release(entry, config.release_channel, true))
        .map_err(|e| format!("Release lookup failed: {e}"))?
        .ok_or_else(|| format!("No release found for {}", entry.display_name))?;

    install_from_release(
        &rt,
        &client,
        &install_mgr,
        entry,
        &release,
        config.keep_versions_count,
    )
}

fn run_update(target: Option<AppId>) -> Result<(), String> {
    let catalog = AppCatalog::load_embedded();
    let config = LauncherConfig::load();
    let install_mgr = InstallationManager::new(config.custom_install_dir.clone());
    let rt = build_runtime()?;
    let client = NetworkClient::new().map_err(|e| format!("Network client error: {e}"))?;

    let entries: Vec<&AppEntry> = match target {
        Some(id) => vec![catalog
            .get(id)
            .ok_or_else(|| format!("App '{}' not found in catalog", id))?],
        None => catalog
            .all()
            .iter()
            .filter(|e| installed_version(&install_mgr, e).is_some())
            .collect(),
    };

    if entries.is_empty() {
        println!("No installed applications to update.");
        return Ok(());
    }

    let mut updated_any = false;
    for entry in entries {
        let Some(current) = installed_version(&install_mgr, entry) else {
            return Err(format!(
                "{} is not installed. Run 'prism install {}' first.",
                entry.display_name,
                entry.id.as_str()
            ));
        };

        let release = rt
            .block_on(client.discover_release(entry, config.release_channel, true))
            .map_err(|e| format!("Release lookup failed for {}: {e}", entry.display_name))?;

        let Some(release) = release else {
            continue;
        };

        let needs_update = match Version::parse(current.trim_start_matches('v')) {
            Ok(cur_v) => release.version > cur_v,
            Err(_) => release.version.to_string() != current.trim_start_matches('v'),
        };

        if needs_update {
            install_from_release(
                &rt,
                &client,
                &install_mgr,
                entry,
                &release,
                config.keep_versions_count,
            )?;
            updated_any = true;
        } else {
            println!("{} is already up to date (v{}).", entry.display_name, current);
        }
    }

    if !updated_any {
        println!("All checked applications are up to date.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_app_id_variants() {
        assert_eq!(parse_app_id("trim"), Ok(AppId::Trim));
        assert_eq!(parse_app_id("triM."), Ok(AppId::Trim));
        assert_eq!(parse_app_id("STEM"), Ok(AppId::Stem));
        assert_eq!(parse_app_id("beaM."), Ok(AppId::Beam));
        assert_eq!(parse_app_id("dram"), Ok(AppId::Dram));
        assert!(parse_app_id("unknown").is_err());
    }

    #[test]
    fn test_parse_args_commands() {
        assert_eq!(parse_args(["--help"]).unwrap(), CliCommand::Help);
        assert_eq!(parse_args(["-V"]).unwrap(), CliCommand::Version);
        assert_eq!(
            parse_args(["list"]).unwrap(),
            CliCommand::List { offline: false }
        );
        assert_eq!(
            parse_args(["list", "--offline"]).unwrap(),
            CliCommand::List { offline: true }
        );
        assert_eq!(
            parse_args(["install", "trim"]).unwrap(),
            CliCommand::Install { app: AppId::Trim }
        );
        assert_eq!(
            parse_args(["update"]).unwrap(),
            CliCommand::Update { app: None }
        );
        assert_eq!(
            parse_args(["update", "--all"]).unwrap(),
            CliCommand::Update { app: None }
        );
        assert_eq!(
            parse_args(["update", "beam"]).unwrap(),
            CliCommand::Update {
                app: Some(AppId::Beam)
            }
        );
        assert_eq!(
            parse_args(["launch", "dram", "/tmp/script.dram"]).unwrap(),
            CliCommand::Launch {
                app: AppId::Dram,
                file: Some(PathBuf::from("/tmp/script.dram")),
            }
        );
        assert_eq!(parse_args(["recent"]).unwrap(), CliCommand::Recent);
    }

    #[test]
    fn test_parse_args_errors() {
        assert!(parse_args(["install"]).is_err());
        assert!(parse_args(["launch"]).is_err());
        assert!(parse_args(["list", "--bogus"]).is_err());
        assert!(parse_args(["unknown-cmd"]).is_err());
    }
}
