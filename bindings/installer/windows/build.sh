#!/bin/bash
# Build the RISE CLI Windows installer (RISE-Setup-<version>.exe) with NSIS.
#
# Usage:  ./build.sh [version]        (default version: 0.2.5)
#
# Prerequisites:
#   - makensis (NSIS 3.x)     e.g.  brew install makensis
#   - gh (GitHub CLI), authenticated, only if the payload must be downloaded
#
# The payload rel-rise-core-x86_64-pc-windows-gnu.exe is fetched from the
# matching GitHub release when it is not already present in this directory, so
# the repo never carries multi-megabyte binaries (see .gitignore).
#
# Source files (RISE-Setup.nsi) are UTF-8 and contain Turkish strings, so the
# compiler always runs with -INPUTCHARSET UTF8.
#
# Output: RISE-Setup-<version>.exe in this directory, plus its sha256.

set -euo pipefail

VERSION="${1:-0.2.5}"
DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

PAYLOAD="rel-rise-core-x86_64-pc-windows-gnu.exe"
OUT="RISE-Setup-${VERSION}.exe"

if ! command -v makensis >/dev/null 2>&1; then
  echo "error: makensis not found. Install NSIS (e.g. brew install makensis)." >&2
  exit 1
fi

if [ ! -f "$PAYLOAD" ]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "error: $PAYLOAD is missing and gh is unavailable to download it." >&2
    echo "       Download $PAYLOAD from release v$VERSION into $DIR." >&2
    exit 1
  fi
  echo "==> fetching $PAYLOAD from release v$VERSION"
  gh release download "v$VERSION" --pattern "$PAYLOAD" --dir "$DIR"
fi

if [ ! -f "$PAYLOAD" ]; then
  echo "error: $PAYLOAD still missing after download." >&2
  exit 1
fi

PAYLOAD_BYTES="$(wc -c < "$PAYLOAD" | tr -d '[:space:]')"
PAYLOAD_KB=$((PAYLOAD_BYTES / 1024))
echo "==> payload: $PAYLOAD (${PAYLOAD_BYTES} bytes, ${PAYLOAD_KB} KB)"

echo "==> building $OUT (version $VERSION)"
makensis -INPUTCHARSET UTF8 -DVERSION="$VERSION" -DPAYLOAD_KB="$PAYLOAD_KB" RISE-Setup.nsi

echo
echo "==> built: $DIR/$OUT"
shasum -a 256 "$OUT"
