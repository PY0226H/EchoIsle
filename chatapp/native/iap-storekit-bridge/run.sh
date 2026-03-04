#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="$SCRIPT_DIR/.build/release/iap-storekit-bridge"

if [[ ! -x "$BIN_PATH" ]]; then
  xcrun swift build -c release --package-path "$SCRIPT_DIR"
fi

exec "$BIN_PATH" "$@"
