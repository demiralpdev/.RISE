#!/bin/bash
# RISE Format — macOS system format registration
# Installs the RISE Format app bundle, registers .rise UTIs with LaunchServices,
# and verifies the registration with mdls.
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
APP_SRC="$DIR/RISE Format.app"
APP_DST="/Applications/RISE Format.app"
LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"

echo "== RISE Format — macOS install =="

# 1. Copy the app bundle
if [ -d "$APP_DST" ]; then
    rm -rf "$APP_DST"
fi
cp -R "$APP_SRC" "$APP_DST"
echo "Copied: $APP_DST"

# 2. Register with LaunchServices
"$LSREGISTER" -f "$APP_DST"
echo "LaunchServices registered."

# 3. Wait for the registration to settle
sleep 2

# 4. Verify: check a .rise file's content type
TEST_FILE="${1:-}"
if [ -z "$TEST_FILE" ]; then
    # Use the desktop sample if present, else create a stub
    if [ -f "$HOME/Desktop/ornek-foto.rise" ]; then
        TEST_FILE="$HOME/Desktop/ornek-foto.rise"
    else
        TEST_FILE=$(mktemp /tmp/rise-test.XXXXXX.rise)
        printf '\xFF\xD8\xFF\xE0' > "$TEST_FILE"
    fi
fi

echo "Verifying with: $TEST_FILE"
UTI=$(mdls -name kMDItemContentType -raw "$TEST_FILE" 2>/dev/null || echo "(null)")
echo "kMDItemContentType = $UTI"

if [ "$UTI" = "org.rise.sealed-image" ] || [ "$UTI" = "org.rise.sealed-video" ]; then
    echo "SUCCESS: .rise is now a registered system format."
else
    echo "Note: UTI shows as '$UTI'."
    echo "LaunchServices may need a moment, or Finder needs a restart to pick it up."
    echo "Try: killall Finder"
fi
