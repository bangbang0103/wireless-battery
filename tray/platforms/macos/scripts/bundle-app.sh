#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TRAY_DIR="$(cd "${MACOS_DIR}/../.." && pwd)"

APP_NAME="Wireless Battery.app"
DIST_DIR="${MACOS_DIR}/dist"
APP_DIR="${DIST_DIR}/${APP_NAME}"

echo "==> Building Rust C-ABI static library (wireless-battery-tray-ffi)..."
(cd "${TRAY_DIR}" && cargo build --locked --release -p wireless-battery-tray-ffi "$@")

echo "==> Building macOS SwiftUI menu bar executable..."
(cd "${MACOS_DIR}" && swift build -c release)

BIN_PATH="$(cd "${MACOS_DIR}" && swift build -c release --show-bin-path)/WirelessBatteryTray"

echo "==> Assembling ${APP_DIR}..."
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS" "${APP_DIR}/Contents/Resources"
cp "${BIN_PATH}" "${APP_DIR}/Contents/MacOS/WirelessBatteryTray"
cp "${MACOS_DIR}/Resources/Info.plist" "${APP_DIR}/Contents/Info.plist"
cp "${MACOS_DIR}/Resources/AppIcon.icns" "${APP_DIR}/Contents/Resources/AppIcon.icns"

if command -v codesign >/dev/null 2>&1; then
  echo "==> Ad-hoc signing ${APP_NAME}..."
  codesign --force --deep --sign - "${APP_DIR}"
fi

echo "Done: ${APP_DIR}"
