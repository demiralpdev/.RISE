# Bindings Matrix — 5-OS Status (T-401)

> Status document, not code. Every claim below is backed by an on-disk check
> (`ls bindings/*/capture bindings/*/attest`, `ls bindings/target/`).
> Anything not run on real hardware is marked **STUB** explicitly.
> Date: 2026-10-10. Machine: macOS (author machine).

## Status table

| OS | Capture state | Attest state | e2e state | Proof type | Tier ceiling |
|---|---|---|---|---|---|
| macOS | REAL — `capture/capture.sh`, `capture/capture.swift`, `capture/capture.log` exist; `bindings/target/frame-001.jpg` on disk | STUB — no `attest/` dir, no Secure Enclave code; goal only in `README.md` | REAL e2e (capture → manifest) — `bindings/target/frame-001.jpg` + `frame-001.manifest.json` exist | real run (artifacts on disk) | silver max currently (no SE key → no gold per gold-gate); gold-L2/L4 only after attest lands |
| iOS | STUB — `capture/Capture.swift` exists, never ran on device | STUB — `attest/Attest.swift` exists, never ran (needs iPhone + Apple servers) | NOT RUN — no frame, no manifest | static (reported `swiftc --typecheck` PASS 2026-10-10 per README; not re-verified here) | gold-L2/L4 candidate by design (SE + App Attest), unproven |
| Linux | STUB — `capture/capture.sh` exists, never ran on Linux | STUB — `attest/detect.sh` + `attest/README.md` exist, report-only | NOT RUN — no frame, no manifest | static (reported `bash -n` clean per README; not re-run here — `ls` only) | silver, always (gold path closed on Linux) |
| Android | STUB, uncompiled — `capture/RiseCapture.kt`, `capture/RiseCameraPermission.kt` exist; no Android SDK on this machine | STUB, uncompiled — `attest/RiseKeystore.kt`, `attest/RiseIntegrity.kt`, `attest/RiseAssurance.kt` exist; never run | NOT RUN — no frame, no manifest | untested (README: "compiles by inspection only") | gold-L4 candidate by design (STRONG + STRONG_BOX + clean); BASIC+clean = gold-L2 max; mock/GPS-off = silver max; broken chain = red |
| Windows | CORE REAL — `cargo test` 33/33 green on DESKTOP-1LD8TK1 (192.168.111.7, SSH user `rise`); `capture/capture.cpp` still uncompiled | STUB — `attest/attest.cpp` exists, never run (needs TPM/VBS probe on that PC) | REAL e2e — `keygen → sign --key --tsa (live FreeTSA) → verify --pubkey` printed `VALID (signed)` exit 0 (2026-10-10) | real run (tests + e2e over SSH) | gold-L2 max by design (L4 unreachable from this binding); no TPM/VBS = silver-only; broken chain = red |

Proof-type legend: **real run** = artifacts on disk from an actual run;
**static** = source exists + a static check is reported (typecheck / `bash -n`)
but nothing executed on target hardware; **untested** = source exists but no
check of any kind has been run.

## Evidence (what `ls` actually showed)

- `bindings/target/`: `frame-001.jpg`, `frame-001.manifest.json`,
  `frame-001.rise.jpg`, `kurcalanmis.jpg` — present.
- `bindings/macos/capture/`: `README.md`, `capture.log`, `capture.sh`,
  `capture.swift`, `core.h` — present. `bindings/macos/attest/` — absent.
- `bindings/ios/capture/`: `Capture.swift` — present.
  `bindings/ios/attest/`: `Attest.swift` — present.
- `bindings/linux/capture/`: `capture.sh` — present.
  `bindings/linux/attest/`: `README.md`, `detect.sh` — present.
- `bindings/android/capture/`: `RiseCameraPermission.kt`, `RiseCapture.kt` —
  present. `bindings/android/attest/`: `RiseAssurance.kt`,
  `RiseIntegrity.kt`, `RiseKeystore.kt` — present.
- `bindings/windows/capture/`: `capture.cpp` — present.
  `bindings/windows/attest/`: `attest.cpp` — present.

## Per-OS notes (3 lines each)

**macOS — REAL capture, STUB attest.**
Capture is the only binding with on-disk run output (`frame-001.jpg` +
manifest in `bindings/target/`). There is no `attest/` implementation, so the
gold gate ("no gold without a Secure Enclave key") caps it at silver today.

**iOS — STUB, static only.**
Both Swift files exist and the README reports `swiftc --typecheck` PASS on
macOS (2026-10-10, iOS-only camera types deliberately unlisted). No device run,
no App Attest receipt validation, no frame verified with core — e2e is NOT RUN.

**Linux — STUB, static only.**
`capture.sh` (V4L2/ffmpeg) and `attest/detect.sh` (TPM2 presence, report-only)
exist; README reports static checks only (`bash -n` / shellcheck) and no real
Linux hardware run. Ceiling is silver by policy even with TPM2 present.

**Android — STUB, uncompiled.**
All five Kotlin files exist but the README states NOT compiled/tested (no
Android SDK here, inspection only). No instrumented tests, no Play Integrity
verdict, no frame — e2e is NOT RUN; downgrade table is design, not behavior.

**Windows — CORE REAL, capture STUB.**
Rust core fully green on the real PC (33/33 tests, MSVC Build Tools installed,
stable-msvc toolchain) and the signed e2e ran live: stamped with FreeTSA, ES256
keypair in temp, `VALID (signed)` exit 0. The MediaFoundation `capture.cpp` and
TPM/VBS `attest.cpp` remain uncompiled/unrun — camera capture proof still pending.
