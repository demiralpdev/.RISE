# Solutions: Digital Attacks (11)

Stack: C2PA 2.4 JUMBF + ES256 + Rekor v2. Principle: server-side verification mandatory.
Trust List frozen at 2026-01-01 + OCSP mandatory. Time proof is RFC3161 QTSP only; Rekor is not a time proof.

## 1. Self-signed certificate
- Rule: only CAs on the Trust List are accepted; self-signed is rejected.
- Technique: chain validation + OCSP (hard-fail) + Trust List pin.
- Residual: CA compromise; detected via transparency log.

## 2. Certificate theft
- Rule: short-lived certificates + rotation mandatory.
- Technique: lifetime <= 90 days, automatic rotation, key in HSM/server, revoke on leak.
- Residual: theft-to-revocation window; shortened with OCSP.

## 3. Revoked certificate use
- Rule: OCSP mandatory, soft-fail forbidden.
- Technique: OCSP/CRL query on every verification; no response = untrusted.
- Residual: OCSP responder outage keeps access down (fail-closed).

## 4. Timestamp forgery
- Rule: RFC3161 QTSP mandatory; Rekor time is not proof.
- Technique: QTSP token embedded in JUMBF, chain + nonce verified.
- Residual: QTSP compromise; reduced with multi-QTSP cross-check.

## 5. Rekor fork / log split
- Rule: Rekor v2 inclusion proof + witness quorum required.
- Technique: signed checkpoint, consistency proof, multiple witnesses.
- Residual: long-term split-view; detected via monitoring.

## 6. Tile swap
- Rule: the tile Merkle root is signed with ES256.
- Technique: every tile hash is bound to the tree, the root is signed in the manifest, order changes are rejected.
- Residual: undetectable if the source tile is already corrupt.

## 7. Metadata strip (JUMBF removal)
- Rule: strip = untrusted.
- Technique: missing/broken manifest gets an "unverifiable" label, no green is issued.
- Residual: legitimate cropping is also penalized (accepted cost).

## 8. Screenshot laundering
- Rule: screenshot = red + watermark detection.
- Technique: screen/compression artifacts + missing watermark -> not a source proof.
- Residual: artifacts weaken on high-quality re-capture.

## 9. AI inpainting / edit
- Rule: every pixel change is declared with an assertion.
- Technique: C2PA action (createdBy/editing) + padded hash comparison; red without declaration.
- Residual: semantic but pixel-consistent fakes require human review.

## 10. Key extraction
- Rule: short lifetime + rotation + server-side signing.
- Technique: signing happens on the server/HSM, never on the client; the key never lives on the client.
- Residual: server compromise; limited with monitoring + rotation.

## 11. Play Integrity spoof
- Rule: not trustworthy on its own; server-side verification required.
- Technique: Play Integrity + backend nonce + device binding; a signal alone never grants green.
- Residual: root/emulator bypasses; reduced with layered checks.
