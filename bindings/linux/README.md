# Linux Binding (thin shell)

> STUB: untested on real Linux hardware (author machine is macOS). Scripts are
> static-checked only (`bash -n`, plus shellcheck where available).

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: TPM2 optional
- Task: camera frame -> core hash/attest call
- Capture: see `capture/capture.sh` (V4L2 via `ffmpeg -f v4l2`, USB priority
  by name parse, manifest step calling the core CLI).
- Attestation: see `attest/` (`detect.sh` TPM2 presence check, report-only).

## Gold-gate checklist

- Linux ceiling is silver — never gold. The gold path is closed on Linux.
- No TPM2 key: no gold, target silver at most.
- TPM2 present + clean chain: still silver at most, never gold.
- No hardware attestation: gold-L4/L2 closed, downgrade to silver.
- Software key + clean chain = silver at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.
