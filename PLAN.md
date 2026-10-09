# .RISE Unified Plan — PLAN.md

> Single file: core crypto plan + 5 OS binding plan + verify/trust plan.
> Written on top of the current state; every task is junior-assignable.
> Source specs: `spec/RISE-01.md`, `spec/threat-model.md`, `spec/assurance-levels.md`,
> `spec/solutions-root.md`, `spec/solutions-digital.md`, `spec/solutions-physical.md`, `spec/solutions-platform.md`.
> Roadmap: `ROADMAP.md` (Phase 0–6). Formula: `SHA256(raw frame) at shutter + sign + timestamp`.

## 0. Current State (real vs skeleton)

| Area | File/Dir | State | Note |
|---|---|---|---|
| Hash + Merkle | `core/src/lib.rs` | REAL | `sha256_frame`, `tile_hash`, `merkle_root`, `create_manifest_v1`, `verify_manifest_v1` work; tests green |
| CLI | `core/src/main.rs` | REAL | `hash/sign/verify` wired; `sign_manifest` prints `UNSIGNED:` prefix, no real signature |
| Dependencies | `core/Cargo.toml` | REAL | `sha2 0.10`, `clap 4`, `serde 1`, `serde_json 1`, `thiserror 2`; no signature/ASN.1/C2PA deps |
| macOS capture | `bindings/macos/capture/capture.swift`, `capture.sh` | REAL | single-frame JPEG via AVCapture/ffmpeg; device parsed by name (Rapoo priority); manifest generated via core CLI |
| macOS shell | `bindings/macos/README.md` | SKELETON | Secure Enclave goal written, no code |
| android shell | `bindings/android/README.md` | SKELETON | StrongBox + Play Integrity goal written, no code |
| ios/linux/windows | `bindings/{ios,linux,windows}/README.md` | SKELETON | thin shell + gold-gate checklist, no code |
| Verifier | `verify-web/` | REAL (MVP) | FastAPI + badge tree (8 tests green); Trust List/OCSP/TSA missing |
| Root synthesis | `spec/solutions-root.md` | SPEC | 5 Golden Rules + supply/OS/crypto/governance; not implemented |

Critical gaps: (1) no real signature (`sign_manifest` stub), (2) no `timestamp_token/assurance` schema
fields beyond v1, (3) no RFC 3161 stamp, (4) no C2PA 2.4/JUMBF writer, (5) no Trust List + OCSP hard-fail,
(6) no FFI exposed, (7) verify-web has no Trust infra.

## 1. Target Architecture (ascii)

```
                     ┌─────────────────────────────────────────────┐
                     │              DEVICE (5 OS shells)           │
                     │  android/ios/macos/windows/linux           │
                     │  bindings/*/ → ONLY capture + attest        │
                     │  NO SIGNING LOGIC (thin shell rule)         │
                     └──────────────┬──────────────┬───────────────┘
                    raw frame +     │              │ hardware attestation
                    meta (sensor,   │              │ (StrongBox/SE/
                    time, GPS)      │              │  TPM/Play Integrity)
                                    ▼              ▼
                     ┌─────────────────────────────────────────────┐
                     │               core/ (single trust core)     │
                     │ core/src/lib.rs                             │
                     │  1. sha256_frame (DONE)                     │
                     │  2. tile_hash + merkle_root (DONE)          │
                     │  3. manifest RISE-01 §5 (DONE, MS1)         │
                     │  4. ES256/Ed25519 signature (R-103)         │
                     │  5. RFC 3161 multi-TSA stamp (R-201)        │
                     │  6. C2PA 2.4 JUMBF embed (R-401)            │
                     └──────────────────────┬──────────────────────┘
                                            │ sealed package
                                            │ (mp4 box: ftyp/mvex/jumb/mdat)
                                            ▼
                     ┌─────────────────────────────────────────────┐
                     │        verify-web (verifier + badge)        │
                     │  hash/Merkle → signature →                  │
                     │  Trust List+OCSP (hard-fail) →              │
                     │  TSA cross-check → badge                    │
                     │  gold-L4 / gold-L2 / silver / red           │
                     │  Rule: on suspicion DOWNGRADE, never upgrade│
                     └─────────────────────────────────────────────┘
```

Data flow: shutter -> raw frame hash (`at shutter`, no delay) -> manifest ->
signature with the device key -> RFC 3161 stamp -> JUMBF/C2PA embed -> sharing via verify link.
Sharing rule (`spec/solutions-platform.md` §8): verify link only; no file submission,
screenshots carry no metadata, strip = untrusted.

Trust decisions (fail-closed, `spec/solutions-root.md` 5 Golden Rules):
an unsigned frame is not evidence; trust no single point (multi-TSA + Trust List + OCSP);
the badge is downgraded on suspicion; if the list is unreachable the result is `unknown`, never green.

## 2. Milestones and Tasks

Prefixes: `R` = core crypto, `A` = android, `I` = ios, `M` = macos, `L` = linux,
`W` = windows, `V` = verify-web, `T` = test/infrastructure.
Every task is one file + one PR sized (junior-assignable).

### MS1 — Manifest v1 + streaming hash (ROADMAP Phase 1; base: `core/`) — DONE

| ID | Task | File | Acceptance |
|---|---|---|---|
| R-001 | ManifestV1 schema per RISE-01 §5 (`rise_version`, `frame_hashes`, `tile_hashes`, `merkle_root`, `sig_alg`, `timestamp_ms`, `device_id`) | `core/src/lib.rs` | tests green |
| R-002 | Streaming hash (large-file safe) | `core/src/lib.rs` | stream == slice hash |
| R-003 | CLI `sign`/`verify` wired to real files + exit codes | `core/src/main.rs` | e2e pass |
| T-001 | serde/thiserror deps + error type | `core/Cargo.toml`, `core/src/lib.rs` | clean build |

### MS2 — Real signature + chain hardening (threat model §1, §4)

| ID | Task | File | Acceptance |
|---|---|---|---|
| R-103 | `sign_manifest` stub -> `Signer` trait (software ES256, key from file) | new `core/src/signer.rs` | `UNSIGNED:` prefix removed; software ES256 verified in tests |
| R-104 | Ed25519 option (selected via `sig_alg`, RISE-01 §3) | `core/src/signer.rs` | sign/verify tests green for both algorithms |
| R-105 | CLI `sign`/`verify` with a real key (`--key` flag) | `core/src/main.rs` | `rise-core sign` + `rise-core verify` pass end-to-end |
| R-106 | Short lifetime + rotation rule (<=90 days) + keygen command | `core/src/main.rs` | expired key rejected in test |
| T-101 | Add signature dependencies + lockfile | `core/Cargo.toml`, `core/Cargo.lock` | clean build; minimal dep set |

### MS3 — Timestamp + transparency (threat model §1, §4; `solutions-digital.md` §4–5)

| ID | Task | File | Acceptance |
|---|---|---|---|
| R-201 | RFC 3161 TSA client (token + embed) | new `core/src/timestamp.rs` | `timestamp_token` carries a real token |
| R-202 | Multi-TSA cross-stamping (>=2 QTSP; single TSA invalid, `solutions-root.md` §3) | `core/src/timestamp.rs` | mismatched tokens rejected in test |
| R-203 | Rekor v2 inclusion client (inclusion + witness quorum; NOT a time proof) | new `core/src/transparency.rs` | checkpoint + consistency proof verified; else `unknown` |
| R-204 | Tile-swap protection: tile Merkle root bound to a signed field | `core/src/lib.rs` | reordered tile list survives verification |
| R-205 | Strip/screenshot rule: missing manifest `untrusted`, missing pixel declaration red | `core/src/lib.rs` | `VerifyDecision` enum (red/suspicious/unknown/valid + reason) |
| T-201 | Timestamp/manifest golden test vectors (recorded TSA response, offline) | new `core/tests/vectors.rs` | offline `cargo test` green |

### MS4 — Remaining binding shells (ROADMAP Phase 4)

Common principle (all shells): capture the frame + get attestation, send to `core/`; NO signing logic.
Every OS has its gold-gate checklist in its own README.

| ID | Task | File | Acceptance |
|---|---|---|---|
| A-101 | Camera frame capture (Camera2/CameraX) + raw byte output | new `bindings/android/capture/` | frame byte verified with `rise-core hash` |
| A-102 | StrongBox key + Play Integrity (no STRONG, no gold; BASIC + clean chain = gold-L2 max) | new `bindings/android/attest/` | decision table tested |
| A-103 | Mock/GPS off auto-downgrade (`solutions-platform.md` §5, §9) | `bindings/android/attest/` | no gold while mock is on (test) |
| I-101 | AVFoundation frame capture + raw byte output | new `bindings/ios/capture/` | frame byte verified with core |
| I-102 | Secure Enclave + DeviceCheck/App Attest (downgrade on weak attest) | new `bindings/ios/attest/` | `bindings/ios/README.md` checklist wired to code |
| L-101 | V4L2 frame capture script (macOS `capture.sh` equivalent) | new `bindings/linux/capture/capture.sh` | output + core hash match |
| L-102 | TPM/fTPM attestation if present, else silver cap (`solutions-platform.md` §10) | new `bindings/linux/attest/` | gold path closed on unattested machines (test) |
| W-101 | MediaFoundation frame capture + raw byte output | new `bindings/windows/capture/` | frame byte verified with core |
| W-102 | TPM + Windows Hello/Pluton attestation (downgrade if weak) | new `bindings/windows/attest/` | `bindings/windows/README.md` checklist wired to code |
| T-401 | 5 OS matrix test (which ran on real hardware, which is a stub) | new `bindings/MATRIX.md` | status + e2e result table per OS |

### MS5 — verify-web + trust (ROADMAP Phase 3)

Badge language is fixed (`spec/solutions-platform.md` §2): Gold/Silver/Red + unverifiable.
Legal language is mandatory (§7): "not evidence, preliminary finding".

| ID | Task | File | Acceptance |
|---|---|---|---|
| V-101 | Hash/Merkle recomputation (core parity) | `verify-web/verify.py` | done (MS1 MVP) |
| V-102 | Signature verification (ES256 + Ed25519 per `sig_alg`) | `verify-web/verify.py` | one changed frame = red |
| V-103 | C2PA Trust List + OCSP hard-fail (no response = `unknown`, revoked = `untrusted`, RISE-01 §7) | new `verify-web/trust.py` | never green when the network is down (test) |
| V-104 | Multi-TSA cross-check (reject on mismatch) | `verify-web/trust.py` | single-TSA packages rejected |
| V-105 | Badge decision tree full version (gold-L4/L2, silver, red + one-line reason) | `verify-web/verify.py` | no STRONG, no L4; broken chain/replay = straight red; downgrade on suspicion |
| V-106 | Sharing model: verify link only; strip = untrusted | `verify-web/app.py` | no file download button; link signed |
| V-107 | eIDAS report output PAdES/XAdES + witness independence score | new `verify-web/report.py` | same device/IP witnesses flagged; score shown in report |
| T-501 | Fake-file + re-capture + edit attack tests (threat model §1, §2, §4) | new `verify-web/tests/attacks.py` | all 3 attacks produce red/suspicious |

## 3. Acceptance Gates (G0–G5, fail-closed)

| Gate | Milestone output | Pass condition | Red line |
|---|---|---|---|
| G0 | Plan approval | This PLAN.md + spec consistency approved | No code start without approval |
| G1 | MS1 | `cargo test` green; manifest carries RISE-01 §5 fields; CLI sign/verify real | `UNSIGNED:` residue = STOP |
| G2 | MS2/MS3 | Multi-TSA + Rekor inclusion + VerifyDecision; golden vectors offline green | Single TSA accepted = STOP; Rekor counted as time proof = STOP |
| G3 | MS4 macOS | JPEG -> manifest -> verify loop passes on target machine; USB = silver | Unconnected capture = STOP |
| G4 | MS4 rest | Real e2e on at least 2 OS; `bindings/MATRIX.md` current; no signing logic in any shell | Signing code in a shell = STOP |
| G5 | MS5 | Trust List + OCSP hard-fail verified live; attack tests produce red; badge reasons one line | Green while list unreachable = STOP (critical) |

## 4. File-by-File Task List (junior-assignable)

| # | File | Tasks | Size |
|---|---|---|---|
| 1 | `core/src/lib.rs` | R-004 tile-root binding, R-205 VerifyDecision | Medium (2 PRs) |
| 2 | `core/src/signer.rs` (new) | R-103 Signer trait + ES256, R-104 Ed25519 | Medium (2 PRs) |
| 3 | `core/src/timestamp.rs` (new) | R-201 single TSA, R-202 multi-TSA | Medium (2 PRs) |
| 4 | `core/src/transparency.rs` (new) | R-203 Rekor v2 inclusion | Small (1 PR) |
| 5 | `core/src/main.rs` | R-105 `--key` sign/verify, R-106 `keygen` | Small (2 PRs) |
| 6 | `core/Cargo.toml` + `core/Cargo.lock` | T-101 signature/time deps | Small (1 PR) |
| 7 | `core/tests/vectors.rs` (new) | T-201 golden vectors | Small (1 PR) |
| 8 | `bindings/macos/attest/` (new) | M-103 Secure Enclave, M-104 light-challenge, M-105 gold-gate | Large (3 PRs) |
| 9 | `bindings/macos/capture/e2e.sh` (new) | T-301 macOS loop test | Small (1 PR) |
| 10 | `bindings/android/capture/` + `attest/` (new) | A-101/A-102/A-103 | Large (3 PRs) |
| 11 | `bindings/ios/capture/` + `attest/` (new) | I-101/I-102 | Large (2 PRs) |
| 12 | `bindings/linux/capture/capture.sh` + `attest/` (new) | L-101/L-102 | Medium (2 PRs) |
| 13 | `bindings/windows/capture/` + `attest/` (new) | W-101/W-102 | Large (2 PRs) |
| 14 | `bindings/MATRIX.md` (new) | T-401 5 OS status table | Small (1 PR) |
| 15 | `verify-web/trust.py` (new) | V-103/V-104 Trust+TSA | Medium (2 PRs) |
| 16 | `verify-web/verify.py` | V-102/V-105 verification + badge tree | Medium (2 PRs) |
| 17 | `verify-web/report.py` (new) | V-107 eIDAS + witness | Medium (1–2 PRs) |
| 18 | `verify-web/tests/attacks.py` (new) | T-501 attack tests | Small (1 PR) |

Every PR: one task ID + test evidence + spec reference.

## 5. Quality Gates (mandatory on every PR)

```sh
# Rust (core/)
cargo test      # all tests green, no merge without tests
cargo clippy -- -D warnings   # warning = error
cargo fmt --check             # formatting clean
# Binding scripts
shellcheck bindings/*/*/*.sh  # if available; else at least `bash -n`
# Swift/Kotlin (device code)
swiftc --typecheck / kotlinc compile cleanliness (on target machine)
```

- [ ] `cargo test` green
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] At least 1 test per new public function
- [ ] Spec reference in the PR description (`spec/...` + section)
- [ ] Badge/assurance-changing PR includes a downgrade test (never upgrade on suspicion)

## 6. Ordering and Dependencies

```
G0 → MS1 (DONE: R-001..R-003, T-001) → G1
   → MS2 (R-103..R-106, T-101) → real signature
   → MS3 (R-201..R-205, T-201) → G2
   → MS4 macOS e2e (M-103..M-105, T-301) → G3   [ROADMAP Phase 2]
   → MS5 verify core (V-102..V-105) → G5-partial [badge needed early to
      verify the macOS e2e; Phase 3 pulled forward]
   → MS4 remaining bindings (A/I/L/W) → G4      [ROADMAP Phase 4]
   → MS5 rest (V-106/V-107, T-501) → G5        [ROADMAP Phase 3 completed]
```

Note: ROADMAP Phase 5 (device tests) closes with the MS4 e2e runs, Phase 6 (release) with G5.
Deliberate pull-forward: a binding e2e does not count as "pass" without verify-web verification.

## 7. Risks and Contracts

| Risk | Mitigation | Spec |
|---|---|---|
| Single TSA/CA trust | Multi-TSA mandatory + Trust List pin + OCSP hard-fail | `solutions-root.md` §3, `solutions-digital.md` §4 |
| Key theft | <=90-day rotation + revocation list; Gold TEE/SE never exports | `solutions-digital.md` §2, `solutions-root.md` §1 |
| Screen re-capture | Nonce light challenge + depth + frame interval check | `solutions-physical.md` §1, §10 |
| Fake USB/MIPI source | USB = automatic silver; no L4 on unencrypted MIPI; serial matching | `solutions-physical.md` §5, §7, §11 |
| Insider single-approval | Multisig (2 signatures) + append-only log | `solutions-platform.md` §3 |
| List unreachability | Show `unknown`, never green (fail-closed) | `RISE-01.md` §7 |
| PQC future | Keep the dual-sign path open, no single-algorithm lock-in | `solutions-root.md` §3 |
| Court language | "Not evidence, preliminary finding" + eIDAS report | `solutions-platform.md` §7 |
