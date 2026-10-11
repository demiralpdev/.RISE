#!/bin/bash
# RISE uninstaller (macOS) — removes everything the installer put in place.
set -u

UID_=$(id -u)

echo "== RISE uninstall =="

# 1. Stop the daemon
launchctl bootout "gui/$UID_" /Library/LaunchAgents/org.rise.autoseal.plist 2>/dev/null \
  || launchctl remove org.rise.autoseal 2>/dev/null

# 2. Remove system files (needs sudo)
sudo rm -f /Library/LaunchAgents/org.rise.autoseal.plist
sudo rm -rf "/Library/Application Support/RISE"
sudo rm -f /usr/local/bin/rise-core

# 3. Remove apps
rm -rf "/Applications/RISE Format.app" "/Applications/RISE Viewer.app"

# 4. Keep user data (~/.rise keys + logs) unless --purge
if [ "${1:-}" = "--purge" ]; then
    rm -rf "$HOME/.rise"
    echo "Removed ~/.rise (keys + logs) too."
else
    echo "Kept ~/.rise (keys + logs). Use 'uninstall.sh --purge' to remove those too."
fi

# 5. Tell LaunchServices
LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
"$LSREGISTER" -u "/Applications/RISE Format.app" 2>/dev/null
"$LSREGISTER" -u "/Applications/RISE Viewer.app" 2>/dev/null

echo "Done. .rise files remain valid JPEGs/MP4s — they open with any image viewer."
