# Android Binding (thin shell)

> STUB — NOT compiled/tested. No Android SDK on this machine, so none of the
> Kotlin below has been typechecked or run. Code compiles by inspection only.
> Status: draft shell for review, not a working binding.

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: StrongBox + Play Integrity
- Task: camera frame -> core hash/attest call
- Core FFI contract: `rise_hash_frame(bytes, len, out65)` via future JNI
  (TODO, not implemented here). `RiseCapture.RawFrame.yuvBytes` is the input.

## Files

- `capture/RiseCapture.kt` (A-101): Camera2 single-frame, raw YUV_420_888
  path, back-camera preferred, one-shot session then close.
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
