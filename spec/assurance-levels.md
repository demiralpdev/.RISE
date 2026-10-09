# Assurance Levels

> Three tiers: silver, gold-L2, gold-L4. Every higher tier covers the ones below.

## Silver (Software)

- The key lives in the device's normal storage.
- Signature: ES256 default, Ed25519 option.
- Same formula: `SHA256(raw frame) at shutter + sign + timestamp`.
- RFC 3161 timestamp mandatory.
- Use: everyday capture, low risk.

## Gold-L2 (Protected Hardware)

- The key is inside TEE / Secure Element, it never leaves.
- The signature is applied inside the hardware.
- C2PA Trust List + revocation checks mandatory.
- Use: press, insurance, corporate evidence.

## Gold-L4 (Sensor + Protected Path)

- The hash is taken at sensor output, before entering the OS.
- A protected media path (secure pipeline) is used.
- C2PA 2.4 manifest embedded via JUMBF.
- Follows the ISO 21617-1:2026 box structure.
- Use: courts, critical infrastructure, high risk.

## Summary Table

| Property | Silver | Gold-L2 | Gold-L4 |
|---|---|---|---|
| Key location | Software | TEE/SE | TEE/SE |
| Hash point | App | App | Sensor output |
| Timestamp (RFC 3161) | Yes | Yes | Yes |
| Trust List + revocation | Yes | Yes | Yes |

## Rule

- The tier is written to the manifest's `assurance` field.
- The verifier shows the tier, never upgrades it.
