# Windows Binding (thin shell)

> HONESTY HEADER: NOT compiled, NOT tested — STUB. No Windows toolchain on
> this machine; static review only. Files: `capture/capture.cpp` (W-101),
> `attest/attest.cpp` (W-102).

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: TPM 2.0 + VBS
- Task: camera frame -> core hash/attest call
- Capture: `capture/capture.cpp` — MediaFoundation `IMFSourceReader` single
  frame, raw YUY2 path (1280x720 preferred, native-type fallback recorded in
  `<output>.meta`); USB FriendlyName priority (external webcam); MJPEG/H264
  never decoded before hashing.
- Attest: `attest/attest.cpp` — platform key via NCrypt P-256 (key never
  leaves TPM/Pluton) + VBS status check; prints `TIER=<...>` + reason.

## Gold-gate checklist (wired to `attest/attest.cpp::DecideTier`)

- No gold without a TPM 2.0 key + VBS confirmation.
- No TPM/VBS -> silver-only.
- Weak attestation (no quote / key outside TPM) + clean chain = gold-L2 at
  most; silver for USB sources.
- Broken chain / replay suspicion = red, the binding does not argue.
- Gold-L4 unreachable from this binding (no sensor-output hash, no protected
  media path); callers get L2 at most.
- On suspicion, downgrade the badge, never upgrade.
