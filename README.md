<p align="center">
  <img src="assets/logo.png" alt="prisM. Suite logo" width="200" />
</p>

# prisM. Suite

[![CI](https://github.com/MOzcelik14/prism-suite/actions/workflows/ci.yml/badge.svg)](https://github.com/MOzcelik14/prism-suite/actions/workflows/ci.yml)
[![Release](https://github.com/MOzcelik14/prism-suite/actions/workflows/release.yml/badge.svg)](https://github.com/MOzcelik14/prism-suite/actions/workflows/release.yml)
[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL--3.0--only-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey.svg)](https://github.com/MOzcelik14/prism-suite/releases)

prisM. Suite is a native desktop launcher, package manager, and auto-updater for creative desktop applications:

- **[triM.](https://github.com/MOzcelik14/triM)**: Non-linear multi-track video editor.
- **[steM.](https://github.com/MOzcelik14/steM)**: Multitrack audio workstation and waveform editor.
- **[beaM.](https://github.com/MOzcelik14/beaM)**: Non-destructive RAW photo editor and image studio.

Supported platforms: **Linux** (x86_64) and **Windows** (x86_64).

---

## Downloads & Installation

Pre-compiled binary releases are available on the [Releases](https://github.com/MOzcelik14/prism-suite/releases) page.

| Platform | Package Format | File Name | Notes |
| :--- | :--- | :--- | :--- |
| Debian / Ubuntu | DEB | `prism_<version>_amd64.deb` | Standard system package |
| Fedora / RHEL | RPM | `prism-<version>-1.x86_64.rpm` | Standard system package |
| Linux (Generic) | AppImage | `prism-<version>-x86_64.AppImage` | Standalone portable executable |
| Linux (Generic) | Tarball | `prism-<version>-linux-x86_64.tar.gz` | Portable archive |
| Windows | ZIP | `prism-<version>-windows-x86_64.zip` | Portable archive with `prism.exe` |

### Debian / Ubuntu (.deb)

```bash
sudo dpkg -i prism_<version>_amd64.deb
sudo apt-get install -f
```

### Fedora / RHEL (.rpm)

```bash
sudo dnf install ./prism-<version>-1.x86_64.rpm
```

### Generic Linux (.AppImage)

```bash
chmod +x prism-<version>-x86_64.AppImage
./prism-<version>-x86_64.AppImage
```

---

## Technical Overview

### Security and Release Integrity
- **Ed25519 & Minisign Signatures**: Release manifests are verified against `SHA256SUMS` and detached Minisign signatures prior to unpacking.
- **Archive Sanitization**: Archive unpacking rejects path traversal attempts (`../`), absolute targets, and symlink escapes outside the target destination.
- **Memory-Safe Network Layer**: Built on `reqwest` with `rustls-tls` to avoid external OpenSSL runtime dependencies.

### Installation Lifecycle
- **Atomic Updates & Instant Rollbacks**: Applications are deployed via atomic symlinks on Linux and directory junctions on Windows (`versions/<version>/` linked to `current`). Prior versions are retained for one-click rollbacks.
- **Detached Process Management**: Managed applications launch in independent process groups (`setsid` on Linux, detached process creation flags on Windows), remaining active across launcher shutdowns.
- **Desktop Integration**: Installs FreeDesktop `.desktop` entries in `~/.local/share/applications/` on Linux and shortcuts on Windows.
- **Language Synchronization**: Manages `<config>/prism/shared.ron` to synchronize user language preferences (English and Turkish) across the entire suite.

---

## Workspace Structure

```
prism-suite/
├── crates/
│   ├── prism-app/       # GUI front-end built on egui and eframe 0.29
│   ├── prism-core/      # Catalog, application models, state, and shared preferences
│   ├── prism-net/       # HTTP release discovery, streaming downloads, hash and signature verifier
│   ├── prism-install/   # Symlinks, archive extraction, desktop entries, process supervisor
│   └── prism-i18n/      # Fluent localization bundles (English and Turkish)
├── assets/              # Desktop integration files, vector logos, and raster icon set
├── docs/                # Architecture specifications, signing guide, and release format
├── scripts/
│   ├── package-linux.sh # Linux packaging script (deb, rpm, appimage, tarball)
│   ├── package.sh       # Release packaging frontend
│   └── sign-release.sh  # Checksum computation and Minisign signing script
├── .github/workflows/
│   ├── ci.yml           # Continuous integration (tests, clippy, formatting)
│   └── release.yml      # Multiplatform release matrix (deb, rpm, appimage, zip)
├── Cargo.toml
├── LICENSE
└── README.md
```

---

## Building from Source

### Prerequisites
- Rust stable toolchain (1.80 or newer):
  ```bash
  rustup default stable
  ```
- Linux development dependencies:
  ```bash
  sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libasound2-dev
  ```

### Build Commands

```bash
# Debug build
cargo run -p prism-app

# Release build
cargo build --release -p prism-app
```

The resulting executable will be located at `target/release/prism` (or `target/release/prism.exe` on Windows).

### Tests and Quality Checks

```bash
# Workspace unit and integration tests
cargo test --workspace

# Linting with Clippy
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --check
```

---

## Packaging

To produce distribution packages locally:

```bash
# Linux (builds tarball and deb/rpm if packaging utilities are available)
./scripts/package.sh all

# Build specific package format
./scripts/package-linux.sh deb
./scripts/package-linux.sh rpm
./scripts/package-linux.sh appimage
./scripts/package-linux.sh tarball
```

Release packages are placed into the `dist/` directory.

---

## License

GNU General Public License v3.0 (GPL-3.0-only). See [LICENSE](LICENSE) for details.

Copyright (C) 2026 Murat Özçelik.
