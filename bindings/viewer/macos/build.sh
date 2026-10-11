#!/bin/bash
# Builds RISE Viewer.app (single-file AppKit, no Xcode project needed).
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
APP="$DIR/RISE Viewer.app"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"

swiftc -O -o "$APP/Contents/MacOS/RISE Viewer" "$DIR/main.swift" 2>&1

cat > "$APP/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>org.rise.viewer</string>
    <key>CFBundleName</key>
    <string>RISE Viewer</string>
    <key>CFBundleDisplayName</key>
    <string>RISE Viewer</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>RISE Viewer</string>
    <key>CFBundleVersion</key>
    <string>0.2.1</string>
    <key>CFBundleShortVersionString</key>
    <string>0.2.1</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>RISE Sealed Image</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>org.rise.sealed-image</string>
                <string>public.jpeg</string>
            </array>
        </dict>
        <dict>
            <key>CFBundleTypeName</key>
            <string>RISE Sealed Video</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>org.rise.sealed-video</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
EOF

plutil -lint "$APP/Contents/Info.plist"
echo "Built: $APP"
