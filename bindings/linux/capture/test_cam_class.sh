#!/bin/bash
# test_cam_class.sh — offline tests for rise_classify_video_node (Rank 1 guard).
#
# Builds synthetic sysfs trees in a temp dir, so the virtual/loopback refusal
# and the built-in-over-USB priority are proven without a camera or root.
set -u

HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/cam_class.sh"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
SYS="$TMP/sys"
FAILS=0

mkdir -p "$SYS/class/video4linux" "$SYS/devices"

check() {
  local name="$1" expected="$2" got
  got="$(rise_classify_video_node "/dev/$name" "$SYS")"
  if [ "$got" = "$expected" ]; then
    echo "ok   $name -> $got"
  else
    echo "FAIL $name -> $got (expected $expected)"
    FAILS=$((FAILS + 1))
  fi
}

# --- v4l2loopback: class entry resolves under devices/virtual, no bus device.
mkdir -p "$SYS/devices/virtual/video4linux/video10"
ln -s ../../devices/virtual/video4linux/video10 "$SYS/class/video4linux/video10"
check video10 virtual

# --- Node with no class entry at all (unreadable sysfs) must fail closed.
check video99 virtual

# --- External UVC webcam: idVendor up the bus chain, removable=removable.
USB="$SYS/devices/pci0000:00/usb1/1-1"
IFACE="$USB/1-1:1.0"
mkdir -p "$IFACE/video4linux/video0"
printf '0x046d\n' > "$USB/idVendor"
printf 'removable\n' > "$USB/removable"
ln -s ../../.. "$IFACE/video4linux/video0/device"
ln -s ../../devices/pci0000:00/usb1/1-1/1-1:1.0/video4linux/video0 "$SYS/class/video4linux/video0"
check video0 external_usb

# --- Built-in UVC (soldered webcam): same chain but removable=fixed.
mkdir -p "$IFACE/video4linux/video4"
printf 'fixed\n' > "$USB/removable"
ln -s ../../.. "$IFACE/video4linux/video4/device"
ln -s ../../devices/pci0000:00/usb1/1-1/1-1:1.0/video4linux/video4 "$SYS/class/video4linux/video4"
check video4 builtin

# --- MIPI/platform sensor: physical bus with no idVendor anywhere above it.
mkdir -p "$SYS/devices/platform/soc/cam0/video4linux/video2"
ln -s ../../.. "$SYS/devices/platform/soc/cam0/video4linux/video2/device"
ln -s ../../devices/platform/soc/cam0/video4linux/video2 "$SYS/class/video4linux/video2"
check video2 builtin

# --- Node present but its device link dangles: indistinguishable from virtual.
mkdir -p "$SYS/devices/platform/soc/cam1/video4linux/video6"
ln -s ../../nowhere "$SYS/devices/platform/soc/cam1/video4linux/video6/device"
ln -s ../../devices/platform/soc/cam1/video4linux/video6 "$SYS/class/video4linux/video6"
check video6 virtual

if [ "$FAILS" -ne 0 ]; then
  echo "cam_class: $FAILS FAILED"
  exit 1
fi
echo "cam_class: all checks passed"
