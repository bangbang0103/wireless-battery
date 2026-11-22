#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TRAY_DIR="$(cd "${MACOS_DIR}/../.." && pwd)"

APP_NAME="Wireless Battery.app"
DIST_DIR="${MACOS_DIR}/dist"
APP_DIR="${DIST_DIR}/${APP_NAME}"

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"

CARGO_ARGS=("$@")
HAS_DEV_TOOLS=0
PREV_WAS_FEATURES=0
for arg in "$@"; do
  if [[ "${PREV_WAS_FEATURES}" -eq 1 ]]; then
    if [[ ",${arg}," == *",dev-tools,"* ]] || [[ " ${arg} " == *" dev-tools "* ]]; then
      HAS_DEV_TOOLS=1
    fi
    PREV_WAS_FEATURES=0
    continue
  fi
  case "${arg}" in
    --features)
      PREV_WAS_FEATURES=1
      ;;
    --features=*)
      val="${arg#--features=}"
      if [[ ",${val}," == *",dev-tools,"* ]] || [[ " ${val} " == *" dev-tools "* ]]; then
        HAS_DEV_TOOLS=1
      fi
      ;;
    --all-features)
      HAS_DEV_TOOLS=1
      ;;
  esac
done

if [[ "${WB_DEV_TOOLS:-0}" == "1" ]] && [[ "${HAS_DEV_TOOLS}" -eq 0 ]]; then
  CARGO_ARGS+=(--features dev-tools)
elif [[ "${HAS_DEV_TOOLS}" -eq 1 ]]; then
  export WB_DEV_TOOLS=1
fi

CARGO_VERSION="$(awk -F'"' '/^version *= *"/{print $2; exit}' "${TRAY_DIR}/Cargo.toml")"
APP_VERSION="${APP_VERSION:-${CARGO_VERSION}}"
if [[ -z "${APP_VERSION}" ]]; then
  echo "error: failed to read workspace version from ${TRAY_DIR}/Cargo.toml" >&2
  exit 1
fi

echo "==> Building Rust C-ABI static library (wireless-battery-tray-ffi)..."
if [[ "${#CARGO_ARGS[@]}" -gt 0 ]]; then
  (cd "${TRAY_DIR}" && cargo build --locked --release -p wireless-battery-tray-ffi "${CARGO_ARGS[@]}")
else
  (cd "${TRAY_DIR}" && cargo build --locked --release -p wireless-battery-tray-ffi)
fi

export WB_RUST_LIB_DIR="${TRAY_DIR}/target/release"

echo "==> Building macOS SwiftUI menu bar executable..."
(cd "${MACOS_DIR}" && swift build -c release)

BIN_PATH="$(cd "${MACOS_DIR}" && swift build -c release --show-bin-path)/WirelessBatteryTray"

echo "==> Assembling ${APP_DIR}..."
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS" "${APP_DIR}/Contents/Resources"
cp "${BIN_PATH}" "${APP_DIR}/Contents/MacOS/WirelessBatteryTray"
cp "${MACOS_DIR}/Resources/Info.plist" "${APP_DIR}/Contents/Info.plist"
cp "${MACOS_DIR}/Resources/AppIcon.icns" "${APP_DIR}/Contents/Resources/AppIcon.icns"

if command -v /usr/libexec/PlistBuddy >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${APP_VERSION}" "${APP_DIR}/Contents/Info.plist"
fi

if command -v codesign >/dev/null 2>&1; then
  echo "==> Ad-hoc signing ${APP_NAME}..."
  codesign --force --deep --sign - "${APP_DIR}"
fi

echo "Done: ${APP_DIR} (version ${APP_VERSION})"
