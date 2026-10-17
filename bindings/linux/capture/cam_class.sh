#!/bin/bash
# cam_class.sh — V4L2 camera-source classification (Rank 1 device guard).
#
# Pure functions: no capture, no signing, no side effects. Sourced by
# capture.sh and exercised directly by test_cam_class.sh against a synthetic
# sysfs tree, so the guard is verifiable without a camera or root.

# Physical path of a symlinked directory entry. `set -P` is required: bash's
# default logical cd would resolve `..` against the path as typed, so a target
# reached through a symlinked parent would walk the wrong tree. macOS readlink
# has no -f, hence the cd/pwd pair.
rise_resolve_dir() {
  local entry="$1" target
  target="$(readlink "$entry" 2>/dev/null)" || return 1
  [ -n "$target" ] || return 1
  case "$target" in
    /*) ( set -P; cd "$target" 2>/dev/null && pwd -P ) || return 1 ;;
    *) ( set -P; cd "$(dirname "$entry")" 2>/dev/null \
           && cd "$target" 2>/dev/null && pwd -P ) || return 1 ;;
  esac
}

# Classifies a /dev/videoN node for camera-evidence purposes.
# Prints exactly one of: virtual | external_usb | builtin
#
# "virtual" is returned for any node that is not provably backed by physical
# bus hardware — v4l2loopback/akvcam register under devices/virtual with no
# bus device, and an unresolvable node is indistinguishable from one, so an
# unreadable class entry fails closed to virtual rather than to builtin.
#
# $1 = device node (e.g. /dev/video0), $2 = sysfs root (default /sys)
rise_classify_video_node() {
  local node="$1" sysfs="${2:-/sys}" name class_entry dev_entry dev_dir bus_dir
  name="$(basename "$node")"
  class_entry="$sysfs/class/video4linux/$name"

  case "$(rise_resolve_dir "$class_entry" 2>/dev/null || true)" in
    *devices/virtual/*) echo "virtual"; return 0 ;;
  esac

  dev_entry="$class_entry/device"
  [ -e "$dev_entry" ] || { echo "virtual"; return 0; }
  dev_dir="$(rise_resolve_dir "$dev_entry" 2>/dev/null || true)"
  case "$dev_dir" in
    "") echo "virtual"; return 0 ;;
    *devices/virtual/*) echo "virtual"; return 0 ;;
  esac

  # A USB-attached sensor has an idVendor on its bus device. removable=removable
  # means the camera can be unplugged (external source); fixed means it is
  # soldered into the chassis (built-in UVC, e.g. a laptop webcam).
  bus_dir="$dev_dir"
  while [ -n "$bus_dir" ] && [ "$bus_dir" != "/" ] && [ "$bus_dir" != "." ]; do
    if [ -e "$bus_dir/idVendor" ]; then
      if [ "$(cat "$bus_dir/removable" 2>/dev/null)" = "removable" ]; then
        echo "external_usb"
      else
        echo "builtin"
      fi
      return 0
    fi
    bus_dir="$(dirname "$bus_dir")"
  done
  echo "builtin"
}
