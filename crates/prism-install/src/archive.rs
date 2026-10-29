// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Zip archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Security violation: Path traversal detected in archive entry '{0}'")]
    PathTraversal(String),

    #[error("Security violation: Absolute path detected in archive entry '{0}'")]
    AbsolutePath(String),

    #[error("Security violation: Symlink target '{target}' escapes destination '{dest}'")]
    SymlinkEscape { target: String, dest: String },

    #[error("Unsupported archive format: {0}")]
    UnsupportedFormat(String),
}

/// Safely validates that a relative archive entry path does not escape the destination root.
pub fn sanitize_entry_path(dest_root: &Path, entry_path: &Path) -> Result<PathBuf, ArchiveError> {
    for component in entry_path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(ArchiveError::PathTraversal(
                    entry_path.display().to_string(),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ArchiveError::AbsolutePath(entry_path.display().to_string()));
            }
        }
    }

    let resolved = dest_root.join(entry_path);
    Ok(resolved)
}

/// Safely extracts a `.zip` archive into `destination_dir`.
pub fn extract_zip(archive_bytes: &[u8], destination_dir: &Path) -> Result<(), ArchiveError> {
    let reader = Cursor::new(archive_bytes);
    let mut zip = zip::ZipArchive::new(reader)?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let raw_path = file.mangled_name();

        let outpath = sanitize_entry_path(destination_dir, &raw_path)?;

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}

/// Safely extracts a `.tar.gz` archive into `destination_dir`.
pub fn extract_tar_gz(archive_bytes: &[u8], destination_dir: &Path) -> Result<(), ArchiveError> {
    let cursor = Cursor::new(archive_bytes);
    let gz = flate2::read::GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(gz);

    for entry_res in archive.entries()? {
        let mut entry = entry_res?;
        let path = entry.path()?.into_owned();

        let outpath = sanitize_entry_path(destination_dir, &path)?;

        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&outpath)?;
        } else if entry.header().entry_type().is_symlink() {
            #[cfg(unix)]
            {
                if let Some(target) = entry.link_name()? {
                    // Validate symlink does not escape destination_dir
                    let symlink_dest = if target.is_absolute() {
                        return Err(ArchiveError::AbsolutePath(target.display().to_string()));
                    } else if let Some(parent) = outpath.parent() {
                        parent.join(&target)
                    } else {
                        outpath.join(&target)
                    };

                    // Check normalized path
                    let normalized = normalize_path(&symlink_dest);
                    if !normalized.starts_with(destination_dir) {
                        return Err(ArchiveError::SymlinkEscape {
                            target: target.display().to_string(),
                            dest: destination_dir.display().to_string(),
                        });
                    }

                    if let Some(parent) = outpath.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let _ = fs::remove_file(&outpath);
                    std::os::unix::fs::symlink(&target, &outpath)?;
                }
            }
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut entry, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(mode) = entry.header().mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}

/// Extracts an archive automatically detecting format from filename or content.
pub fn extract_archive(
    filename: &str,
    archive_bytes: &[u8],
    destination_dir: &Path,
) -> Result<(), ArchiveError> {
    fs::create_dir_all(destination_dir)?;

    if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        extract_tar_gz(archive_bytes, destination_dir)
    } else if filename.ends_with(".zip") {
        extract_zip(archive_bytes, destination_dir)
    } else {
        // Try gzip magic bytes (0x1f, 0x8b)
        if archive_bytes.len() >= 2 && archive_bytes[0] == 0x1f && archive_bytes[1] == 0x8b {
            extract_tar_gz(archive_bytes, destination_dir)
        } else if archive_bytes.len() >= 4 && &archive_bytes[0..4] == b"PK\x03\x04" {
            extract_zip(archive_bytes, destination_dir)
        } else {
            Err(ArchiveError::UnsupportedFormat(filename.to_string()))
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            _ => normalized.push(comp),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_entry_path_rejects_traversal() {
        let dest = Path::new("/tmp/prism-dest");
        assert!(sanitize_entry_path(dest, Path::new("../evil.txt")).is_err());
        assert!(sanitize_entry_path(dest, Path::new("sub/../../evil.txt")).is_err());
        assert!(sanitize_entry_path(dest, Path::new("/absolute/path")).is_err());
        assert!(sanitize_entry_path(dest, Path::new("safe/sub/file.txt")).is_ok());
    }
}
