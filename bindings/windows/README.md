# Windows Binding (thin shell)

> HONESTY HEADER: the C++ sources were compiled and run once on
> DESKTOP-1LD8TK1 (2026-10-10, see `bindings/MATRIX.md`). There is no Windows
> toolchain on this machine, so the 2026-10-17 Rank 1 source-gate edit below is
> static-only: it has NOT been compiled or run. Files: `capture/capture.cpp`
> (W-101), `attest/attest.cpp` (W-102).
>
> The `.ps1` files, by contrast, ARE syntax-verified here: PowerShell 7.6.7
> (official `osx-arm64` tarball, sha256 checked against the release's own
> `hashes.sha256`) parses `daemon/windows/auto-seal.ps1` and
> `system-format/windows/install.ps1` with 0 errors. Syntax only — no line of
> them has been executed.

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: TPM 2.0 + VBS
- Task: camera frame -> core hash/attest call
- Capture: `capture/capture.cpp` — MediaFoundation `IMFSourceReader` single
  frame, raw YUY2 path (1280x720 preferred, native-type fallback recorded in
  `<output>.meta`); built-in camera priority, external USB second, a virtual
  source (OBS/NDI/DroidCam or a ROOT/SWD-enumerated device) is rejected with
  `ERR_VIRTUAL_CAMERA_REJECTED`; the sidecar records
  `capture_source=camera|external_usb`; MJPEG/H264 never decoded before hashing.
- Attest: `attest/attest.cpp` — platform key via NCrypt P-256 (key never
  leaves TPM/Pluton) + VBS enablement check (running state is NOT verified);
  prints `TIER=<...>` + reason.

## Gold-gate checklist (wired to `attest/attest.cpp::DecideTier`)

- No gold without a TPM 2.0 key + VBS enablement in the registry.
- No TPM/VBS -> silver-only.
- Weak attestation (no quote / key outside TPM) + clean chain = gold-L2 at
  most; silver for USB sources.
- Broken chain / replay suspicion = red, the binding does not argue.
- Gold-L4 unreachable from this binding (no sensor-output hash, no protected
  media path); callers get L2 at most.
- On suspicion, downgrade the badge, never upgrade.
