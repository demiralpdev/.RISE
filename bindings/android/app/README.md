# Minimal Android app (JNI proof)

Proves `System.loadLibrary("rise_jni")` + `RiseCore.hashFrame()` work inside a
real Android app process. The HASH logged under the RISE tag must match
`rise-core hash` of the same bytes on the desktop.

Status: REAL — built headlessly (no gradle, no Android Studio), installed and
run on the test device (2026-10-10):
`HASH=d7867236809b32fb8871b2af1d8a9e2b314639dcd5af7ba9eab2fd514be97837` (matches Mac).

Rebuilt 2026-10-17 (kotlinc 2.3.20 + JRE 26.0.1) because the 2026-10-14 APK
predated the Rank 1 source gate — its dex contained no
`ERR_VIRTUAL_CAMERA_REJECTED`, the fresh one does. `app-aligned.apk` sha256
`92bc7669689919f906c191a9500c6eb5eea88f30a00b7f732238e79f85857bd3`, `app.apk`
sha256 `3cbe2d083d446890cd27d4407184f2d4c297f66a4a6e57bc3883268e8d5817ff`.

The fresh build WAS installed and launched on the attached M2103K19C the same
day: the installed `/data/app/…/base.apk` sha256 equals the local `app.apk`
(`3cbe2d08…`), so the phone is running this exact artifact, and the process
stayed up with no crash-entry in `logcat -b crash` (which retires the doubt
raised by trap 4 below).

The button-driven capture path was then DRIVEN on that same phone (user-tapped,
device unlocked) and produced real seals under the RISE tag:
`RISE_VIDEO_FRAMES_EXTRACTED=14`, `…FRAME_HASHES_COUNT=14`,
`…TILES=56 MERKLE_ROOT=d898dffbe…`, `…SIGNATURE=5793c0f0…`,
`…RVID_SAVED=…video-001.mp4.rvid BYTES=1698306 FRAMES=14`, plus a photo seal.
Both manifests carry `"capture_source":"camera"` and no `RISE_CAMERA_REJECT:`
line fired (cam0 ACCEPTED — this phone has no virtual/external camera, so the
guard cannot false-positive here, but the reject path stays unexercised).
Cross-checked on the Mac: the photo frame hash `43d6b99b…` matches; `rise-core
unpack` of both the `.rise` and `.rvid` reproduces the on-disk manifests
byte-identically (modulo a trailing newline); the video frame hashes (14/14),
tiles (56/56) and Merkle root recompute exactly; and the ES256 signature
verifies `Verified OK` with `video-001.pub`. Note `rise-core verify <MANIFEST>
<FRAME>` is single-frame, so it returns `FrameMismatch` (rc 3) on the 14-frame
manifest — a CLI scope limit, not a chain fault.

## Build + install + run

Prerequisites: NDK-built `librise_jni.so` (see `../jni/`), kotlinc 2.3.20
(Android Studio's bundled compiler works: `/Applications/Android Studio.app/Contents/plugins/Kotlin/kotlinc/bin/kotlinc`), JDK 21+, SDK build-tools;35.0.0.

The whole module is compile-verified against android-35 with a bare
`android.jar` classpath (kotlinc 2.3.20 + JRE 26.0.1, `rc 0`, 31 classes,
2026-10-17) — the app is framework-only, so no AndroidX is needed to typecheck it.

```sh
./build_app.sh
adb install -r build/app.apk
adb logcat -c
adb shell am start -n rise.android.app/.MainActivity
adb logcat -d -s RISE      # HASH=<64 hex>
```

## Traps that WILL bite (they bit us — documented so they stay fixed)

1. `kotlin-stdlib.jar` must be dexed into the APK (d8 input) — otherwise the
   app crashes with `NoClassDefFoundError: kotlin/text/Charsets`.
2. If kotlin-stdlib pushes the dex past 64K method refs, d8 emits `classes.dex` +
   `classes2.dex`. Both 2026 `--release` builds produced a single `classes.dex`
   (minified stdlib fits), so `classes2.dex` is NOT always there — package every
   `classes*.dex` (native multidex, API 21+) instead of a fixed list.
3. macOS zip does not glob quoted patterns: `zip -u app.apk '*.dex'` fails with
   "name not matched" — pass the glob unquoted.
4. kotlinc 2.3.20 with build-tools 35.0.0's R8 8.6.2 prints
   `com.android.tools.r8.kotlin.c0.a: Should never be called` while dexing
   (Kotlin metadata newer than that R8 understands). The build still completes
   and the dex holds every class plus the stdlib — but do not assume it is
   benign: verify with `strings classes.dex | grep <known-symbol>` before
   shipping, or downgrade to a kotlinc whose metadata R8 8.6.2 can read.
5. The on-device frames dir persists across runs — `video-001.frames/` held a
   stale `frame-015.jpg` from an earlier run whose hash matched no manifest
   entry. The manifest is authoritative: compare against `frame_hashes` in
   order and never assume the dir holds exactly N fresh files.

Note: this app now wires the Camera2 capture flow (`../capture/`) end-to-end —
the photo and video seals above were produced by the 2026-10-17 build, which
predates the later code-health refactor of the capture glue (pure extraction,
behavior unchanged). Still future
work: the attest gating (`attest/RiseIntegrity.kt`, `RiseAssurance.kt`) is not
wired into the capture path.
