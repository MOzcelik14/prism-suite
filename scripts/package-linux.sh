#!/usr/bin/env bash
# Copyright (C) 2026 Murat Özçelik <mozcelik14>
# SPDX-License-Identifier: GPL-3.0-only
#
# Build Linux packages for prisM. Suite (deb, rpm, appimage, tarball)
# Usage: bash scripts/package-linux.sh deb|rpm|appimage|tarball

set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FORMAT="${1:-}"
case "$FORMAT" in
  deb|rpm|appimage|tarball) ;;
  *)
    echo "Usage: $0 deb|rpm|appimage|tarball" >&2
    exit 2
    ;;
esac

VERSION="$(sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"$/\1/p' Cargo.toml | head -n1)"
if [[ -z "$VERSION" ]]; then
  echo "Error: Cannot determine Cargo workspace version from Cargo.toml" >&2
  exit 1
fi

if [[ "${GITHUB_REF_TYPE:-}" == "tag" && "${GITHUB_REF_NAME:-}" != "v$VERSION" ]]; then
  echo "Warning: Tag ${GITHUB_REF_NAME} does not match Cargo version v$VERSION" >&2
fi

BIN="$ROOT/target/release/prism"
DESKTOP="$ROOT/assets/prism.desktop"
METAINFO="$ROOT/assets/prism.metainfo.xml"
ICON="$ROOT/assets/prism.svg"
ICON_PNG="$ROOT/assets/prism.png"
DIST="$ROOT/dist"

mkdir -p "$DIST"

for path in "$BIN" "$DESKTOP" "$METAINFO" "$ICON" "$ICON_PNG"; do
  [[ -f "$path" ]] || { echo "Missing required input: $path" >&2; exit 1; }
done
[[ -x "$BIN" ]] || { echo "Binary not executable: $BIN" >&2; exit 1; }

# Sync AppStream release metadata with current version
METAINFO_SOURCE="$METAINFO"
METAINFO_STAGE="$ROOT/build/linux/prism.metainfo.xml"
mkdir -p "$(dirname "$METAINFO_STAGE")"

python3 - "$METAINFO_SOURCE" "$METAINFO_STAGE" "$VERSION" <<'PY'
import datetime
import sys
import xml.etree.ElementTree as ET

source, destination, version = sys.argv[1:]
tree = ET.parse(source)
root = tree.getroot()
releases = root.find("releases")
if releases is None:
    releases = ET.SubElement(root, "releases")

existing = [r.get("version") for r in releases.findall("release")]
if version not in existing:
    release_el = ET.Element("release", {"version": version, "date": datetime.date.today().isoformat()})
    releases.insert(0, release_el)

tree.write(destination, encoding="utf-8", xml_declaration=True)
PY

install_payload() {
  local root="$1"
  install -Dm755 "$BIN" "$root/usr/bin/prism"
  install -Dm644 "$DESKTOP" "$root/usr/share/applications/prism.desktop"
  install -Dm644 "$METAINFO_STAGE" "$root/usr/share/metainfo/prism.metainfo.xml"
  install -Dm644 "$ICON" "$root/usr/share/icons/hicolor/scalable/apps/prism.svg"

  for size in 16 32 48 64 128 256 512; do
    local image="$ROOT/assets/icons/prism_${size}x${size}.png"
    if [[ -f "$image" ]]; then
      install -Dm644 "$image" "$root/usr/share/icons/hicolor/${size}x${size}/apps/prism.png"
    fi
  done
}

write_runtime_launcher() {
  local target_script="$1"
  local rel_bin="$2"
  cat > "$target_script" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

for libdir in "$HERE/usr/lib" "$HERE/usr/lib/x86_64-linux-gnu" "$HERE/lib"; do
  if [[ -d "$libdir" ]]; then
    export LD_LIBRARY_PATH="$libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  fi
done

for bindir in "$HERE/usr/bin" "$HERE/bin"; do
  if [[ -d "$bindir" ]]; then
    export PATH="$bindir:$PATH"
  fi
done

exec "$HERE/usr/bin/prism" "$@"
EOF
  chmod +x "$target_script"
}

build_deb() {
  command -v dpkg-deb >/dev/null || { echo "dpkg-deb not found" >&2; exit 1; }
  local stage="$ROOT/build/linux/deb"
  rm -rf "$stage"
  install_payload "$stage"

  mkdir -p "$stage/DEBIAN"
  cat > "$stage/DEBIAN/control" <<EOF
Package: prism
Version: $VERSION
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Murat Özçelik <mozcelik14@gmail.com>
Depends: libc6 (>= 2.31)
Recommends: libx11-6, libxcursor1, libxrandr2, libxi6, libgl1
Homepage: https://github.com/MOzcelik14/prism-suite
Description: Unified desktop launcher and package manager for prisM creative suite
 prisM. Suite is a native launcher, installer, and auto-updater for
 the creative suite desktop applications (triM., steM., beaM.).
EOF

  dpkg-deb --build --root-owner-group "$stage" "$DIST/prism_${VERSION}_amd64.deb"
  dpkg-deb --info "$DIST/prism_${VERSION}_amd64.deb"
  dpkg-deb --contents "$DIST/prism_${VERSION}_amd64.deb" | grep -F '/usr/bin/prism'
  echo "Debian package created: $DIST/prism_${VERSION}_amd64.deb"
}

build_rpm() {
  command -v rpmbuild >/dev/null || { echo "rpmbuild not found" >&2; exit 1; }
  local top="$ROOT/build/linux/rpmbuild"
  rm -rf "$top"
  mkdir -p "$top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

  install -m755 "$BIN" "$top/SOURCES/prism"
  cp "$DESKTOP" "$top/SOURCES/prism.desktop"
  cp "$METAINFO_STAGE" "$top/SOURCES/prism.metainfo.xml"
  cp "$ICON" "$top/SOURCES/prism.svg"

  for size in 16 32 48 64 128 256 512; do
    local image="$ROOT/assets/icons/prism_${size}x${size}.png"
    if [[ -f "$image" ]]; then
      cp "$image" "$top/SOURCES/prism_${size}.png"
    fi
  done

  cat > "$top/SPECS/prism.spec" <<EOF
Name: prism
Version: $VERSION
Release: 1
Summary: Unified launcher and manager for prisM creative suite
License: GPL-3.0-only
URL: https://github.com/MOzcelik14/prism-suite
BuildArch: x86_64
%global debug_package %{nil}

%description
prisM. Suite is a native desktop launcher, package manager, and auto-updater for
triM., steM., and beaM.

%prep

%build

%install
install -Dm755 %{_sourcedir}/prism %{buildroot}%{_bindir}/prism
install -Dm644 %{_sourcedir}/prism.desktop %{buildroot}%{_datadir}/applications/prism.desktop
install -Dm644 %{_sourcedir}/prism.metainfo.xml %{buildroot}%{_datadir}/metainfo/prism.metainfo.xml
install -Dm644 %{_sourcedir}/prism.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/prism.svg
EOF

  for size in 16 32 48 64 128 256 512; do
    if [[ -f "$top/SOURCES/prism_${size}.png" ]]; then
      echo "install -Dm644 %{_sourcedir}/prism_${size}.png %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/prism.png" >> "$top/SPECS/prism.spec"
    fi
  done

  cat >> "$top/SPECS/prism.spec" <<'EOF'

%files
%{_bindir}/prism
%{_datadir}/applications/prism.desktop
%{_datadir}/metainfo/prism.metainfo.xml
%{_datadir}/icons/hicolor/

%changelog
EOF

  rpmbuild -bb --define "_topdir $top" "$top/SPECS/prism.spec"
  find "$top/RPMS" -type f -name '*.rpm' -exec cp -v {} "$DIST/" \;
  echo "RPM package created in $DIST"
}

build_appimage() {
  local tools="$ROOT/build/linux/tools"
  local work="$ROOT/build/linux/appimage"
  local appdir="$work/prisM.AppDir"
  rm -rf "$work"
  mkdir -p "$tools" "$work"
  install_payload "$appdir"

  install -Dm644 "$ICON_PNG" "$appdir/prism.png"
  ln -sf "prism.png" "$appdir/.DirIcon"

  local custom_apprun="$work/AppRun.custom"
  write_runtime_launcher "$custom_apprun" "usr/bin/prism"

  local linuxdeploy="$tools/linuxdeploy-x86_64.AppImage"
  if [[ ! -s "$linuxdeploy" ]]; then
    echo "Downloading linuxdeploy..."
    curl --fail --location --retry 3 --silent --show-error \
      "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage" \
      --output "$linuxdeploy"
    chmod +x "$linuxdeploy"
  fi

  (
    cd "$work"
    export ARCH=x86_64 VERSION="$VERSION" LINUXDEPLOY_OUTPUT_VERSION="$VERSION" APPIMAGE_EXTRACT_AND_RUN=1
    "$linuxdeploy" --appdir "$appdir" \
      --executable "$BIN" \
      --desktop-file "$DESKTOP" \
      --icon-file "$ICON" \
      --icon-file "$ICON_PNG" \
      --custom-apprun "$custom_apprun" \
      --output appimage
  )

  local output
  output="$(find "$work" -maxdepth 1 -type f -name '*.AppImage' -print -quit)"
  [[ -n "$output" ]] || { echo "Error: linuxdeploy did not produce an AppImage" >&2; exit 1; }

  mv "$output" "$DIST/prism-${VERSION}-x86_64.AppImage"
  chmod +x "$DIST/prism-${VERSION}-x86_64.AppImage"
  echo "AppImage created: $DIST/prism-${VERSION}-x86_64.AppImage"
}

build_tarball() {
  local stage_parent="$ROOT/build/linux/tarball"
  local folder_name="prism-${VERSION}-linux-x86_64"
  local stage="$stage_parent/$folder_name"
  rm -rf "$stage_parent"
  mkdir -p "$stage"

  cp "$BIN" "$stage/prism"
  cp -r "$ROOT/assets" "$stage/assets"
  cp "$ROOT/README.md" "$stage/README.md"
  cp "$ROOT/LICENSE" "$stage/LICENSE"

  tar -C "$stage_parent" -czf "$DIST/${folder_name}.tar.gz" "$folder_name"
  echo "Tarball created: $DIST/${folder_name}.tar.gz"
}

case "$FORMAT" in
  deb) build_deb ;;
  rpm) build_rpm ;;
  appimage) build_appimage ;;
  tarball) build_tarball ;;
esac

echo "==> Package build completed for $FORMAT (v$VERSION)"
