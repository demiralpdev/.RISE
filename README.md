<p align="center">
  <img src=".github/banner.svg" alt="RISE" width="480">
</p>

[![CI](https://github.com/demiralpdev/.RISE/actions/workflows/ci.yml/badge.svg)](https://github.com/demiralpdev/.RISE/actions)
[![Rust tests](https://img.shields.io/badge/rust_tests-36_passed_3_ignored-blue)](https://github.com/demiralpdev/.RISE/tree/main/core)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)
[![Release](https://img.shields.io/github/v/release/demiralpdev/.RISE)](https://github.com/demiralpdev/.RISE/releases)

.rise is an open source photo and video authenticity protocol. It hashes each camera frame at shutter time, signs the hash with the device key, stamps it with an independent timestamp, and seals the result into the file itself, so anyone can later check whether a picture is original or has been altered.

## How it works

```
capture --> hash --> sign --> timestamp --> seal --> verify
  |          |        |          |            |         |
camera    SHA256   ES256 /    RFC 3161    JUMBF /    badge:
frame     Merkle   Ed25519    multi-TSA   C2PA       gold / silver /
                   + ML-DSA   stamp       embed      red / unknown
                   dual-sign
```

1. **Capture.** A thin OS shell grabs the raw frame and device attestation. Shells never sign, they only capture.
2. **Hash.** `rise-core` computes a streaming SHA256 over the frame plus a tile Merkle root.
3. **Sign.** The manifest is signed with ES256 or Ed25519, with optional ML-DSA-65 post-quantum dual signing.
4. **Timestamp.** An RFC 3161 stamp is applied before signing, so the signature covers the time token. A second TSA can cross-stamp it.
5. **Seal.** The signed manifest is packed into the JPEG via a JUMBF APP11 segment.
6. **Verify.** Anyone can recompute the hash, check the signature and timestamps, and get a badge: gold, silver, red, or unknown. On suspicion the badge is downgraded, never upgraded. If trust data is unreachable the answer is unknown, never green.

## Quick links

- Public verifier: **https://riseproof.me**
- Mirrors: https://universe-rise.netlify.app/ and https://demiralpdev.github.io/.RISE/
- Releases: https://github.com/demiralpdev/.RISE/releases
- Roadmap: [ROADMAP.md](ROADMAP.md)
- Threat model: [spec/threat-model.md](spec/threat-model.md)
- Security policy: [SECURITY.md](SECURITY.md)
- Platform status: [bindings/MATRIX.md](bindings/MATRIX.md)
- Changelog: [CHANGELOG.md](CHANGELOG.md)

The web verifier runs fully client side. Files never leave the browser.

## Quickstart

Prerequisites: Rust stable toolchain and (for the verifier tests) Python 3 with pytest.

```bash
# clone
git clone https://github.com/demiralpdev/.RISE.git
cd .RISE

# build the core CLI
cd core
cargo build
cargo test

# hash a frame
./target/debug/rise-core hash path/to/frame.jpg

# create a keypair (private key is written with mode 0600)
./target/debug/rise-core keygen my-device

# sign a frame into a manifest (stamp-then-sign with a live TSA)
./target/debug/rise-core sign path/to/frame.jpg \
  --key my-device.key \
  --alg es256 \
  --tsa https://freetsa.org/tsr \
  --out frame.manifest.json

# verify a manifest against its frame
./target/debug/rise-core verify frame.manifest.json path/to/frame.jpg \
  --pubkey my-device.pub

# seal the manifest into the JPEG, then read it back
./target/debug/rise-core pack path/to/frame.jpg frame.manifest.json --out sealed.jpg
./target/debug/rise-core unpack sealed.jpg

# post-quantum dual signing (ML-DSA-65 alongside the classical signature)
./target/debug/rise-core sign path/to/frame.jpg \
  --key my-device.key \
  --pqc-key my-device.pqc.seed \
  --out frame.manifest.json
./target/debug/rise-core verify frame.manifest.json path/to/frame.jpg \
  --pubkey my-device.pub \
  --pqc-pub my-device.pqc.pub
```

Verifier tests:

```bash
cd verify-web
python -m pytest tests/ -q
```

## Install (npm)

```bash
npm install -g @rise/cli
rise --help
```

The npm package wraps the `rise-core` CLI (`hash`, `sign`, `verify`, `keygen`, `pack`, `unpack`, `stamp`). See [package/](package/) for details.

## Platform status

Source of truth: [bindings/MATRIX.md](bindings/MATRIX.md). Summary as of 0.2.0:

| OS | Capture | Attest | End to end | Tier ceiling |
|---|---|---|---|---|
| macOS | REAL (AVCapture frame on disk) | Stub (no Secure Enclave code yet) | REAL (capture to manifest) | Silver max until attest lands |
| Windows | REAL (MediaFoundation frame, hashed on the PC) | REAL gold-L2 (TPM P-256 key, VBS running) | REAL (keygen, live FreeTSA stamp, `VALID (signed)`) | gold-L2 proven, L4 unreachable by design |
| Android | REAL core on device (on-device SHA256 matches, JNI bridge loads in an APK) | Stub (StrongBox and Play Integrity not run) | REAL for core CLI and in-app hash | L2 max by design until STRONG verdict |
| Linux | REAL on real hardware (camera frame, keygen, live stamp, `VALID (signed)`) | REAL report-only (TPM2 detected) | REAL on device plus cross-check on macOS | Silver always (gold path closed by policy) |
| iOS | REAL core in simulator (hash matches byte for byte, XCFramework builds) | Stub (needs a real iPhone and Apple servers) | REAL in simulator, not run on a device | Unproven on device, L2/L4 candidate by design |

Every shell follows the thin shell rule: capture the frame, fetch attestation, hand both to `core/`. No signing logic lives in any binding.

## Security

Classical signatures are ES256 and Ed25519, selected via `sig_alg`. Post-quantum dual signing uses ML-DSA-65 (`--pqc-key` / `--pqc-pub`), so a manifest can carry both a classical and a quantum resistant signature. Keys rotate within 90 days, and verification is fail closed: a missing manifest means untrusted, a revoked or unreachable trust anchor means unknown, never green.

Red team results ([REDTEAM_FINDINGS.md](REDTEAM_FINDINGS.md), version 0.2.0): 15 attack scenarios were run against the badge and trust chain, covering forged gold claims, a compromised caller, trust anchor spoofing, stolen keys, JUMBF transplanting, field injection, verify link poisoning, type confusion, and oversized manifests. Gold stayed unreachable in every case. Two critical findings were fixed in the same pass (production trust list now ships empty, verify link IDs bind the full manifest hash), and the remaining open items (trust list integrity pinning, revocation checks, TPM quote binding) are documented in the file.

## Repository structure

```
.RISE/
  core/            Rust trust core: hash, sign, timestamp, seal, verify (rise-core CLI)
  bindings/        Thin OS shells: android, ios, macos, windows, linux (capture + attest only)
  bindings/MATRIX.md   5-OS status table with evidence
  verify-web/      Verifier and badge API (FastAPI) plus trust, report, and attack tests
  spec/            Protocol specs: RISE-01, threat model, assurance levels, solutions
  package/         npm wrapper (@rise/cli)
  PLAN.md          Unified build plan with milestones and acceptance gates
  ROADMAP.md       Phase 0 to 6 roadmap
```

## License

Apache-2.0. See [LICENSE](LICENSE).
