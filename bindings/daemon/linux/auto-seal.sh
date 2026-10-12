#!/bin/bash
# RISE auto-seal daemon (Linux) — watches capture folders and seals any new
# photo/video with .rise/.rvid proof. Zero dependencies (pure polling).
#
# Videos (.mp4/.mkv/.mov/.webm) -> rise-core video -> <file>.rvid (playable MP4)
# Images (.png/.jpg/.jpeg)     -> rise-core sign + pack -> <file>.rise
# Runs as a systemd USER service (no root needed). Logs to ~/.rise/daemon.log

set -u

RISE_BIN="$HOME/.local/bin/rise-core"
KEY_DIR="$HOME/.rise/keys"
KEY_FILE="$KEY_DIR/auto-seal.key"
LOG_FILE="$HOME/.rise/daemon.log"
MARKER_FILE="$HOME/.rise/.autoseal-marker"
POLL_INTERVAL=5

xdg_dir() {
  if command -v xdg-user-dir >/dev/null 2>&1; then
    xdg-user-dir "$1" 2>/dev/null
  fi
}
DESKTOP_DIR="$(xdg_dir DESKTOP)"; [ -n "$DESKTOP_DIR" ] || DESKTOP_DIR="$HOME/Desktop"
VIDEOS_DIR="$(xdg_dir VIDEOS)"; [ -n "$VIDEOS_DIR" ] || VIDEOS_DIR="$HOME/Videos"
PICTURES_DIR="$(xdg_dir PICTURES)"; [ -n "$PICTURES_DIR" ] || PICTURES_DIR="$HOME/Pictures"
DOCUMENTS_DIR="$(xdg_dir DOCUMENTS)"; [ -n "$DOCUMENTS_DIR" ] || DOCUMENTS_DIR="$HOME/Documents"

WATCH_DIRS=("$DESKTOP_DIR" "$VIDEOS_DIR" "$DOCUMENTS_DIR" "$PICTURES_DIR" "$PICTURES_DIR/Screenshots" "$HOME/Desktop")

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
  if [ ! -f "$KEY_FILE" ]; then
    log "KEYGEN generating auto-seal keypair at $KEY_FILE"
    if "$RISE_BIN" keygen --out "${KEY_FILE%.key}" >> "$LOG_FILE" 2>&1; then
      chmod 600 "$KEY_FILE" 2>/dev/null || true
      log "KEYGEN done: $KEY_FILE"
    else
      log "ERROR keygen failed, sealing will be unsigned"
    fi
  fi
  if [ ! -f "$MARKER_FILE" ]; then
    touch "$MARKER_FILE"
  fi
}

lower_ext() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

is_video() {
  case "$(lower_ext "$1")" in
    *.mp4|*.mkv|*.mov|*.webm) return 0 ;;
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
  case "$(basename "$f")" in
    .*) return 0 ;;
    *.tmp|*.temp|*.part|*.crdownload|*.download|*.bak|*.swp|*~) return 0 ;;
  esac
  return 1
}

wait_stable() {
  local f="$1"
  local tries=0
  local prev=-1 cur=0
  sleep 3
  while [ $tries -lt 20 ]; do
    [ -f "$f" ] || return 1
    cur=$(stat -c%s "$f" 2>/dev/null || echo -1)
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
  if "$RISE_BIN" video "$f" --device auto --key "$KEY_FILE" --out "$out" >> "$LOG_FILE" 2>&1; then
    log "SEAL-VIDEO ok $f -> $out"
  else
    log "ERROR seal-video failed for $f"
    rm -f "$out"
  fi
}

seal_image() {
  local f="$1"
  local manifest="${f}.manifest.json"
  local out="${f}.rise"
  log "SEAL-IMAGE start $f"
  if ! "$RISE_BIN" sign "$f" --device auto --key "$KEY_FILE" --out "$manifest" >> "$LOG_FILE" 2>&1; then
    log "ERROR sign failed for $f"
    rm -f "$manifest"
    return
  fi
  if "$RISE_BIN" pack "$f" "$manifest" --out "$out" >> "$LOG_FILE" 2>&1; then
    log "SEAL-IMAGE ok $f -> $out"
  else
    log "ERROR pack failed for $f, keeping manifest as .rise fallback"
    cp -f "$manifest" "$out"
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
    \( -iname '*.mp4' -o -iname '*.mkv' -o -iname '*.mov' -o -iname '*.webm' \
     -o -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' \) \
    -newer "$MARKER_FILE" -print0 2>/dev/null)
  touch "$MARKER_FILE"
}

main() {
  ensure_env
  log "START linux daemon pid=$$ bin=$RISE_BIN watching: ${WATCH_DIRS[*]}"
  while true; do
    poll_once
    sleep "$POLL_INTERVAL"
  done
}

main "$@"
