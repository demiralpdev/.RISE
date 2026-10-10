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
| Android | CORE REAL - core binary built for aarch64-linux-android and RUN on the attached device (`emugy5nbrsmrmzmz`): on-device SHA-256 matches Mac byte-for-byte; device keygen+sign pulled to Mac and verified `VALID (signed)` (2026-10-10). JNI bridge REAL: librise_jni.so NDK-compiled (CMake, static librise_core.a), `Java_com_rise_core_RiseCore_hashFrame` exported, dlopen-loaded on device (JNI_SYMBOL_OK). Kotlin capture/attest files COMPILE against android-35 (kotlinc 2.0.21 + JDK 21, 20 classes, 0 errors, 2026-10-10); device run still pending an app  | STUB - `attest/RiseKeystore.kt`, `RiseIntegrity.kt`, `RiseAssurance.kt` exist; never run | REAL for core CLI on device (hash + cross-device signature); NOT RUN for Kotlin capture | real run (adb device, release binary) | capture/attest tiers still design-only: STRONG missing -> L2/silver; BASIC+clean = L2 max; mock/GPS-off = silver; broken = red |
| Windows | CORE+CAPTURE REAL - `cargo test` 33/33 green on DESKTOP-1LD8TK1 (192.168.111.7, SSH user `rise`); `capture.cpp` compiled+run on the PC (2026-10-10): MediaFoundation SourceReader, HP True Vision HD Camera (internal, usb=0), YUY2 1280x720, 1843200 bytes written + hashed by the PC rise-core (`3f651575...`) | REAL - `attest.cpp` compiled+run on the PC (2026-10-10): TPM present, `RISE-device-key` P-256 created+finalized in TPM via NCrypt, VBS off in registry -> fail-closed silver | REAL e2e - `keygen -> sign --key --tsa (live FreeTSA) -> verify --pubkey` printed `VALID (signed)` exit 0 (2026-10-10) | real run (tests + attest + capture + e2e over SSH) | gold-L2 requires VBS enabled on that PC (registry off today -> silver); L4 unreachable by design, PC-signed frame+manifest (live FreeTSA) pulled to Mac: hash matched byte-for-byte and verified `VALID (signed)` on the Mac (2026-10-10) |

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

**Android - CORE REAL on device, Kotlin STUB.**
The Rust core binary (aarch64-linux-android, NDK r27) ran on the real phone via
adb: on-device SHA-256 matched the Mac byte-for-byte, and a keypair generated +
signed ON THE DEVICE verified as `VALID (signed)` on the Mac (cross-device proof).
The JNI bridge (librise_jni.so) was NDK-compiled via CMake, symbol-verified with
llvm-nm, and dlopen-loaded on the device (JNI_SYMBOL_OK); calling it needs an app
process (future work). The Kotlin capture/attest files now compile against
android-35 (kotlinc 2.0.21 + JDK 21, 20 classes, 0 errors) - Play Integrity
gating and the capture flow still need a real app run.

**Windows — CORE+ATTEST REAL, capture STUB.**
Rust core fully green on the real PC (33/33 tests, MSVC Build Tools installed,
stable-msvc toolchain) and the signed e2e ran live: stamped with FreeTSA, ES256
keypair in temp, `VALID (signed)` exit 0. `attest.cpp` compiled+ran on the PC:
TPM present, `RISE-device-key` P-256 created+finalized inside the TPM via NCrypt,
VBS off in registry -> fail-closed silver (gold-L2 opens when VBS is enabled).
`capture.cpp` compiled+ran on the PC: MF SourceReader grabbed a real YUY2
frame (1280x720, 1843200 bytes) from the HP True Vision camera; the frame was
signed on the PC (live FreeTSA stamp) and verified on the Mac as `VALID (signed)`
with a byte-for-byte hash match - a full cross-platform evidence chain.
E_POINTER gotcha documented in the source (ReadSample out-params must be non-NULL).
