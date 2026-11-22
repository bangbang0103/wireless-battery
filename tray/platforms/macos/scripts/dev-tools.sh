#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
export WB_DEV_TOOLS=1

bash "${SCRIPT_DIR}/bundle-app.sh"
open "${MACOS_DIR}/dist/Wireless Battery.app"
