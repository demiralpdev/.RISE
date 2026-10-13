#![no_main]

/// Thin wrapper: all parser logic and invariants live in the shared
/// `rise_core::fuzz_oracle` so the CI corpus-replay test (stable toolchain,
/// no sanitizer) exercises exactly what ASan fuzzing explores.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    rise_core::fuzz_oracle::fuzz_parsers(data);
});
