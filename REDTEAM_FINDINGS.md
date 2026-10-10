# Red Team Findings — .rise 0.2.0 (2026-10-10)

> Goal: break our own product — reach a gold badge or break the fail-closed chain.
> Verdict: **GOLD UNREACHABLE** (all 8 attack methods failed). 5 real findings:
> 2 fixed in this pass, 3 documented open.

## Attacks run (all real executions)

| # | Attack | Result |
|---|---|---|
| A1 | forged gold-L4 claim + garbage signature | silver (gold cap holds) |
| A2 | compromised caller passing strong_signal=True | silver — gold-L4 NEVER issued (cap total) |
| A3 | old field-name injection (`guvence`) | silver (ignored) |
| B1 | trust-anchor spoof, wrong subject | unknown |
| B1b | trust-anchor spoof, EXACT subject read from the PUBLIC trust-list.json | **TRUSTED — finding F1 (critical)** |
| B2 | self-signed | untrusted |
| B3 | unknown CA | unknown |
| C | stolen key: sign fake content with a copied key file | VALID — signature binds the caller's key (findings F3/F5) |
| D | gold manifest packed into an unrelated image (JUMBF) | INVALID: frame hash mismatch (hash binds) |
| E | extra field injection | silver (harmless) |

## Findings

- **F1 (critical, FIXED):** the shipped `verify-web/trust-list.json` contained a
  TEST anchor (`test-anchor-ca`); the file is public, so anyone could claim its
  subject and get a TRUSTED verdict from check_trust_list. Fix: the prod list now
  ships with EMPTY anchors (fail-closed unknown for every issuer), the anchor test
  moved to its own fixture, and two lock tests added
  (`test_prod_trust_list_ships_empty`, `test_prod_list_never_trusts_any_claim`).
- **F2 (fixed):** the silver reason string said "Software signature valid" while
  verify-web never crypto-verifies the ES256 signature (V-102 pending). The string
  now honestly says "signature NOT crypto-verified (MS1)".
- **F3 (design, documented):** core verify_decision trusts whichever pubkey the
  caller passes; key-identity gating belongs to verify-web + Trust List.
- **F4 (open):** trust-list.json on a production server needs integrity
  protection (WORM/pinning) — swap attack otherwise.
- **F5 (open):** no revocation check in core (key-theft window) — spec'd TODO
  (OCSP + short-lived keys, MS3+ work).

## Gold-reachability summary

Every path to gold-L4/L2 requires: real hardware key + attestation + STRONG
verdict + Trust List. The MS1 cap (gold never issued) held even against a
compromised caller (A2). The badge stays silver/red until the real trust
infrastructure lands — exactly as designed.

## Round 2 (2026-10-10, deeper pass)

| # | Attack | Result |
|---|---|---|
| R1 | verify_link poisoning: two different manifests sharing manifest[:32] get the same /v/ id | **CONFIRMED -> FIXED** (the id now binds the full manifest sha256) |
| R2 | type confusion: frame_hashes/tile_hashes/merkle_root as plain strings | contained by the tile-list equality -> HARDENED (explicit list/str type checks, fail-closed) |
| R3 | duplicate JSON keys (python last-wins vs rust serde rejects) | divergence documented; both sides fail-closed for signing |
| R4 | 2MB manifest (API DoS surface, no size cap) | **CONFIRMED -> FIXED** (1MB cap, red) |
| R5 | assurance case/type confusion (GOLD-L4, gold-l2, list, dict, None) | ALL red — the tier enum check holds |
| F7 (new, open) | Windows gold-L2 is locally self-asserted: attest.exe computes the tier from local facts; the NCrypt key attestation + VBS quote are not yet bound into the manifest | documented — the binding is required before gold-L2 is third-party verifiable |

Post-fix re-attack: prefix_same=True but ids_now_differ=True — the poisoning
path is closed. Gold remains unreachable.
