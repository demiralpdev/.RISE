#!/bin/bash
# RISE auto-seal daemon installer (macOS LaunchAgent)
# - verifies rise-core + fswatch
# - copies plist to ~/Library/LaunchAgents/
# - (re)loads it and verifies it's running
set -u

RISE_BIN="/Users/demir/Desktop/.RISE/core/target/release/rise-core"
SRC_DIR="/Users/demir/Desktop/.RISE/bindings/daemon"
PLIST_NAME="org.rise.autoseal.plist"
DEST_DIR="$HOME/Library/LaunchAgents"
DEST_PLIST="$DEST_DIR/$PLIST_NAME"
LABEL="org.rise.autoseal"

pass() { echo "  [OK] $*"; }
fail() { echo "  [FAIL] $*" >&2; exit 1; }

echo "== RISE auto-seal install =="

echo "[1/5] checking rise-core..."
[ -x "$RISE_BIN" ] || fail "rise-core not executable at $RISE_BIN (build core first)"
"$RISE_BIN" --help >/dev/null 2>&1 || fail "rise-core does not run"
pass "rise-core: $RISE_BIN"

echo "[2/5] checking fswatch..."
if ! command -v fswatch >/dev/null 2>&1; then
  echo "  fswatch missing, trying: brew install fswatch"
  if command -v brew >/dev/null 2>&1 && brew install fswatch; then
    pass "fswatch installed via brew"
  else
    echo "  [WARN] fswatch unavailable; daemon will use polling fallback (sleep 5)"
  fi
else
  pass "fswatch: $(command -v fswatch)"
fi

echo "[3/5] installing plist..."
chmod +x "$SRC_DIR/auto-seal.sh" "$SRC_DIR/install.sh"
bash -n "$SRC_DIR/auto-seal.sh" || fail "auto-seal.sh has syntax errors"
mkdir -p "$DEST_DIR" "$HOME/.rise/keys" "$HOME/Movies/OBS"
cp -f "$SRC_DIR/$PLIST_NAME" "$DEST_PLIST" || fail "copy to $DEST_PLIST failed"
pass "plist -> $DEST_PLIST"

echo "[4/5] (re)loading LaunchAgent..."
UID_NUM="$(id -u)"
# Unload any previous copy (ignore errors), then load fresh
launchctl bootout "gui/$UID_NUM/$LABEL" 2>/dev/null || true
launchctl unload "$DEST_PLIST" 2>/dev/null || true
if launchctl bootstrap "gui/$UID_NUM" "$DEST_PLIST" 2>/dev/null; then
  pass "bootstrap gui/$UID_NUM"
else
  launchctl load "$DEST_PLIST" 2>/dev/null || fail "launchctl load failed"
  pass "load (legacy)"
fi

echo "[5/5] verifying..."
sleep 2
if launchctl list | grep -q "$LABEL"; then
  pass "daemon running:"
  launchctl list | grep "$LABEL"
else
  fail "daemon not in launchctl list after load"
fi

echo ""
echo "Watching: ~/Desktop ~/Movies ~/Movies/OBS ~/Documents"
echo "Log:      ~/.rise/daemon.log"
tail -5 "$HOME/.rise/daemon.log" 2>/dev/null || echo "(log will appear on first run)"
echo ""
if grep -q "no access to" "$HOME/.rise/daemon.log" 2>/dev/null; then
  echo "  [WARN] macOS is denying the daemon folder access (TCC)."
  echo "  One-time fix: System Settings > Privacy & Security > Full Disk Access,"
  echo "  add: /bin/bash, $(command -v fswatch), $RISE_BIN"
  echo "  then: launchctl kickstart -k gui/$(id -u)/$LABEL"
else
  echo "  [OK] daemon can read all watch folders"
fi
echo ""
echo "Done. Test: drop a .mp4 into ~/Desktop and watch for ~/Desktop/<file>.mp4.rise"
