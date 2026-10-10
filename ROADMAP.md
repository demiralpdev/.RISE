# ROADMAP

Status: 0.2.0 — per-OS proof details in bindings/MATRIX.md.

- Phase 0: skeleton + root docs — DONE (a889376)
- Phase 1: core hash + signing stub — DONE (MS1: manifest v1 + streaming hash, 110a38d)
- Phase 2: first binding end-to-end — DONE (macOS capture + e2e 8/8, d82c21e; Android core on-device, bb4386f; Windows attest + capture, 317abfa/4ab24a9)
- Phase 3: verify-web badges — DONE MVP (badge tree + trust + report + attack tests, 2412a2e; live Trust List sync pending upstream DNS)
- Phase 4: remaining binding shells — DONE (android/ios/linux/windows shells; Kotlin compiles against android-35, b86822e; JNI dlopen proof, 9bce8a9)
- Phase 5: device tests — PARTIAL (macOS e2e 8/8, Windows 33/33 + cross-device VALID, Android on-device hash + signature; iOS/Linux device runs pending)
- Phase 6: release + publishing prep — DONE 0.2.0 (CHANGELOG, Quickstart, CI, SECURITY, 28e0e50/34e8ebb)

Next — planned waves (see QTSP_GUIDE.md for the qualified-TSA flow):

- Wave 1 (free, autonomous): Kotlin Camera2 capture wired into the minimal APK
  (real phone camera + hash + signature in the app process; SDK already installed).
- Wave 2 (free, autonomous): verify-web deepening - embed the trust list in the
  static verifier, multi-TSA display, Oracle Free Tier VM for the API server.
- Wave 3 (user action, paid): Mobil İmza activation + KamuSM e-Damga (or an EU
  QTSP) -> hand the endpoint to the project -> qualified-stamp proof + court packet.
- Wave 4 (user action): iPhone -> Xcode free 7-day provisioning -> device proof.
- DONE this cycle: VBS gold-L2 on Windows, Nobara Linux leg, Android JNI + APK
  proof, public verifier live, red team round 2 (link-poisoning fix), PQC dual-sign.
