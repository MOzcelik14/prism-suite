# prisM. Suite

Unified launcher, installer, and updater for the **prisM. Creative Desktop Application Suite**:
- **triM.** — Precision non-linear video editing for desktop ([Repository](https://github.com/MOzcelik14/triM))
- **steM.** — Multitrack audio workstation & precision waveform editor ([Repository](https://github.com/MOzcelik14/steM))
- **beaM.** — Creative photo editing & raster design studio ([Repository](https://github.com/MOzcelik14/beaM))

Target platforms: **Linux** and **Windows**.

---

## Branding
- **Suite display name**: `prisM. Suite` (used in window titles, installer, about dialog, docs).
- **Launcher display name**: `prisM.` (used in compact places: taskbar, tray, shortcuts).
- **Component naming**: `triM.`, `steM.`, `beaM.` (always retrieved dynamically from the catalog; never hardcoded in UI strings).

---

## Workspace Architecture

```
prism-suite/
├── crates/
│   ├── prism-app/       # Desktop application UI (egui/eframe) with custom vector icons & dark theme
│   ├── prism-core/      # Branding constants, embedded RON app catalog, install states, config
│   ├── prism-net/       # HTTP release client (reqwest + rustls, no OpenSSL), Ed25519 signatures
│   ├── prism-install/   # Local installation manager, directory resolver, archive extraction, rollback
│   └── prism-i18n/      # Fluent localization (English & Turkish) with 100% key-parity enforcement
├── docs/
│   └── release-format.md # Specification of release archives, SHA256SUMS, and Ed25519 signatures
├── Cargo.toml
└── README.md
```

---

## Features (Milestone 1)

1. **Embedded App Catalog**:
   - Single source of truth in `catalog.ron` loaded into `AppCatalog`.
   - Dynamic query of app names, taglines, categories, and repositories.
2. **Modern Dark Theme UI**:
   - Clean obsidian & slate visual style with custom vector badges.
   - Distinctive vector logos for `triM.`, `steM.`, `beaM.`, and `prisM.` featuring the signature capital "M" and trailing-dot motif.
   - Interactive demo states for Milestone 1 (Not Installed, Installed, Update Available, Running).
3. **Dual-Language Localization (English & Turkish)**:
   - Powered by Fluent (`locales/en.ftl`, `locales/tr.ftl`).
   - Instant language switching in Settings with state persistence (`config.ron`).
   - Full Latin Extended-A coverage for Turkish characters (`ç, ğ, ı, İ, ö, ş, ü`).
   - Automated 100% key parity test.
4. **Security & Cryptography Foundation**:
   - Zero OpenSSL dependency (`reqwest` with `rustls-tls`).
   - Ed25519 public key verification framework with compile-time configurable key.

---

## Building & Running

### Requirements:
- Rust toolchain (stable 1.80+)
- Linux build dependencies: standard X11 / Wayland development libraries (e.g. `libxkbcommon-dev`, `libxcb1-dev` if compiling on Linux)

### Run the Launcher:
```bash
cargo run -p prism-app
```

### Run All Workspace Tests:
```bash
cargo test --workspace
```

### Run Clippy (Strict Zero-Warning Policy):
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Check Formatting:
```bash
cargo fmt --check
```

---

## License
Licensed under **GNU General Public License v3.0 (GPL-3.0-only)**.
Copyright © Murat Özçelik. All rights reserved.
