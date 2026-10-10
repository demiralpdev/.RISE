# iOS XCFramework Bridge (`bindings/ios/XCFRAMEWORK.md`)

> Status: **REAL** — built and proven on this host (2026-10-10, Xcode 27.0).
> The static verifier story moved past the old STUB state: Rust core built for
> both iOS targets, XCFramework assembled, and the core hashed bytes INSIDE an
> iOS simulator process with a byte-for-byte match against the desktop.
> Thin-shell rule unchanged: the bridge only moves frame bytes into core
> `rise_hash_frame`. No signing logic, no key handling, no key material
> logging. Signing lives in Secure Enclave / `attest/Attest.swift`.

## FFI contract (one source of truth)

`bindings/macos/capture/core.h`:
`void rise_hash_frame(const unsigned char *bytes, size_t len, char *out_hex65)`
— implemented in `core/src/lib.rs`. The iOS bridge uses that exact signature.

## What ran on this host (real outputs, 2026-10-10)

1. **Xcode 27.0 (Build 27A266a)** installed; iOS SDKs 27.0 present
   (`xcrun --sdk iphoneos --show-sdk-version` -> 27.0). `xcode-select` still
   points at CommandLineTools, so EVERY xcrun/xcodebuild call uses
   `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` — no sudo needed.
2. **rustup + iOS targets**: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`.
3. **Rust core built for both targets**:
   ```sh
   export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
   export SDKROOT=$(xcrun --sdk iphonesimulator --show-sdk-path)   # REQUIRED: ring's build script probes the SDK
   cargo build --release --target aarch64-apple-ios-sim
   export SDKROOT=$(xcrun --sdk iphoneos --show-sdk-path)
   cargo build --release --target aarch64-apple-ios
   ```
   Output: `librise_core.a` 27.2MB per target. Gotcha: without `SDKROOT`,
   `ring`'s build script fails with "SDK iphonesimulator cannot be located".
4. **XCFramework assembled**:
   ```sh
   xcodebuild -create-xcframework \
     -library core/target/aarch64-apple-ios-sim/release/librise_core.a -headers bindings/ios/include/ \
     -library core/target/aarch64-apple-ios/release/librise_core.a -headers bindings/ios/include/ \
     -output /tmp/RiseCore.xcframework
   ```
   -> `Info.plist`, `ios-arm64/`, `ios-arm64-simulator/` slices. `bindings/ios/include/` holds
   `core.h` + `module.modulemap` (`module RiseCore { header "core.h" export * }`).
5. **Simulator RUN proof** (beyond the old plan's typecheck-only scope):
   - `bindings/ios/ios_harness.swift` — hashes `"rise-ios-sim-proof-001"` via
     `rise_hash_frame` and prints `IOS_HASH=<hex>`.
   - `xcrun --sdk iphonesimulator swiftc -target arm64-apple-ios16.0-simulator
     -sdk "$SDK" -import-objc-header bindings/macos/capture/core.h
     bindings/ios/ios_harness.swift librise_core.a -o /tmp/ios_harness`
     (ld warnings about the lib targeting iOS-sim 27.0 vs the harness's 16.0 are
     cosmetic).
   - `xcrun simctl boot "iPhone 17e"` (iOS 27.0 runtime, installed with Xcode 27).
   - `xcrun simctl spawn booted /tmp/ios_harness` -> **`IOS_HASH: 2f7359beb7f6cdce462bfcf431b2dd5f3bee7b1e448ab335ba202443df6de267`**
   - Desktop `rise-core hash` over the same bytes -> **identical**. Byte-for-byte
     cross-platform proof: macOS, Windows, Linux, Android, iOS simulator.

## Honest remaining gaps

- **Physical iPhone run**: pending a device + Apple ID (free 7-day provisioning
  is enough for testing). The simulator proves linkage + execution; real
  Secure Enclave attestation (`attest/Attest.swift`) needs the device.
- `xcode-select` still points at CommandLineTools (no sudo used); every
  Xcode call on this host needs `DEVELOPER_DIR`. Set
  `sudo xcode-select -s /Applications/Xcode.app` on an interactive session to
  make it permanent.
- ld version warnings (lib built for iOS-sim 27.0, harness targets 16.0) are
  cosmetic; raise the harness deployment target to silence them.

## Rules for future edits

- English everywhere in this bridge story.
- Thin shell only: frame bytes in, hex string out. No signing, no keychain,
  no DeviceCheck calls in the bridge file.
- One FFI contract: if `rise_hash_frame` changes, update `core.h`, the JNI
  wrapper, and the iOS header together. Never fork the signature per platform.
- No fake success claims. If a step cannot run on the proving host, say so
  with the failing command + output.
