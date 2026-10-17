#!/bin/bash
# RISE auto-seal daemon (macOS)
# Watches common capture folders and seals ANY new recording/screenshot
# with .rise proof using rise-core. Runs via LaunchAgent on login.
#
# Videos (.mp4/.mkv/.mov/.flv) -> rise-core video -> <file>.rise (manifest JSON)
# Images (.png/.jpg/.jpeg)     -> rise-core sign + pack -> <file>.rise
#   (JPEG -> packed JPEG; PNG -> manifest JSON copy, since pack is JPEG-only)
# Logs to ~/.rise/daemon.log

set -u

RISE_BIN="/Users/demir/Desktop/.RISE/core/target/release/rise-core"
KEY_DIR="$HOME/.rise/keys"
KEY_FILE="$KEY_DIR/auto-seal.key"
LOG_FILE="$HOME/.rise/daemon.log"
MARKER_FILE="$HOME/.rise/.autoseal-marker"
POLL_INTERVAL=5

WATCH_DIRS=("$HOME/Desktop" "$HOME/Movies" "$HOME/Movies/OBS" "$HOME/Documents")

log() {
  local ts
  ts="$(date '+%Y-%m-%d %H:%M:%S')"
  echo "[$ts] $*" >> "$LOG_FILE"
}

ensure_env() {
  mkdir -p "$KEY_DIR" "$(dirname "$LOG_FILE")"
  for d in "${WATCH_DIRS[@]}"; do
    mkdir -p "$d" 2>/dev/null || true
  done
  touch "$LOG_FILE"

  # Preflight: a LaunchAgent has no Terminal TCC grant, so macOS may deny
  # Desktop/Documents/Movies. Fail LOUD here instead of silently sealing nothing.
  for d in "${WATCH_DIRS[@]}"; do
    if ! ls "$d" >/dev/null 2>&1; then
      log "WARN no access to $d (macOS Privacy/TCC denied)"
      log "WARN fix (one-time): System Settings > Privacy & Security > Full Disk Access > add: /bin/bash, fswatch, rise-core — then: launchctl kickstart -k gui/$(id -u)/org.rise.autoseal"
    fi
  done

  if [ ! -f "$KEY_FILE" ]; then
    log "KEYGEN generating auto-seal keypair at $KEY_FILE"
    if "$RISE_BIN" keygen --out "${KEY_FILE%.key}" >> "$LOG_FILE" 2>&1; then
      chmod 600 "$KEY_FILE" 2>/dev/null || true
      log "KEYGEN done: $KEY_FILE"
    else
      log "ERROR keygen failed, sealing will be unsigned"
    fi
  fi

  # Marker so polling mode only picks up files created after daemon start.
  # Pre-existing files are skipped (their .rise would already exist anyway).
  if [ ! -f "$MARKER_FILE" ]; then
    touch "$MARKER_FILE"
  fi
}

lower_ext() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

is_video() {
  case "$(lower_ext "$1")" in
    *.mp4|*.mkv|*.mov|*.flv) return 0 ;;
    *) return 1 ;;
  esac
}

is_image() {
  case "$(lower_ext "$1")" in
    *.png|*.jpg|*.jpeg) return 0 ;;
    *) return 1 ;;
  esac
}

should_skip() {
  local f="$1"
  case "$f" in
    *.rise|*.rvid|*.manifest.json) return 0 ;;
  esac
  # Skip temp/partial files writers commonly use
  case "$(basename "$f")" in
    .*) return 0 ;;
    *.tmp|*.temp|*.part|*.crdownload|*.download|*.bak|*.swp|*~) return 0 ;;
  esac
  return 1
}

# Wait until file size is stable (recording still being written otherwise).
# Returns 0 when stable, 1 on timeout / missing file.
wait_stable() {
  local f="$1"
  local tries=0
  local prev=-1 cur=0
  sleep 3 # grace: recording might still be writing
  while [ $tries -lt 20 ]; do
    [ -f "$f" ] || return 1
    cur=$(stat -f%z "$f" 2>/dev/null || echo -1)
    if [ "$cur" = "$prev" ] && [ "$cur" -ge 0 ]; then
      return 0
    fi
    prev="$cur"
    tries=$((tries + 1))
    sleep 3
  done
  return 1
}

seal_video() {
  local f="$1"
  local out="${f}.rvid"
  log "SEAL-VIDEO start $f"
  # --capture-source screen is fail-closed: the daemon watches folders, so it
  # cannot know the origin. Labelling it non-camera proves the seal, never a
  # camera sensor, and blocks AI-generated drops from earning camera evidence.
  if "$RISE_BIN" video "$f" --device auto --key "$KEY_FILE" --capture-source screen --out "$out" >> "$LOG_FILE" 2>&1; then
    log "SEAL-VIDEO ok $f -> $out"
  else
    log "ERROR seal-video failed for $f (see above)"
    rm -f "$out"
  fi
}

seal_image() {
  local f="$1"
  local manifest="${f}.manifest.json"
  local out="${f}.rise"
  log "SEAL-IMAGE start $f"
  if ! "$RISE_BIN" sign "$f" --device auto --key "$KEY_FILE" --capture-source screen --out "$manifest" >> "$LOG_FILE" 2>&1; then
    log "ERROR sign failed for $f (see above)"
    rm -f "$manifest"
    return
  fi
  if "$RISE_BIN" pack "$f" "$manifest" --out "$out" >> "$LOG_FILE" 2>&1; then
    log "SEAL-IMAGE ok $f -> $out (packed, self-contained)"
  else
    log "ERROR pack failed for $f, keeping manifest as .rise fallback"
    cp -f "$manifest" "$out" && log "SEAL-IMAGE ok $f -> $out (manifest fallback)"
  fi
}

handle_file() {
  local f="$1"
  [ -f "$f" ] || return
  if should_skip "$f"; then
    return
  fi
  if ! is_video "$f" && ! is_image "$f"; then
    return
  fi
  # Already sealed and sealed file is newer than source -> skip
  local sealed="${f}.rvid"
  is_video "$f" || sealed="${f}.rise"
  if [ -f "$sealed" ] && [ "$sealed" -nt "$f" ]; then
    return
  fi
  log "DETECT new file $f"
  if ! wait_stable "$f"; then
    log "SKIP $f still changing or vanished"
    return
  fi
  if is_video "$f"; then
    seal_video "$f"
  else
    seal_image "$f"
  fi
}

poll_once() {
  local f
  while IFS= read -r -d '' f; do
    handle_file "$f"
  done < <(find "${WATCH_DIRS[@]}" -maxdepth 1 -type f \
    \( -iname '*.mp4' -o -iname '*.mkv' -o -iname '*.mov' -o -iname '*.flv' \
     -o -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' \) \
    -newer "$MARKER_FILE" -print0 2>/dev/null)
  touch "$MARKER_FILE"
}

poll_loop() {
  log "WATCH polling every ${POLL_INTERVAL}s (fswatch unavailable): ${WATCH_DIRS[*]}"
  while true; do
    poll_once
    sleep "$POLL_INTERVAL"
  done
}

fswatch_loop() {
  log "WATCH fswatch recursive: ${WATCH_DIRS[*]}"
  # Batch events: fire once per settled change, handle any new files found.
  # -0 NUL-delimited, -r recursive, -l latency for write-settling.
  fswatch -0 -r -l 5 "${WATCH_DIRS[@]}" 2>>"$LOG_FILE" | while IFS= read -r -d '' ev; do
    if [ -f "$ev" ]; then
      handle_file "$ev"
    elif [ -d "$ev" ]; then
      # A directory changed -> scan it for brand-new files (cheap, top-level only)
      poll_once
    fi
  done
}

main() {
  ensure_env
  log "START auto-seal daemon pid=$$ bin=$RISE_BIN"
  if command -v fswatch >/dev/null 2>&1; then
    fswatch_loop
  else
    log "WARN fswatch not found, falling back to polling"
    poll_loop
  fi
}

main "$@"
