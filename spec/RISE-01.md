# RISE-01 — Core Capture Format

> Goal: produce proof at shutter time. No after-the-fact claims.

## 1. Core Formula

```
SHA256(raw frame) at shutter + sign + timestamp
```

- `raw frame`: the unprocessed frame coming from the sensor.
- `shutter`: the moment of capture, no delay.
- `sign`: signature with the device key.
- `timestamp`: RFC 3161 token.

## 2. Box Structure

```
[ftyp] [mvex-extension] [jumb] [mdat]
```

- `ftyp`: file type, compatible with ISO 21617-1:2026.
- `mvex-extension`: frame hash list (one SHA-256 per frame).
- `jumb`: C2PA 2.4 manifest embedded via JUMBF.
- `mdat`: raw media bytes.

## 2.1 Sealed File Naming

- The sealed artifact is `<original>.rise` — the bytes stay a valid JPEG
  (APP11/JUMBF evidence embedded), so any image viewer opens it; the
  extension marks the seal and is the product's file identity.
- Raw-frame flows (Windows YUY2) keep a sidecar `<original>.manifest.json`
  plus the raw file — APP11 requires a JPEG container.

## 3. Hash and Signature

- Hash: SHA-256 (mandatory, single algorithm).
- Default signature: ES256 (P-256 + ECDSA).
- Signature option: Ed25519 (constrained devices).
- Every frame hash lives inside the signed manifest.

## 4. C2PA 2.4 Compatibility

- The manifest follows the C2PA 2.4 schema.
- The JUMBF box opens in standard readers.
- RISE boxes do not break C2PA validators.

## 5. Manifest Fields

| Field | Description |
|---|---|
| `rise_version` | format version (`1`) |
| `frame_hashes` | list of frame SHA-256 hashes |
| `sig_alg` | `ES256` or `Ed25519` |
| `timestamp_token` | RFC 3161 token |
| `device_id` | anonymous device id |
| `assurance` | trust tier: `silver`, `gold-L2`, `gold-L4` |

## 6. Versioning

- Starts with `rise_version: 1`.
- New fields may be added, old fields are never removed.
- Readers ignore unknown fields.

## 7. Trust Requirements

- C2PA Trust List check is mandatory.
- Certificate revocation check is mandatory.
- If the list is unreachable the result is `unknown`, never `trusted`.

## References

- C2PA 2.4, ISO 21617-1:2026, RFC 3161.
