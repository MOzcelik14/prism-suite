<p align="center">
  <img src="assets/logo.png" alt="prisM. Suite" width="160" />
</p>

<h1 align="center">prisM. Suite</h1>

<p align="center">
  Centralized desktop portal and update manager for the prisM. creative application suite.
</p>

<p align="center">
  <a href="https://github.com/MOzcelik14/prism-suite/actions/workflows/ci.yml"><img src="https://github.com/MOzcelik14/prism-suite/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/MOzcelik14/prism-suite/actions/workflows/release.yml"><img src="https://github.com/MOzcelik14/prism-suite/actions/workflows/release.yml/badge.svg" alt="Release" /></a>
  <a href="https://www.gnu.org/licenses/gpl-3.0"><img src="https://img.shields.io/badge/License-GPL--3.0--only-blue.svg" alt="License: GPL-3.0-only" /></a>
  <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20Windows-lightgrey.svg" alt="Platform: Linux | Windows" />
</p>

---

## Overview

**prisM. Suite** provides a single desktop hub to install, launch, and update the prisM. creative applications on Linux and Windows. Designed for reliability and privacy, it operates without user accounts, cloud lock-in, or telemetry.

## Included Applications

| Application | Category | Description | Repository |
| :--- | :--- | :--- | :--- |
| **triM.** | Video Editing | Multi-track timeline, color grading, audio mixing, and hardware-accelerated export | [MOzcelik14/triM](https://github.com/MOzcelik14/triM) |
| **steM.** | Audio Production | Non-destructive multi-track audio editor, real-time effects, and spectral cleanup | [MOzcelik14/steM](https://github.com/MOzcelik14/steM) |
| **beaM.** | Photo Studio | Non-destructive RAW photo editor, color grading, and local library catalog | [MOzcelik14/beaM](https://github.com/MOzcelik14/beaM) |

## Key Highlights

- **Unified Management**: Install, update, and launch all suite applications from one interface.
- **Safe Updates & Rollback**: New versions are installed alongside the previous release so you can revert with a single click if needed.
- **Verified Packages**: Every download is checked for integrity and authenticity before installation.
- **Independent Execution**: Applications run in their own processes; closing or updating the launcher will not interrupt active work.
- **Offline & Private**: No accounts, tracking, or background data collection. Supports English and Turkish out of the box.

---

## Download & Installation

Pre-built packages for **v1.0.0** are available on the [GitHub Releases](https://github.com/MOzcelik14/prism-suite/releases) page:

| Operating System | Package | File |
| :--- | :--- | :--- |
| **Debian / Ubuntu** | `.deb` | `prism_1.0.0_amd64.deb` |
| **Fedora / RHEL** | `.rpm` | `prism-1.0.0-1.x86_64.rpm` |
| **Linux (Universal)** | `.AppImage` | `prism-1.0.0-x86_64.AppImage` |
| **Linux (Portable)** | `.tar.gz` | `prism-1.0.0-linux-x86_64.tar.gz` |
| **Windows (64-bit)** | `.zip` | `prism-1.0.0-windows-x86_64.zip` |

### Debian / Ubuntu
```bash
sudo dpkg -i prism_1.0.0_amd64.deb
sudo apt-get install -f
```

### Fedora / RHEL
```bash
sudo dnf install ./prism-1.0.0-1.x86_64.rpm
```

### Linux AppImage
```bash
chmod +x prism-1.0.0-x86_64.AppImage
./prism-1.0.0-x86_64.AppImage
```

### Windows
1. Extract `prism-1.0.0-windows-x86_64.zip` to a folder of your choice.
2. Run `prism.exe`.

---

## System Requirements

- **Linux**: 64-bit distribution (glibc 2.31+), X11 or Wayland.
- **Windows**: Windows 10 (1809+) or Windows 11 (64-bit).
- **Hardware**: 4 GB RAM minimum (8 GB recommended for media workloads) and 500 MB free disk space for the launcher.

## Building from Source

Requires Rust 1.80+ (and standard X11/Wayland/ALSA development headers on Linux):

```bash
cargo build --release -p prism-app
```

Run tests:
```bash
cargo test --workspace
```

## License

Licensed under the **GNU General Public License v3.0 (GPL-3.0-only)**. See [LICENSE](LICENSE) for details.

Copyright (C) 2026 Murat Özçelik.
