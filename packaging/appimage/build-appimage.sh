#!/usr/bin/env bash
set -euo pipefail

echo "==> Building Verci Release Binary..."
cargo build --release

APP_DIR="packaging/appimage/AppDir"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/share/applications"
mkdir -p "$APP_DIR/usr/share/icons/hicolor/scalable/apps"

cp target/release/verci "$APP_DIR/usr/bin/"
cp packaging/xyz.verci.linux.desktop "$APP_DIR/xyz.verci.linux.desktop"
cp packaging/xyz.verci.linux.desktop "$APP_DIR/usr/share/applications/"
cp packaging/icons/xyz.verci.linux.svg "$APP_DIR/xyz.verci.linux.svg"
cp packaging/icons/xyz.verci.linux.svg "$APP_DIR/usr/share/icons/hicolor/scalable/apps/"

cat << 'EOF' > "$APP_DIR/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH:-}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "${HERE}/usr/bin/verci" "$@"
EOF

chmod +x "$APP_DIR/AppRun"

echo "==> AppDir structure prepared at $APP_DIR"
if command -v appimagetool >/dev/null 2>&1; then
    echo "==> Generating Verci-x86_64.AppImage..."
    appimagetool "$APP_DIR" Verci-x86_64.AppImage
else
    echo "Notice: appimagetool not installed in this environment; AppDir structure is ready."
fi
