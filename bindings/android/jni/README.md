# Android JNI Bridge (`bindings/android/jni/`)

> Status: REAL - NDK r27 aarch64-v8a build + on-device proof (2026-10-10).
> - librise_jni.so built via CMake (Unix Makefiles, android.toolchain.cmake),
>   statically linked against librise_core.a (cargo-ndk release build).
> - llvm-nm -D: Java_com_rise_core_RiseCore_hashFrame exported (T).
> - On-device dlopen + dlsym via dlopen_smoke.c: JNI_SYMBOL_OK.
> - APK-context System.loadLibrary: PROVEN via the minimal APK (bindings/android/app/) - hashFrame called in an app process, HASH matches Mac (2026-10-10).

## What this is

`rise_jni.c` exposes one JNI entry point:

```c
JNIEXPORT jstring JNICALL
Java_com_rise_core_RiseCore_hashFrame(JNIEnv *env, jclass clazz, jbyteArray data);
```

Kotlin side:

```kotlin
package com.rise.core

object RiseCore {
    init {
        System.loadLibrary("rise_jni")
    }

    external fun hashFrame(frame: ByteArray): String
}
```

Behavior:

- Input: `ByteArray` of raw frame bytes (same bytes `RiseCapture.RawFrame.yuvBytes` produces).
- Output: 64-char lowercase hex SHA256 string from core.
- Empty array is valid (yields the known empty SHA256).
- `null` input returns `null` (no exception thrown by us; OOM propagates).
- Input array is never modified (`ReleaseByteArrayElements` with `JNI_ABORT`).
- Nothing is logged. Never add `__android_log_print` of frame bytes, hashes are fine to return but pointless to log.

## Layout

```text
bindings/android/jni/
  rise_jni.c     # JNI wrapper, includes ../../macos/capture/core.h
  CMakeLists.txt # builds librise_jni.so, links prebuilt librise_core.a per ABI
  README.md      # this file
  libs/<ABI>/librise_core.a  # NOT checked in; staged by the core build step below
```

The relative include (`../../macos/capture/core.h`) is intentional: one FFI
contract shared with the proven macOS C harness, not a forked copy.

## Exact NDK build steps (PROVEN on this host, 2026-10-10; only arm64-v8a; ninja absent -> Unix Makefiles)

Prerequisites on the build machine (versions are minimums, adjust to current):

1. Android NDK r26+ with `ANDROID_NDK_HOME` set:
   ```sh
   echo "$ANDROID_NDK_HOME"
   ls "$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake"
   ```
2. Rust with Android targets + `cargo-ndk`:
   ```sh
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   cargo install cargo-ndk
   ```
3. CMake 3.22+ and Ninja:
   ```sh
   cmake --version
   ninja --version
   ```

Step A — build the Rust core per ABI (from repo root):

```sh
cd /path/to/.rise/core
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -- build --release
```

Step B — stage the static libs where CMake expects them:

```sh
# Repo root = /path/to/.rise
mkdir -p bindings/android/jni/libs/arm64-v8a \
         bindings/android/jni/libs/armeabi-v7a \
         bindings/android/jni/libs/x86_64
cp target/aarch64-linux-android/release/librise_core.a \
   bindings/android/jni/libs/arm64-v8a/
cp target/armv7-linux-androideabi/release/librise_core.a \
   bindings/android/jni/libs/armeabi-v7a/
cp target/x86_64-linux-android/release/librise_core.a \
   bindings/android/jni/libs/x86_64/
ls bindings/android/jni/libs/*/librise_core.a
```

Step C — configure + build the JNI lib per ABI:

```sh
cd bindings/android/jni
ABI=arm64-v8a
cmake -S . -B build/$ABI -G Ninja \
  -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
  -DANDROID_ABI=$ABI \
  -DANDROID_PLATFORM=android-24
cmake --build build/$ABI
ls build/$ABI/librise_jni.so
```

Repeat Step C for `armeabi-v7a` and `x86_64` as needed.

Step D — smoke check the ABI (on device/emulator, or with an x86_64 host harness):

```sh
# Expected: librise_jni.so exports exactly one Rise entry point
llvm-nm -D build/$ABI/librise_jni.so | grep RiseCore_hashFrame
# Expected output contains:
#   Java_com_rise_core_RiseCore_hashFrame
```

Step E — Kotlin call path (app module):

```kotlin
// RiseCapture produces RawFrame.yuvBytes; hand it straight to core.
val hex: String = RiseCore.hashFrame(rawFrame.yuvBytes)
// hex is 64 lowercase hex chars; compare against core sha256_frame on desktop.
```

## Honest verdict (this host, 2026-10-10)

- JNI C source: NDK-compiled (r27, aarch64-v8a) via CMake + android.toolchain.cmake.
- librise_core.a staged from cargo-ndk release build (staticlib crate-type added).
- llvm-nm -D: Java_com_rise_core_RiseCore_hashFrame exported (T).
- On-device dlopen + dlsym (dlopen_smoke.c): JNI_SYMBOL_OK.
- APK-context System.loadLibrary call: PROVEN (minimal APK, HASH matches Mac).
- Status: REAL at bridge and app-process level.
## Rules for future edits

- Thin shell only. Any signing, attestation verdict parsing, or badge logic belongs in `../attest/` or `verify-web/`, never here.
- Never log frame bytes, hashes passed for debugging only at verbose level, never key material (there is none in this file — keep it that way).
- Keep the relative `core.h` include. If the FFI signature changes, update macOS + Android + iOS together.
