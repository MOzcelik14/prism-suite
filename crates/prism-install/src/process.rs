// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::path::Path;
use std::process::{Command, Stdio};

/// Launches an application binary as a completely detached child process.
pub fn launch_detached(bin_path: &Path, working_dir: &Path) -> std::io::Result<u32> {
    // Resolve symlinks so launcher scripts that locate resources relative to
    // themselves (e.g. `dirname "$BASH_SOURCE"`) see their real directory.
    let real_bin = std::fs::canonicalize(bin_path).unwrap_or_else(|_| bin_path.to_path_buf());
    let real_dir = if working_dir == bin_path.parent().unwrap_or(working_dir) {
        real_bin.parent().unwrap_or(working_dir).to_path_buf()
    } else {
        working_dir.to_path_buf()
    };

    let mut cmd = Command::new(&real_bin);
    cmd.current_dir(&real_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd.spawn()?;
    let pid = child.id();
    // Reap the child when it exits; otherwise it lingers as a zombie and the
    // launcher keeps reporting the application as "running".
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(pid)
}

/// Checks whether an application binary is currently running.
pub fn is_binary_running(binary_name: &str, known_pid: Option<u32>) -> bool {
    let clean_bin = binary_name.trim();

    #[cfg(target_os = "linux")]
    {
        // 1. Direct check via known PID if available
        if let Some(pid) = known_pid {
            let comm_path = format!("/proc/{}/comm", pid);
            if let Ok(comm) = std::fs::read_to_string(comm_path) {
                if comm.trim() == clean_bin && !is_zombie(pid) {
                    return true;
                }
            }
        }

        // 2. Scan /proc for running instances
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.chars().all(|c| c.is_ascii_digit()) {
                        let comm_path = entry.path().join("comm");
                        if let Ok(comm) = std::fs::read_to_string(comm_path) {
                            if comm.trim() == clean_bin {
                                if let Ok(pid) = name.parse::<u32>() {
                                    if is_zombie(pid) {
                                        continue;
                                    }
                                }
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = clean_bin;
        let _ = known_pid;
    }

    false
}

/// Returns true when the process has exited but has not been reaped yet.
#[cfg(target_os = "linux")]
fn is_zombie(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            // Format: pid (comm) S ...; comm may contain spaces/parens, so split after the last ')'.
            stat.rfind(')').and_then(|i| {
                stat[i + 1..]
                    .split_whitespace()
                    .next()
                    .map(|s| s == "Z" || s == "X")
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    #[test]
    fn test_process_detection_for_current_runner() {
        assert!(!is_binary_running("nonexistent_binary_xyz_12345", None));
    }
}
