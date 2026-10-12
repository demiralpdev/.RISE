#!/bin/bash
# RISE auto-seal daemon installer (Linux) — user-level, no root needed.
set -eu

DIR="$(cd "$(dirname "$0")" && pwd)"

mkdir -p "$HOME/.rise" "$HOME/.config/systemd/user" "$HOME/.local/bin"

# CLI binary: download from the latest GitHub Release when not present.
if [ ! -x "$HOME/.local/bin/rise-core" ]; then
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64) BIN="rise-core-x86_64-unknown-linux-musl" ;;
        aarch64) BIN="rise-core-aarch64-unknown-linux-musl" ;;
        *) echo "unsupported arch: $ARCH"; exit 1 ;;
    esac
    curl -fsSL -o "$HOME/.local/bin/rise-core" \
        "https://github.com/demiralpdev/.RISE/releases/latest/download/$BIN"
    chmod +x "$HOME/.local/bin/rise-core"
fi

# Daemon script + systemd user service.
cp "$DIR/auto-seal.sh" "$HOME/.rise/auto-seal.sh"
chmod +x "$HOME/.rise/auto-seal.sh"
cp "$DIR/rise-autoseal.service" "$HOME/.config/systemd/user/rise-autoseal.service"

systemctl --user daemon-reload
systemctl --user enable --now rise-autoseal.service

echo "RISE auto-seal daemon installed (user service)."
echo "Logs: ~/.rise/daemon.log"
