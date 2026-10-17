#!/bin/bash
# Capture one frame via V4L2: prefer a USB camera if present, else the first /dev/videoN.
# Output: ../../target/frame-001.jpg (i.e. bindings/target/frame-001.jpg)
set -e
cd "$(dirname "$0")"

OUT="${1:-../../target/frame-001.jpg}"
mkdir -p "$(dirname "$OUT")"

# Device priority: L-101 — the device node is parsed by name from the device
# list (no hardcode). Rank 1: a node with no physical bus device
# (v4l2loopback/akvcam) is refused, and a built-in camera wins over external USB.
. ./cam_class.sh

readonly EXIT_NO_CAMERA=1
readonly EXIT_VIRTUAL_CAMERA=10

SYSFS_ROOT="${RISE_SYSFS_ROOT:-/sys}"
CANDIDATES=""
LISTING=""
if command -v v4l2-ctl >/dev/null 2>&1; then
  LISTING=$(v4l2-ctl --list-devices 2>/dev/null || true)
fi
if [ -n "$LISTING" ]; then
  echo "$LISTING"
  # A v4l2-ctl listing looks like:
  #   Rapoo Camera: USB Camera (usb-...):
  #           /dev/video0
  #           /dev/video1
  CANDIDATES=$(echo "$LISTING" | grep -o '/dev/video[0-9]*' || true)
else
  # No v4l2-ctl: enumerate nodes directly, even indexes first
  # (odd indexes are usually V4L2 metadata nodes, e.g. /dev/video1).
  for node in /dev/video0 /dev/video2 /dev/video4 /dev/video6 /dev/video8 /dev/video1 /dev/video3; do
    if [ -e "$node" ]; then
      CANDIDATES="$CANDIDATES $node"
    fi
  done
fi

DEV=""
DEV_NAME=""
CAPTURE_SOURCE="camera"
EXTERNAL_DEV=""
REJECTED=""
for node in $CANDIDATES; do
  case "$(rise_classify_video_node "$node" "$SYSFS_ROOT")" in
    virtual)
      REJECTED="$REJECTED $node"
      echo "REFUSED: $node has no physical bus device (software/virtual camera)."
      ;;
    builtin)
      DEV="$node"
      DEV_NAME="builtin-camera"
      break
      ;;
    external_usb)
      [ -n "$EXTERNAL_DEV" ] || EXTERNAL_DEV="$node"
      ;;
  esac
done

if [ -z "$DEV" ] && [ -n "$EXTERNAL_DEV" ]; then
  DEV="$EXTERNAL_DEV"
  DEV_NAME="usb-camera"
  CAPTURE_SOURCE="external_usb"
fi

if [ -z "$DEV" ]; then
  if [ -n "$REJECTED" ]; then
    echo "ERR_VIRTUAL_CAMERA_REJECTED:$REJECTED no physical bus device — RISE refuses to seal a software/virtual camera as camera evidence." >&2
    exit "$EXIT_VIRTUAL_CAMERA"
  fi
  echo "ERROR: no suitable camera found (no /dev/video* node, no v4l2-ctl listing)."
  echo "Permission hint: check that the camera is plugged in and readable:"
  echo "  ls -l /dev/video* ; sudo usermod -aG video \$USER (log out/in), then run again."
  exit "$EXIT_NO_CAMERA"
fi
echo "Camera device: $DEV ($DEV_NAME, capture_source=$CAPTURE_SOURCE)"

if ffmpeg -f v4l2 -framerate 30 -video_size 1280x720 -i "$DEV" \
    -frames:v 1 -q:v 2 -y "$OUT" 2>capture.log; then
  :
else
  echo "WARN: first attempt failed, retrying with default settings..."
  ffmpeg -f v4l2 -i "$DEV" -frames:v 1 -q:v 2 -y "$OUT" 2>capture.log || {
    echo "ERROR: could not capture a frame. See capture.log."
    echo "Permission hint: the device node must be readable (group 'video'):"
    echo "  ls -l $DEV ; sudo usermod -aG video \$USER (log out/in), then run again."
    exit 1
  }
fi

if [ -s "$OUT" ]; then
  ls -lh "$OUT"
else
  echo "ERROR: $OUT is empty. Permission may have been denied:"
  echo "  ls -l $DEV ; sudo usermod -aG video \$USER (log out/in), then run again."
  exit 1
fi

# Manifest: if the core CLI is built, convert the frame into a sealed manifest
CORE_BIN="../../../core/target/release/rise-core"
[ -x "$CORE_BIN" ] || CORE_BIN="../../../core/target/debug/rise-core"
if [ -x "$CORE_BIN" ]; then
  "$CORE_BIN" sign "$OUT" --device "$DEV_NAME" --capture-source "$CAPTURE_SOURCE" \
    --out "${OUT%.jpg}.manifest.json" \
    || echo "WARN: manifest could not be generated (core error)"
else
  echo "Note: core CLI is not built (core/target/debug/rise-core), manifest skipped."
fi
if [ -x "$CORE_BIN" ] && [ -f "${OUT%.jpg}.manifest.json" ]; then
  "$CORE_BIN" pack "$OUT" "${OUT%.jpg}.manifest.json" --out "${OUT%.jpg}.rise" \
    || echo "WARN: pack failed (core error)"
fi
