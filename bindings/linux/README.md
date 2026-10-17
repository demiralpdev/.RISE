# Linux Binding (thin shell)

> STUB: untested on real Linux hardware (author machine is macOS). Scripts are
> static-checked (`bash -n`, plus shellcheck where available); the camera-source
> guard additionally has offline unit tests (`capture/test_cam_class.sh`).

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: TPM2 optional
- Task: camera frame -> core hash/attest call
- Capture: see `capture/capture.sh` (V4L2 via `ffmpeg -f v4l2`, built-in camera
  priority, virtual-camera refusal, manifest step calling the core CLI).
- Attestation: see `attest/` (`detect.sh` TPM2 presence check, report-only).

## Files

- `capture/capture.sh` — device selection, frame capture, manifest step.
- `capture/cam_class.sh` — `rise_classify_video_node`: labels a `/dev/videoN`
  node `virtual` / `external_usb` / `builtin` from sysfs.
- `capture/test_cam_class.sh` — offline tests over synthetic sysfs trees.

## Camera-source policy (spec/scene-authenticity.md §2 Rank 1)

| Detected source | Verdict | `capture_source` |
|---|---|---|
| No physical bus device (`devices/virtual/`, dangling `device` link, unreadable class entry) | refused, exit 10 `ERR_VIRTUAL_CAMERA_REJECTED` | — |
| Built-in, chassis-fixed (incl. soldered UVC) | accepted, preferred | `camera` |
| External USB (`removable=removable`) | accepted only when no built-in camera exists | `external_usb` |

## Gold-gate checklist

- Linux ceiling is silver — never gold. The gold path is closed on Linux.
- No TPM2 key: no gold, target silver at most.
- TPM2 present + clean chain: still silver at most, never gold.
- No hardware attestation: gold-L4/L2 closed, downgrade to silver.
- Software key + clean chain = silver at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.
