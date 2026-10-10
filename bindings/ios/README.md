# iOS Binding (thin shell)

> HONESTY HEADER — STUB, NOT device-tested.
> - `capture/Capture.swift` (I-101): `swiftc -typecheck` PASS on this macOS
>   machine (2026-10-10). iOS-only device types (dual/triple camera) are
>   deliberately not listed so the check passes; noted in code.
> - `attest/Attest.swift` (I-102): `swiftc -typecheck` PASS on this macOS
>   machine (2026-10-10) via `canImport(DeviceCheck)` guard. Real App Attest
>   receipt validation requires a physical iPhone + Apple servers — never run.
> - No e2e run, no frame byte verified with core yet. Do not treat as tested.

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: Secure Enclave + App Attest (`attest/Attest.swift`)
- Task: camera frame -> core hash/attest call (`capture/Capture.swift` -> `rise-core hash`)
- Host app MUST set `NSCameraUsageDescription` in Info.plist (see Capture.swift header)

## Files

- `capture/Capture.swift` — `RiseCapture.pickDevice()` (USB/UVC first, back
  wide-angle default), `ensurePermission()`, `captureSingleFrame(to:)`.
  Mirrors `bindings/macos/capture/capture.swift` (priority, permission wait,
  1s exposure settle, JPEG out, manifest via core CLI note).
- `attest/Attest.swift` — Secure Enclave P-256 key holder (`RiseKeystore`) +
  App Attest flow sketch (`RiseAttest`) + downgrade table. Keys and
  attestation objects only; no frame signing, no manifest, no verification.

## Gold-gate checklist

- No gold without a Secure Enclave key + valid App Attest.
- If attestation is weak, downgrade from gold-L4 to gold-L2 or silver.
- Weak attestation + clean chain = gold-L2 at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.

## Downgrade table (binding -> verifier contract)

| # | Attestation state | Binding reports | Max badge |
|---|---|---|---|
| 1 | Secure Enclave P-256 + valid App Attest | strong | gold-L2/L4* |
| 2 | Secure Enclave key, App Attest weak (simulator, iOS < 14, no receipt) | weak | L2/silver at most |
| 3 | No Secure Enclave (old device) | weak | silver |
| 4 | App Attest validation failed | broken | red |
| 5 | Replay suspected (nonce/challenge miss) | broken | red |
| 6 | Broken chain from core (any red signal) | broken | red |

\* Row 1: L4 additionally requires sensor-path + protected-pipeline proof
(spec/assurance-levels.md Gold-L4); this binding cannot assert L4 alone.
