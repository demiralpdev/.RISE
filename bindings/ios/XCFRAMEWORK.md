# iOS XCFramework Bridge (`bindings/ios/XCFRAMEWORK.md`)

> STUB HONESTY HEADER — NOT BUILT, NOT RUN (2026-10-10, macOS host).
> - No iOS SDK on this machine, so no static-lib + `swiftc -emit-module`
>   proof has run. This document is a plan + honest status, not a result.
> - Thin-shell rule: the future bridge only moves frame bytes into core
>   `rise_hash_frame`. No signing logic, no key handling, no key material
>   logging. Signing lives in Secure Enclave / `attest/Attest.swift`, never
>   in the bridge.
> - FFI contract (verified by reading on disk): `bindings/macos/capture/core.h`
>   declares `void rise_hash_frame(const unsigned char *bytes, size_t len,
>   char *out_hex65)`; `core/src/lib.rs` implements it. The iOS bridge must
>   use that exact signature, not a fork.

## 1. SDK check (actually run here, 2026-10-10)

Commands:

```sh
xcrun --sdk iphoneos --show-sdk-path
xcrun --sdk iphonesimulator --show-sdk-path
xcode-select -p
xcrun --show-sdk-path
ls /Library/Developer/CommandLineTools/SDKs/
swiftc --version
```

Results:

```text
xcrun --sdk iphoneos --show-sdk-path
  -> xcrun: error: SDK "iphoneos" cannot be located
xcrun --sdk iphonesimulator --show-sdk-path
  -> xcrun: error: SDK "iphonesimulator" cannot be located
xcode-select -p
  -> /Library/Developer/CommandLineTools
xcrun --show-sdk-path
  -> /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
ls /Library/Developer/CommandLineTools/SDKs/
  -> MacOSX.sdk, MacOSX26.5.sdk, MacOSX26.sdk, MacOSX27.0.sdk, MacOSX27.sdk
swiftc --version
  -> swift-driver 1.168.6, Apple Swift 6.4, Target arm64-apple-macosx27.0.0
```

Verdict: **iOS SDK ABSENT.** Only CommandLineTools + MacOSX SDKs are
installed. No `/Applications/Xcode*.app`, no `iPhoneOS.sdk`, no
`iPhoneSimulator.sdk`. `swiftc` can only target macOS here.

## 2. What is missing (exactly)

1. **Full Xcode** (not just CommandLineTools). Provides `iPhoneOS.sdk`,
   `iPhoneSimulator.sdk`, `xcodebuild -create-xcframework`, and the iOS
   Swift stdlib overlays. Install from the App Store or Apple Developer,
   then `sudo xcode-select -s /Applications/Xcode.app/Contents/Developer`.
2. **Rust iOS targets** via rustup (`aarch64-apple-ios`,
   `aarch64-apple-ios-sim`). This host has a Homebrew `rustc` with no
   `rustup`, so `rustup target add` is unavailable. A proving host needs
   rustup + both targets.
3. **A physical iPhone + Apple Developer account** for the real App Attest
   path (`attest/Attest.swift` already notes this). The simulator can prove
   linkage but never attestation.

Because (1) is missing, the proof in section 3 was **not attempted**.
Per task constraints, no Xcode download was attempted either.

## 3. Proof that was NOT run (steps for an Xcode host)

On a machine where `xcrun --sdk iphoneos --show-sdk-path` prints a real
`iPhoneOS.sdk` path, the owner should run exactly this, with no uniffi,
no cbindgen, no code generation — plain `staticlib` + modulemap:

```sh
# 0. Confirm the SDK exists (must print a path, not an error).
xcrun --sdk iphoneos --show-sdk-path
xcrun --sdk iphonesimulator --show-sdk-path

# 1. Build the Rust core as a static lib for device + simulator.
cd /path/to/.rise/core
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
ls target/aarch64-apple-ios/release/librise_core.a
ls target/aarch64-apple-ios-sim/release/librise_core.a

# 2. Expose the C header (reuse the single contract, do not fork it).
# bindings/ios/include/rise_core.h -> copy of, or umbrella pointing at,
# ../../macos/capture/core.h content. Plus module.modulemap:
#   module RiseCore { header "rise_core.h" export * }

# 3. Swift import proof against each SDK (typecheck only, no device needed).
swiftc -typecheck -import-objc-header bindings/ios/include/rise_core.h \
  -sdk "$(xcrun --sdk iphoneos --show-sdk-path)" \
  bindings/ios/capture/Capture.swift
swiftc -typecheck -import-objc-header bindings/ios/include/rise_core.h \
  -sdk "$(xcrun --sdk iphonesimulator --show-sdk-path)" \
  bindings/ios/capture/Capture.swift

# 4. Assemble the XCFramework (device + simulator slices).
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/librise_core.a \
    -headers bindings/ios/include/ \
  -library target/aarch64-apple-ios-sim/release/librise_core.a \
    -headers bindings/ios/include/ \
  -output bindings/ios/RiseCore.xcframework
ls bindings/ios/RiseCore.xcframework/Info.plist

# 5. Emit-module proof (Swift sees the C function).
swiftc -emit-module -module-name RiseCoreCheck \
  -import-objc-header bindings/ios/include/rise_core.h \
  -sdk "$(xcrun --sdk iphoneos --show-sdk-path)" \
  -o /tmp/RiseCoreCheck.swiftmodule \
  bindings/ios/capture/Capture.swift
echo "emit-module exit: $?"
```

Success criteria (all required before dropping the STUB header):

- Both `librise_core.a` files exist and `lipo -info` shows `arm64` each.
- Both `swiftc -typecheck` invocations exit 0.
- `RiseCore.xcframework/Info.plist` exists with two `AvailableLibraries`.
- `swiftc -emit-module` exits 0.
- A one-frame hash from Swift (`rise_hash_frame` via the imported header)
  matches desktop `sha256_frame` for the same bytes (compare against
  `core/tests/ffi_e2e.sh` expected vectors).

None of the above has run. Status stays **STUB**.

## 4. Honest verdict per platform (this machine)

| Platform | Bridge artifact | Verdict here |
|---|---|---|
| Android JNI | `bindings/android/jni/rise_jni.c` + `CMakeLists.txt` + `README.md` | STUB, uncompiled. Source grounded in `core.h`/`lib.rs`; no NDK, no build. |
| iOS XCFramework | this file only (no `.xcframework`, no `include/`, no built `.a`) | STUB, unproven. iOS SDK absent; proof steps documented but not run. |
| macOS (reference) | `core/tests/ffi_e2e.sh` C harness | Proven elsewhere (green per task context); not re-run in this task. |

## 5. Rules for future edits

- English everywhere in this bridge story.
- Keep the STUB header until the section-3 proof is pasted with real output.
- Thin shell only: frame bytes in, hex string out. No signing, no keychain,
  no DeviceCheck calls in the bridge file.
- One FFI contract: if `rise_hash_frame` changes, update `core.h`, the JNI
  wrapper, and the iOS header together. Never fork the signature per platform.
- No fake success claims. If it cannot build on the proving host, say so
  here with the failing command + output.
