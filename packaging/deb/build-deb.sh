#!/usr/bin/env bash
set -euo pipefail

VERSION="0.1.0"
PACKAGE_NAME="verci"
DEB_DIR="packaging/deb/${PACKAGE_NAME}_${VERSION}_amd64"

echo "==> Building Verci Release Binary..."
cargo build --release

rm -rf "$DEB_DIR"
mkdir -p "$DEB_DIR/DEBIAN"
mkdir -p "$DEB_DIR/usr/bin"
mkdir -p "$DEB_DIR/usr/share/applications"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/scalable/apps"

cat << EOF > "$DEB_DIR/DEBIAN/control"
Package: ${PACKAGE_NAME}
Version: ${VERSION}
Section: sound
Priority: optional
Architecture: amd64
Depends: libgtk-4-1, libadwaita-1-0, libpango-1.0-0, libcairo2
Maintainer: Antigravity <antigravity@deepmind.google.com>
Description: Synchronized animated song lyrics desktop visualizer
 Verci is a lightweight Linux desktop application that detects currently
 playing music via MPRIS and displays synchronized, animated lyrics in
 cinematic 3D, Fisheye shader, and contextual visual symbol modes.
EOF

cp target/release/verci "$DEB_DIR/usr/bin/"
cp packaging/xyz.verci.linux.desktop "$DEB_DIR/usr/share/applications/"
cp packaging/icons/xyz.verci.linux.svg "$DEB_DIR/usr/share/icons/hicolor/scalable/apps/"

if command -v dpkg-deb >/dev/null 2>&1; then
    echo "==> Building .deb package..."
    dpkg-deb --build --root-owner-group "$DEB_DIR"
else
    echo "Notice: dpkg-deb not present on Arch host; Debian file tree created at $DEB_DIR"
fi
