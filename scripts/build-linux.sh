#!/usr/bin/env bash
set -euo pipefail

cd /app

export CARGO_TARGET_DIR=/app/src-tauri/target-linux-x86_64

echo "==> npm install"
npm install

echo "==> tauri build (deb, appimage)"
npm run tauri build -- --bundles deb,appimage

echo "==> Build artifacts:"
find "$CARGO_TARGET_DIR/release/bundle" -maxdepth 2 -type f \( -name "*.deb" -o -name "*.AppImage" \)
