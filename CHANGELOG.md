# Changelog

## 0.2.0 (2026-10-10)

- MS1: manifest v1 schema (`rise_version`, `frame_hashes`, `tile_hashes`, `merkle_root`, `sig_alg`, `timestamp_ms`, `device_id`) with streaming SHA256 hash.
- MS2: real ES256/Ed25519 signing via `Signer` trait plus `keygen` CLI (`--key` sign / `--pubkey` verify, 90-day rotation rule).
- JUMBF pack: C2PA container `pack`/`unpack` CLI embedding the manifest into JPEG via APP11, with `assurance` + `timestamp_token` schema fields and FFI export.
- MS3: RFC 3161 timestamp client (`stamp`, multi-TSA cross-stamp) plus Rekor v2 transparency inclusion client and `VerifyDecision` enum.
- MS4 wave1: stamp-then-sign ordering plus linux/android/ios/windows binding shells (capture + attest, no signing logic in shells).
- MS4 wave2: macOS capture-to-manifest e2e loop script plus Android JNI and iOS XCFramework scaffolds plus trust-list sync script.
- verify-web MVP: FastAPI badge API (gold-L4/gold-L2/silver/red) with hash/Merkle recomputation and signature verification.
- verify-web trust: C2PA Trust List + OCSP hard-fail and multi-TSA cross-check (`trust.py`, fail-closed to `unknown`, never green when unreachable).
- verify-web report: eIDAS PAdES/XAdES report bundle with witness-independence score plus fake-file/re-capture/edit attack tests.
- e2e: macOS frame-to-manifest-to-verify loop with on-disk artifacts (`bindings/target/`) and 5-OS status table (`bindings/MATRIX.md`).
