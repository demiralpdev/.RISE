#!/bin/bash
# Capture one frame: use the Rapoo USB camera if present, otherwise FaceTime HD.
# Output: ../../target/frame-001.jpg (i.e. bindings/target/frame-001.jpg)
set -e
cd "$(dirname "$0")"

OUT="${1:-../../target/frame-001.jpg}"
mkdir -p "$(dirname "$OUT")"

# Device priority: M-1 — the index is parsed by name from the device list (no hardcode)
LISTE=$(ffmpeg -f avfoundation -list_devices true -i "" 2>&1 || true)
DEV=""
# Extract the Rapoo index by name: [2] Rapoo Camera
DEV=$(echo "$LISTE" | grep -o '\[[0-9]*\] .*Rapoo Camera' | grep -o '[0-9]*' | head -1 || true)
DEV_NAME="rapoo-usb"
if [ -z "$DEV" ]; then
  # No Rapoo: extract the FaceTime index by name
  DEV=$(echo "$LISTE" | grep -o '\[[0-9]*\] .*FaceTime HD' | grep -o '[0-9]*' | head -1 || true)
  DEV_NAME="facetime-hd"
fi
if [ -z "$DEV" ]; then
  echo "ERROR: no suitable camera found (Rapoo/FaceTime)."
  exit 1
fi
echo "Camera index: $DEV ($DEV_NAME)"

if ffmpeg -f avfoundation -framerate 30 -video_size 1280x720 -i "$DEV" \
    -frames:v 1 -q:v 2 -y "$OUT" 2>capture.log; then
  :
else
  echo "WARN: first attempt failed, retrying with default settings..."
  ffmpeg -f avfoundation -i "$DEV" -frames:v 1 -q:v 2 -y "$OUT" 2>capture.log || {
    echo "ERROR: could not capture a frame. See capture.log."
    echo "Permission hint: System Settings > Privacy & Security > Camera > allow Terminal."
    exit 1
  }
fi

if [ -s "$OUT" ]; then
  ls -lh "$OUT"
else
  echo "ERROR: $OUT is empty. Permission may have been denied:"
  echo "System Settings > Privacy & Security > Camera > allow Terminal, then run again."
  exit 1
fi

# Manifest: if the core CLI is built, convert the frame into a sealed manifest
CORE_BIN="../../../core/target/debug/rise-core"
if [ -x "$CORE_BIN" ]; then
  "$CORE_BIN" sign "$OUT" --device "$DEV_NAME" --out "${OUT%.jpg}.manifest.json" \
    || echo "WARN: manifest could not be generated (core error)"
else
  echo "Note: core CLI is not built (core/target/debug/rise-core), manifest skipped."
fi
