# Android Binding (thin shell)

> Compile-verified AND run on a device (capture path only). The whole module
> (app + `capture/` + `attest/` + `com/rise/core/RiseCore.kt`) compiles against
> android-35 with kotlinc 2.3.20 (Android Studio's bundled compiler) + JRE 26.0.1,
> `rc 0`, 31 classes (2026-10-17). The app is framework-only, so a bare
> `android.jar` classpath is enough — no AndroidX. That run caught and fixed an
> `isNullOrEmpty()` misuse on `FloatArray?` in the Rank 1 source gate, which would
> otherwise have failed to compile. The guard-inclusive build then RAN on the
> attached M2103K19C: the button-driven capture path produced a photo seal and a
> 14-frame video seal, both `capture_source:"camera"`, cross-verified on the Mac
> (see `app/README.md`). The Kotlin glue was afterwards refactored for code
> health (pure extraction — the Rank 1 source gate split into small helpers, no
> behavior change); this revision recompiles `rc 0`, 31 classes, but the
> device-run binary predates that refactor. `attest/` (Play Integrity, StrongBox) is still never-run;
> the reject path is unexercised because this phone exposes no virtual/external
> camera.

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: StrongBox + Play Integrity
- Task: camera frame -> core hash/attest call
- Core FFI contract: `rise_hash_frame(bytes, len, out65)` via future JNI
  (TODO, not implemented here). `RiseCapture.RawFrame.yuvBytes` is the input.

## Files

- `capture/RiseCapture.kt` (A-101): Camera2 single-frame, raw YUV_420_888
  path, back-camera preferred, one-shot session then close. Rank 1 source gate:
  a camera is only selected when it passes `sourceRejection()` — an external
  lens facing, a legacy/external hardware level, an absent physical sensor size
  or no finite focal length fails closed with
  `CaptureError.VirtualCameraRejected` (`ERR_VIRTUAL_CAMERA_REJECTED`).
- `capture/RiseCameraPermission.kt` (A-101): CAMERA check -> rationale ->
  system request -> app-settings deep-link
  (`ACTION_APPLICATION_DETAILS_SETTINGS`).
- `attest/RiseKeystore.kt` (A-102): AndroidKeyStore P-256 (secp256r1),
  StrongBox-first with TEE fallback, attestation challenge. Key material
  is never read, exported, or logged.
- `attest/RiseIntegrity.kt` (A-102): Play Integrity verdict parsing
  (STRONG/DEVICE/BASIC/NONE) + STRONG gating.
- `attest/RiseAssurance.kt` (A-102/A-103): explicit downgrade table,
  mock/GPS-off auto-downgrade per `spec/solutions-platform.md` §5/§9,
  old-phone cap per §10.

## Downgrade table (binding ceiling; verify-web may go lower, never higher)

| Play Integrity | Key home   | Environment             | Ceiling         |
|---|---|---|---|
| STRONG         | STRONG_BOX | clean                   | gold-L4 candidate |
| STRONG         | TEE        | clean                   | gold-L2 max     |
| STRONG missing | any        | clean                   | silver max      |
| BASIC + clean  | any        | clean                   | gold-L2 max     |
| any            | any        | mock ON / GPS off       | silver max      |
| any            | any        | broken chain / replay   | red             |
| NONE           | SOFTWARE   | any                     | red             |

## Gold-gate checklist

- No gold without a StrongBox key + Play Integrity STRONG verdict.
- If STRONG is missing, downgrade from gold-L4 to gold-L2 or silver.
- BASIC verdict + clean chain = gold-L2 at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.
- Mock location ON or location OFF = silver at most, never gold.
- No mock tests presented as real: no instrumented tests exist yet.
