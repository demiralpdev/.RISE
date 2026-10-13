//! CI regression: replays the committed fuzz seed corpus through the same
//! oracle the ASan fuzzer explores. Any parser regression on a known-
//! adversarial input fails the build deterministically on a stable
//! toolchain (the ubuntu nightly + ASan combination is too flaky for CI;
//! full sanitizer fuzzing stays a local command: cargo +nightly fuzz run parsers).

use rise_core::fuzz_oracle::fuzz_parsers;

#[test]
fn replays_fuzz_seed_corpus() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/parsers");
    let mut count = 0usize;
    for entry in std::fs::read_dir(&dir).expect("seed corpus dir") {
        let path = entry.expect("dir entry").path();
        let bytes = std::fs::read(&path).expect("read seed");
        fuzz_parsers(&bytes);
        count += 1;
    }
    assert!(count >= 100, "suspiciously small corpus: {count} seeds");
}
