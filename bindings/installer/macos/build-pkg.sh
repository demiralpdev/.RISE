#!/bin/bash
# Builds RISE-0.2.1.pkg — everything in one downloadable installer:
#   /usr/local/bin/rise-core                  (CLI)
#   /Applications/RISE Format.app            (system format registration, UTI)
#   /Applications/RISE Viewer.app             (.rise viewer + hash check)
#   /Library/Application Support/RISE/auto-seal.sh   (daemon script)
#   /Library/LaunchAgents/org.rise.autoseal.plist   (daemon autostart)
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$DIR/../../.." && pwd)"
STAGE="$DIR/.stage"
PAYLOAD="$STAGE/payload"
SCRIPTS="$STAGE/scripts"
VERSION="0.2.2"

rm -rf "$STAGE"
mkdir -p "$PAYLOAD/Applications" "$PAYLOAD/usr/local/bin" \
         "$PAYLOAD/Library/Application Support/RISE" \
         "$PAYLOAD/Library/LaunchAgents" "$SCRIPTS"

# 1. CLI binary
cp "$REPO/core/target/release/rise-core" "$PAYLOAD/usr/local/bin/rise-core"
chmod 755 "$PAYLOAD/usr/local/bin/rise-core"

# 2. Apps
if [ ! -d "$REPO/bindings/viewer/macos/RISE Viewer.app" ]; then
    "$REPO/bindings/viewer/macos/build.sh"
fi
cp -R "$REPO/bindings/system-format/macos/RISE Format.app" "$PAYLOAD/Applications/"
cp -R "$REPO/bindings/viewer/macos/RISE Viewer.app" "$PAYLOAD/Applications/"

# 3. Daemon script — patch RISE_BIN to the installed location
sed 's|RISE_BIN=".*"|RISE_BIN="/usr/local/bin/rise-core"|' \
    "$REPO/bindings/daemon/auto-seal.sh" > "$PAYLOAD/Library/Application Support/RISE/auto-seal.sh"
chmod 755 "$PAYLOAD/Library/Application Support/RISE/auto-seal.sh"

# 4. LaunchAgent — installed-location paths
cat > "$PAYLOAD/Library/LaunchAgents/org.rise.autoseal.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>org.rise.autoseal</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>/Library/Application Support/RISE/auto-seal.sh</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ThrottleInterval</key>
    <integer>30</integer>
    <key>StandardOutPath</key>
    <string>/tmp/org.rise.autoseal.out</string>
    <key>StandardErrorPath</key>
    <string>/tmp/org.rise.autoseal.err</string>
    <key>WorkingDirectory</key>
    <string>/tmp</string>
</dict>
</plist>
EOF

# 5. postinstall — register apps, start daemon
cat > "$SCRIPTS/postinstall" << 'EOF'
#!/bin/bash
set -u
LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
"$LSREGISTER" -f "/Applications/RISE Format.app" 2>/dev/null
"$LSREGISTER" -f "/Applications/RISE Viewer.app" 2>/dev/null
mkdir -p "$HOME/.rise/keys"
UID_=$(stat -f%u /dev/console 2>/dev/null || id -u)
launchctl bootout "gui/$UID_" /Library/LaunchAgents/org.rise.autoseal.plist 2>/dev/null
launchctl bootstrap "gui/$UID_" /Library/LaunchAgents/org.rise.autoseal.plist 2>/dev/null \
  || launchctl load /Library/LaunchAgents/org.rise.autoseal.plist 2>/dev/null
echo "RISE installed: rise-core + RISE Format + RISE Viewer + auto-seal daemon."
echo "If Desktop/Movies sealing is silent, grant Full Disk Access to /bin/bash once:"
echo "  System Settings > Privacy & Security > Full Disk Access > add /bin/bash"
exit 0
EOF
chmod 755 "$SCRIPTS/postinstall"

# 6. Build the pkg
OUT="$DIR/RISE-$VERSION.pkg"
pkgbuild \
    --root "$PAYLOAD" \
    --identifier "org.rise.pkg" \
    --version "$VERSION" \
    --scripts "$SCRIPTS" \
    "$OUT"

echo ""
echo "Built: $OUT"
pkgutil --check-signature "$OUT" 2>/dev/null || true
