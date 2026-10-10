# ROADMAP

Status: 0.2.0 — per-OS proof details in bindings/MATRIX.md.

- Phase 0: skeleton + root docs — DONE (a889376)
- Phase 1: core hash + signing stub — DONE (MS1: manifest v1 + streaming hash, 110a38d)
- Phase 2: first binding end-to-end — DONE (macOS capture + e2e 8/8, d82c21e; Android core on-device, bb4386f; Windows attest + capture, 317abfa/4ab24a9)
- Phase 3: verify-web badges — DONE MVP (badge tree + trust + report + attack tests, 2412a2e; live Trust List sync pending upstream DNS)
- Phase 4: remaining binding shells — DONE (android/ios/linux/windows shells; Kotlin compiles against android-35, b86822e; JNI dlopen proof, 9bce8a9)
- Phase 5: device tests — PARTIAL (macOS e2e 8/8, Windows 33/33 + cross-device VALID, Android on-device hash + signature; iOS/Linux device runs pending)
- Phase 6: release + publishing prep — DONE 0.2.0 (CHANGELOG, Quickstart, CI, SECURITY, 28e0e50/34e8ebb)

Next (need user decisions or hardware):
- VBS enable on the Windows PC (reboot) -> gold-L2 re-run
- Linux dual-boot leg (boot Linux, install openssh-server, share IP)
- Android app shell (Android Studio SDK -> Kotlin capture run + System.loadLibrary in an APK)
- iOS leg (Xcode + Apple Developer account)
- Qualified TSA + PAdES certificate (eIDAS court weight)
