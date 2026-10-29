// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::path::Path;
use std::process::{Command, Stdio};

/// Launches an application binary as a completely detached child process.
pub fn launch_detached(bin_path: &Path, working_dir: &Path) -> std::io::Result<u32> {
    let mut cmd = Command::new(bin_path);
    cmd.current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd.spawn()?;
    Ok(child.id())
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
                if comm.trim() == clean_bin {
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

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    #[test]
    fn test_process_detection_for_current_runner() {
        assert!(!is_binary_running("nonexistent_binary_xyz_12345", None));
    }
}
