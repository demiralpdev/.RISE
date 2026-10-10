# Minimal Android app (JNI proof)

Proves `System.loadLibrary("rise_jni")` + `RiseCore.hashFrame()` work inside a
real Android app process. The HASH logged under the RISE tag must match
`rise-core hash` of the same bytes on the desktop.

Status: REAL — built headlessly (no gradle, no Android Studio), installed and
run on the test device (2026-10-10):
`HASH=d7867236809b32fb8871b2af1d8a9e2b314639dcd5af7ba9eab2fd514be97837` (matches Mac).

## Build + install + run

Prerequisites: NDK-built `librise_jni.so` (see `../jni/`), kotlinc 2.0.21
(`/tmp/kotlinc`), JDK 21, SDK build-tools;35.0.0.

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
2. kotlin-stdlib pushes the dex past 64K method refs: d8 emits `classes.dex` +
   `classes2.dex` — package EVERY `classes*.dex` (native multidex, API 21+).
3. macOS zip does not glob quoted patterns: `zip -u app.apk '*.dex'` fails with
   "name not matched" — pass the glob unquoted.

Note: this app proves the JNI bridge in an app process only. The Camera2
capture flow (`../capture/`) still needs camera permission UI + an app that
wires RiseCapture with the attest gating — future work.
