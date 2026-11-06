<p align="center">
  <img src="assets/logo.png" alt="prisM. Suite logo" width="180" />
</p>

# prisM. Suite

[![CI](https://github.com/MOzcelik14/prism-suite/actions/workflows/ci.yml/badge.svg)](https://github.com/MOzcelik14/prism-suite/actions/workflows/ci.yml)
[![Release](https://github.com/MOzcelik14/prism-suite/actions/workflows/release.yml/badge.svg)](https://github.com/MOzcelik14/prism-suite/actions/workflows/release.yml)
[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL--3.0--only-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey.svg)](https://github.com/MOzcelik14/prism-suite/releases)

prisM. Suite is a desktop management portal, package manager, and update supervisor designed for open-source creative desktop applications. It provides a centralized, secure, and offline-first environment for deploying, maintaining, and launching media production software on Linux and Windows workstations.

---

## Managed Creative Applications

The suite coordinates three standalone creative tools developed with an emphasis on low latency, native system execution, and data sovereignty:

| Application | Domain | Primary Focus | Repository |
| :--- | :--- | :--- | :--- |
| **triM.** | Video Editing | Multi-track timeline, hardware-accelerated playback, non-linear video assembly | [github.com/MOzcelik14/triM](https://github.com/MOzcelik14/triM) |
| **steM.** | Audio Production | Digital audio workstation, multitrack mixing, waveform editing, low-latency DSP | [github.com/MOzcelik14/steM](https://github.com/MOzcelik14/steM) |
| **beaM.** | Photo Studio | Non-destructive RAW photographic darkroom, parametric color grading, batch export | [github.com/MOzcelik14/beaM](https://github.com/MOzcelik14/beaM) |

Each application runs as an independent native process. The suite manages installations and updates without interfering with active editing sessions.

---

## Architectural Principles

### 1. Atomic Deployment and Version Rollbacks
Applications are installed into immutable version directories (`apps/<id>/versions/<version>/`). The active release is referenced via filesystem-level atomic symlinks on Linux and directory junctions on Windows. If an update introduces unexpected regressions during production work, users can revert to the previously installed version in a single step with zero data loss.

### 2. Cryptographic Supply-Chain Verification
All binary distributions are validated against published checksums (`SHA256SUMS`) and signed with detached Ed25519 Minisign signatures prior to extraction. Archive unpacking enforces strict path sanitization, rejecting path traversal attempts, absolute paths, and symlink escapes outside the target version tree.

### 3. Local-First and Privacy-Preserving
prisM. Suite requires no user account, transmits no analytics or telemetry, and does not depend on cloud synchronization. When configured in offline mode, it functions entirely within the local environment.

### 4. Process Isolation
Launched creative tools operate in detached process trees (`setsid` on Linux, independent process creation flags on Windows). Closing or updating the launcher never terminates running creative projects.

### 5. Embedded Standalone Typography
The user interface embeds the DejaVu Sans typography engine directly into the compiled binary. This guarantees consistent rendering of all Latin Extended characters (including full Turkish language glyphs) and UI control symbols without relying on external system font packages.

---

## Downloads & Distribution Packages

Pre-compiled production packages for version `0.2.0` are available on the [GitHub Releases](https://github.com/MOzcelik14/prism-suite/releases) page:

| Operating System | Package Type | Artifact | Architecture |
| :--- | :--- | :--- | :--- |
| **Debian / Ubuntu** | Native `.deb` | `prism_0.2.0_amd64.deb` | x86_64 |
| **Fedora / RHEL** | Native `.rpm` | `prism-0.2.0-1.x86_64.rpm` | x86_64 |
| **Generic Linux** | Portable `.AppImage` | `prism-0.2.0-x86_64.AppImage` | x86_64 |
| **Generic Linux** | Portable `.tar.gz` | `prism-0.2.0-linux-x86_64.tar.gz` | x86_64 |
| **Windows** | Portable `.zip` | `prism-0.2.0-windows-x86_64.zip` | x86_64 |

---

## Installation Instructions

### Debian, Ubuntu, and Derivatives (.deb)

```bash
# Download and install the package
sudo dpkg -i prism_0.2.0_amd64.deb

# Resolve any missing runtime dependencies
sudo apt-get install -f
```

### Fedora, CentOS, RHEL, and Rocky Linux (.rpm)

```bash
sudo dnf install ./prism-0.2.0-1.x86_64.rpm
```

### Generic Linux (AppImage)

The AppImage package requires no installation and can be executed directly:

```bash
chmod +x prism-0.2.0-x86_64.AppImage
./prism-0.2.0-x86_64.AppImage
```

### Windows (Portable Zip)

1. Extract `prism-0.2.0-windows-x86_64.zip` to a directory of your choice (e.g., `C:\Program Files\prism-suite` or a local user folder).
2. Execute `prism.exe`.
3. Application shortcuts and directory junctions will be initialized automatically in `%LOCALAPPDATA%\prism-suite`.

---

## System Requirements

- **Linux**: Kernel 5.4 or newer, glibc 2.31 or newer, X11 or Wayland display server, standard audio subsystem (ALSA / PulseAudio / PipeWire).
- **Windows**: Windows 10 (version 1809 or newer) or Windows 11, 64-bit architecture.
- **Hardware**: Minimum 4 GB RAM (8 GB recommended for media editing), 500 MB disk space for the suite launcher, plus dedicated storage for creative applications and media caches.

---

## Workspace Structure

The repository is organized as a Cargo workspace with strict module boundaries:

```
prism-suite/
├── crates/
│   ├── prism-app/       # Desktop frontend built with egui and eframe 0.29
│   ├── prism-core/      # Domain models, app catalog, state definitions, shared preferences
│   ├── prism-net/       # HTTP release client, hash verifier, Ed25519 signature validation
│   ├── prism-install/   # Atomic symlink supervisor, archive extraction, desktop entry integration
│   └── prism-i18n/      # Fluent localization framework (English and Turkish bundles)
├── assets/              # Desktop entries, vector graphics, icons, and embedded font resources
├── docs/                # Architecture specifications, signing protocols, and website documentation
├── scripts/
│   ├── package-linux.sh # Multi-format Linux packaging automation (deb, rpm, appimage, tarball)
│   ├── package.sh       # Unified release staging script
│   └── sign-release.sh  # Cryptographic manifest generation and Minisign automation
├── .github/workflows/
│   ├── ci.yml           # Multi-platform CI pipeline (Linux, Windows, clippy, unit tests)
│   └── release.yml      # Release artifact build and publishing workflow
├── Cargo.toml           # Workspace configuration (version 0.2.0)
├── LICENSE              # GNU General Public License v3.0
└── README.md
```

---

## Building from Source

### Prerequisites

1. **Rust Toolchain**: Stable compiler version 1.80 or newer:
   ```bash
   rustup default stable
   ```

2. **Linux Development Libraries**:
   ```bash
   # Debian / Ubuntu:
   sudo apt-get install -y build-essential libxcb-render0-dev libxcb-shape0-dev \
       libxcb-xfixes0-dev libxkbcommon-dev libasound2-dev
   ```

### Compilation

```bash
# Debug build (faster compilation)
cargo run -p prism-app

# Release build (optimized native executable)
cargo build --release -p prism-app
```

The resulting binary will be placed at:
- Linux: `target/release/prism`
- Windows: `target/release/prism.exe`

### Automated Test Suite

```bash
# Run unit and integration tests across all crates
cargo test --workspace

# Validate linter compliance
cargo clippy --workspace --all-targets -- -D warnings

# Validate code formatting
cargo fmt --check
```

---

## License

This software is released under the **GNU General Public License v3.0 (GPL-3.0-only)**. Refer to the [LICENSE](LICENSE) file for the complete terms and conditions.

Copyright (C) 2026 Murat Özçelik. All rights reserved.
