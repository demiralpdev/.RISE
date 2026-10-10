#!/bin/bash
# .rise one-shot bring-up for Nobara Linux (Fedora-based).
# Run as your normal user; sudo is used for dnf installs (you type the password).
# At the end it prints a PASTE-THIS-BACK block: send that output back.
set -e
echo "=== [1/7] system packages (sudo) ==="
sudo dnf install -y git v4l-utils tpm2-tools
# Nobara ships ffmpeg-free which CONFLICTS with the full ffmpeg package; the
# ffmpeg CLI comes with ffmpeg-free, so only install if the binary is missing.
if ! command -v ffmpeg >/dev/null 2>&1; then
  sudo dnf install -y ffmpeg --allowerasing || sudo dnf install -y ffmpeg --skip-broken || echo "WARN: ffmpeg missing - install it manually"
fi

echo "=== [2/7] rust toolchain ==="
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  . "$HOME/.cargo/env"
fi
cargo --version

echo "=== [3/7] repo ==="
if [ ! -d "$HOME/RISE" ]; then
  git clone https://github.com/demiralpdev/.RISE.git "$HOME/RISE"
fi

echo "=== [4/7] core tests (expect 33 passed) ==="
cd "$HOME/RISE/core"
cargo test 2>&1 | grep "test result" | head -1
cargo build --release
CORE="$PWD/target/release/rise-core"

echo "=== [5/7] camera check ==="
v4l2-ctl --list-devices 2>&1 | head -8 || echo "no v4l2 devices found"

echo "=== [6/7] capture + sign + stamp + verify ==="
cd "$HOME/RISE/bindings/linux/capture"
./capture.sh || echo "CAPTURE_FAILED: see capture.log"
FRAME=$(ls -t "$HOME/RISE/bindings/target/frame-001.jpg" "$HOME/RISE/target/frame-001.jpg" 2>/dev/null | head -1)
if [ -z "$FRAME" ]; then
  echo "NO_FRAME: capture failed - paste capture.log content back"
  exit 1
fi
echo "FRAME=$FRAME"
mkdir -p "$HOME/RISE/target"
"$CORE" keygen --out "$HOME/RISE/target/nb-key" --alg es256
"$CORE" sign "$FRAME" --device nobara-laptop --key "$HOME/RISE/target/nb-key.key" \
  --tsa https://freetsa.org/tsr --out "$HOME/RISE/target/nb-m.json"
"$CORE" verify "$HOME/RISE/target/nb-m.json" "$FRAME" --pubkey "$HOME/RISE/target/nb-key.pub"

echo "=== [7/7] TPM ==="
cd "$HOME/RISE/bindings/linux/attest"
./detect.sh 2>&1 | head -6 || true

echo ""
echo "================ PASTE THIS BACK ================"
cd "$HOME/RISE/core"
cargo test 2>&1 | grep "test result" | head -1
ls -la "$FRAME" 2>/dev/null
"$CORE" verify "$HOME/RISE/target/nb-m.json" "$FRAME" --pubkey "$HOME/RISE/target/nb-key.pub"
echo "verify_exit=$?"
./detect.sh 2>&1 | head -6 || true
echo "================================================="
