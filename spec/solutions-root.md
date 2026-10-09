# Root Solutions — Synthesis of Root Fixes

> Sources: `RISE-01`, `solutions-digital/physical/platform`, `threat-model`, `assurance-levels`.
> Core: `SHA256(raw frame) at shutter + sign + timestamp`. An unsigned frame is not evidence.

## 1. Supply chain (hardware root)
- Gold only from an internal attested sensor; external sources never get gold.
- Sensor serial matching mandatory; on mismatch red, on swap key revocation.
- Encrypted MIPI required for L4; unencrypted MIPI cannot grant L4.
- USB camera = automatic silver; unattested source red.
- Gold keys live in TEE/SE, never leave; Silver uses short lifetime + rotation.

## 2. OS (path and build)
- The L4 hash is taken at sensor output before entering the OS; a secure pipeline is used.
- Reproducible builds mandatory: same source = same bits, SBOM published.
- OS attestation + Play Integrity are signals only; server-side verification required.
- Mock/GPS off triggers automatic downgrade (silver at most).
- Old devices = automatic silver; API-level check.

## 3. Crypto (signature and time)
- Signature: ES256 default, Ed25519 option; PQC-ready: ML-DSA preparation.
- Dual-sign path: classic + PQC dual signature path open, no single-algorithm lock-in.
- Time: RFC 3161 QTSP mandatory, multi-TSA cross-stamping; a single TSA is invalid.
- Rekor v2 is not a time proof; only inclusion + witness quorum.
- Trust List + OCSP hard-fail; unreachable = `unknown`, revoked = `untrusted`.
- LTV: chain + stamp + revocation proof stored for long-term validation.
- Rotation <= 90 days; the old key is on the revocation list.

## 4. Governance (trust distribution)
- No single trust point: no person, TSA, CA, or server grants trust alone.
- Multisig: 2 signatures required for critical approvals; one person cannot approve (RBAC + separation of duties).
- Log is append-only; insider actions are dual-signed and auditable.
- Witness independence score below threshold -> downgrade; same device/IP witnesses are flagged.
- Sharing is verify-link only; strip = untrusted, screenshot = red.
- Court language: "not evidence, preliminary finding"; eIDAS report (PAdES/XAdES) is generated.

## 5 Golden Rules
1. An unsigned frame is not evidence: missing hash + signature + stamp trio = red.
2. Trust no single point: multi-TSA + Trust List + OCSP are checked together.
3. Lock the path: L4 internal encrypted MIPI + depth + nonce light challenge.
4. Govern with dual signatures: multisig on critical decisions, dual-sign path for the future.
5. Stay fail-closed: downgrade on suspicion, never green when the list is unreachable; store LTV.
