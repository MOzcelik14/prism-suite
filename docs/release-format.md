# prisM. Suite — Release Package & Update Format Specification

## 1. Overview
The **prisM. Suite** launcher distributes, installs, updates, and rolls back creative applications (**triM.**, **steM.**, **beaM.**) using cryptographically signed, reproducible release bundles published via GitHub Releases.

This specification describes the archive packaging structure, checksum manifests, Ed25519 digital signatures, manifest schemas, and local directory layouts.

---

## 2. Archive Naming Convention

Release packages follow a strict artifact naming pattern:

```
<app_id>-<version>-<target_os>-<target_arch>.<extension>
```

### Parameters:
- `<app_id>`: `trim`, `stem`, `beam` (all lowercase, no dots)
- `<version>`: Semantic Versioning without leading `v` (e.g., `0.1.0`, `1.2.3-rc.1`)
- `<target_os>`:
  - `linux`: GNU/Linux (glibc / musl standard binaries)
  - `windows`: Microsoft Windows
- `<target_arch>`:
  - `x86_64`: 64-bit x86 architecture
  - `aarch64`: 64-bit ARM architecture
- `<extension>`:
  - `.tar.gz`: Standard archive format for Linux
  - `.zip`: Standard archive format for Windows

### Examples:
- `trim-0.1.0-linux-x86_64.tar.gz`
- `stem-0.1.0-linux-x86_64.tar.gz`
- `beam-0.1.0-windows-x86_64.zip`

---

## 3. Archive Internal Layout

Each release archive extracts to a root folder corresponding to `<app_id>-<version>`:

```
trim-0.1.0/
├── bin/
│   └── trim (or trim.exe on Windows)
├── share/
│   ├── icons/
│   │   ├── 128x128/trim.png
│   │   └── scalable/trim.svg
│   └── applications/
│       └── trim.desktop
├── locales/
│   ├── en.ftl
│   └── tr.ftl
├── version.txt
├── app.ron
└── LICENSE
```

### Key Files:
- **`bin/<binary_name>`**: The executable binary for the app.
- **`version.txt`**: Plain text file containing the exact semver string (e.g. `0.1.0\n`).
- **`app.ron`**: Complete machine-readable metadata descriptor for the release.

---

## 4. Release Manifest (`app.ron`)

An optional or embedded `app.ron` describes the release metadata:

```ron
(
    app_id: Trim,
    version: "0.1.0",
    release_date: "2026-10-29",
    minimum_launcher_version: "0.1.0",
    notes: "Initial release featuring timeline multi-track cutting, transition effects, and hardware-accelerated preview.",
    executable_relative_path: "bin/trim",
    assets: [
        (
            os: linux,
            arch: x86_64,
            filename: "trim-0.1.0-linux-x86_64.tar.gz",
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            size_bytes: 42105432,
            download_url: "https://github.com/MOzcelik14/triM/releases/download/v0.1.0/trim-0.1.0-linux-x86_64.tar.gz",
        ),
        (
            os: windows,
            arch: x86_64,
            filename: "trim-0.1.0-windows-x86_64.zip",
            sha256: "f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2",
            size_bytes: 45210984,
            download_url: "https://github.com/MOzcelik14/triM/releases/download/v0.1.0/trim-0.1.0-windows-x86_64.zip",
        ),
    ]
)
```

---

## 5. Security & Verification Architecture

Integrity and authenticity are verified at multiple levels before any downloaded code is extracted or executed.

### 5.1 Checksums Manifest (`SHA256SUMS`)
Each GitHub Release must publish a `SHA256SUMS` file formatted according to standard GNU coreutils:

```text
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  trim-0.1.0-linux-x86_64.tar.gz
f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2  trim-0.1.0-windows-x86_64.zip
```

### 5.2 Digital Signature (`SHA256SUMS.sig`)
The `SHA256SUMS` file is signed using an **Ed25519** signing key in Minisign-compatible format:

```text
untrusted comment: signature from prism secret key
RWR7Xk...[64-byte base64 encoded signature]...
trusted comment: timestamp:1793262000
...
```

### 5.3 Verification Workflow
1. Download `SHA256SUMS` and `SHA256SUMS.sig`.
2. Verify `SHA256SUMS.sig` against `SHA256SUMS` using the embedded `PRISM_PUBLIC_KEY`. If verification fails, abort immediately with `PrismError::Security`.
3. Download target archive `<app_id>-<version>-<os>-<arch>.(tar.gz|zip)`.
4. Calculate SHA-256 of the downloaded archive and verify exact match with `SHA256SUMS`.
5. Extract archive into temporary staging directory `<apps_root>/<app_id>/staging/<version>`.
6. Atomic swap into `<apps_root>/<app_id>/versions/<version>`.

---

## 6. Local Installation & Rollback Structure

Applications are installed in dedicated directories inside the platform application data store:
- **Linux**: `~/.local/share/prism-suite/apps/<app_id>/`
- **Windows**: `%LOCALAPPDATA%\prism-suite\apps\<app_id>\`

### Directory Layout:
```
apps/
└── stem/
    ├── current -> versions/0.2.0/ (symlink or junction)
    ├── previous -> versions/0.1.0/
    └── versions/
        ├── 0.1.0/
        │   ├── bin/stem
        │   └── version.txt
        └── 0.2.0/
            ├── bin/stem
            └── version.txt
```

### Atomic Updates & Safe Rollback:
- When a new update arrives, it is installed into `versions/<new_version>/`.
- The `previous` link is updated to point to the currently active version.
- The `current` link is atomically switched to point to `versions/<new_version>/`.
- If the newly updated app crashes on first launch or fails verification, the launcher automatically rolls back the `current` symlink to `previous`.
- The user can also choose "Rollback to previous version" in the launcher UI.
