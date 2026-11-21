#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ROOT_DIR="$(cd "${MACOS_DIR}/../../.." && pwd)"

APP_NAME="Wireless Battery.app"
APP_DIR="${MACOS_DIR}/dist/${APP_NAME}"
OUT_DMG="${1:-${ROOT_DIR}/dist/wireless-battery-macos.dmg}"

if [[ ! -d "${APP_DIR}" ]]; then
  bash "${SCRIPT_DIR}/bundle-app.sh"
fi

STAGING_DIR="$(mktemp -d)"
trap 'rm -rf "${STAGING_DIR}"' EXIT

cp -R "${APP_DIR}" "${STAGING_DIR}/${APP_NAME}"
ln -s /Applications "${STAGING_DIR}/Applications"
cp "${ROOT_DIR}/LICENSE" "${STAGING_DIR}/LICENSE"
cp "${ROOT_DIR}/NOTICE" "${STAGING_DIR}/NOTICE"

mkdir -p "$(dirname "${OUT_DMG}")"
rm -f "${OUT_DMG}"

echo "==> Creating DMG at ${OUT_DMG}..."
hdiutil create \
  -volname "Wireless Battery" \
  -srcfolder "${STAGING_DIR}" \
  -ov \
  -format UDZO \
  "${OUT_DMG}"

echo "Done: ${OUT_DMG}"
