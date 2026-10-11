#!/usr/bin/env bash
# RISE format registration for Linux (shared-mime-info)
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"

if [ "$(id -u)" -ne 0 ]; then
  sudo cp "$DIR/rise.xml" /usr/share/mime/packages/rise.xml
  sudo update-mime-database /usr/share/mime
else
  cp "$DIR/rise.xml" /usr/share/mime/packages/rise.xml
  update-mime-database /usr/share/mime
fi

echo "RISE format registered. .rise files now recognized as image/jpeg + video/mp4."
