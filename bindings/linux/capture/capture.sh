#!/bin/bash
# Capture one frame via V4L2: prefer a USB camera if present, else the first /dev/videoN.
# Output: ../../target/frame-001.jpg (i.e. bindings/target/frame-001.jpg)
set -e
cd "$(dirname "$0")"

OUT="${1:-../../target/frame-001.jpg}"
mkdir -p "$(dirname "$OUT")"

# Device priority: L-101 — the device node is parsed by name from the device
# list (no hardcode). USB cameras win over integrated ones.
DEV=""
DEV_NAME=""
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
  # Prefer a block whose label mentions USB (covers "Rapoo Camera: USB Camera").
  USB_BLOCK_DEV=$(echo "$LISTING" | grep -i -B1 -A3 'usb' | grep -o '/dev/video[0-9]*' | head -1 || true)
  if [ -n "$USB_BLOCK_DEV" ]; then
    DEV="$USB_BLOCK_DEV"
    DEV_NAME="usb-camera"
  else
    # No USB label: fall back to the first listed node by name order.
    DEV=$(echo "$LISTING" | grep -o '/dev/video[0-9]*' | head -1 || true)
    DEV_NAME="integrated-v4l2"
  fi
else
  # No v4l2-ctl: enumerate nodes directly, prefer the lowest even index
  # (odd indexes are usually V4L2 metadata nodes, e.g. /dev/video1).
  for node in /dev/video0 /dev/video2 /dev/video4 /dev/video6 /dev/video8 /dev/video1 /dev/video3; do
    if [ -e "$node" ]; then
      DEV="$node"
      break
    fi
  done
  DEV_NAME="fallback-v4l2"
fi
if [ -z "$DEV" ]; then
  echo "ERROR: no suitable camera found (no /dev/video* node, no v4l2-ctl listing)."
  echo "Permission hint: check that the camera is plugged in and readable:"
  echo "  ls -l /dev/video* ; sudo usermod -aG video \$USER (log out/in), then run again."
  exit 1
fi
echo "Camera device: $DEV ($DEV_NAME)"

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
  "$CORE_BIN" sign "$OUT" --device "$DEV_NAME" --out "${OUT%.jpg}.manifest.json" \
    || echo "WARN: manifest could not be generated (core error)"
else
  echo "Note: core CLI is not built (core/target/debug/rise-core), manifest skipped."
fi
