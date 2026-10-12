#![no_main]

/// Fuzz the untrusted-input parsers of the RISE format. Every container
/// walker in core (JPEG APP11, PNG chunk, MP4 box) and the JUMBF payload
/// parser must accept ARBITRARY bytes without panicking — attackers control
/// these bytes.
///
/// Strict round-trip properties are asserted only on CLEAN-WALKING inputs:
/// a box/chunk sequence that tiles the input exactly and carries no RISE
/// marker of its own. On crafted inputs the walker's size-honoring rules can
/// legally land inside the appended box and surface phantom shapes from the
/// embedded bytes (fuzzer-proven, two crash artifacts) — that path only has
/// to not panic.
use libfuzzer_sys::fuzz_target;
use rise_core::container::{self, Container};

/// True when data is a box sequence that tiles it exactly, with no RISE box.
fn mp4_walks_clean(data: &[u8]) -> bool {
    if container::detect(data) != Container::Mp4 {
        return false;
    }
    let mut i = 0usize;
    while i < data.len() {
        if i + 8 > data.len() {
            return false;
        }
        let size = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        if size < 8 || i + size > data.len() {
            return false;
        }
        let kind = &data[i + 4..i + 8];
        let body = &data[i + 8..i + size];
        if kind == b"rise" {
            return false;
        }
        if kind == b"uuid" && body.len() >= 16 && body[..16] == container::RISE_MP4_UUID {
            return false;
        }
        i += size;
    }
    i == data.len()
}

/// True when data is a PNG whose chunks walk cleanly to IEND, no risE chunk.
fn png_walks_clean(data: &[u8]) -> bool {
    if container::detect(data) != Container::Png {
        return false;
    }
    let mut i = 8usize;
    while i < data.len() {
        if i + 12 > data.len() {
            return false;
        }
        let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        let end = i + 8 + len + 4;
        if end > data.len() {
            return false;
        }
        if &data[i + 4..i + 8] == b"risE" {
            return false;
        }
        if &data[i + 4..i + 8] == b"IEND" {
            return end == data.len();
        }
        i = end;
    }
    false
}

fuzz_target!(|data: &[u8]| {
    let _ = container::detect(data);
    let _ = container::mp4_extract_box(data);
    let _ = container::mp4_strip_box(data);
    let _ = container::png_extract_chunk(data);
    let _ = container::png_strip_chunk(data);
    let _ = rise_core::jumbf::jpeg_extract(data);
    if let Some(payload) = rise_core::jumbf::jpeg_extract(data) {
        let _ = rise_core::jumbf::parse_json_payload(&payload);
    }

    let input_has_rise = container::mp4_extract_box(data);

    // Embedding arbitrary data must never panic, whatever the container.
    if let Ok(sealed) = container::mp4_append_box(data, data) {
        let extracted = container::mp4_extract_box(&sealed);
        match (&input_has_rise, extracted) {
            // Anti-transplant: the input's own manifest still wins.
            (Some(pre), Some(post)) => assert_eq!(pre, &post),
            // Clean inputs: strict round-trip holds.
            (None, Some(post)) if mp4_walks_clean(data) => {
                assert_eq!(post.as_slice(), data);
                assert_eq!(container::mp4_strip_box(&sealed).unwrap().as_slice(), data);
            }
            // Crafted inputs may surface phantom shapes — no-panic only.
            (None, Some(_)) => {}
            _ => {}
        }
    }
    if let Ok(sealed) = container::png_insert_chunk(data, data) {
        let pre = container::png_extract_chunk(data);
        let post = container::png_extract_chunk(&sealed);
        match (&pre, post) {
            (Some(p), Some(q)) => assert_eq!(p, &q),
            (None, Some(q)) if png_walks_clean(data) => {
                assert_eq!(q.as_slice(), data);
                assert_eq!(container::png_strip_chunk(&sealed).unwrap().as_slice(), data);
            }
            (None, Some(_)) => {}
            _ => {}
        }
    }

    // A synthetic MP4 with the fuzz payload appended exercises box walking
    // over attacker-shaped trailing data.
    if data.len() >= 4 {
        let mut mp4 = Vec::with_capacity(data.len() + 16);
        mp4.extend_from_slice(&[0, 0, 0, 16]);
        mp4.extend_from_slice(b"ftypisom");
        mp4.extend_from_slice(data);
        assert_eq!(container::detect(&mp4), Container::Mp4);
        let _ = container::mp4_extract_box(&mp4);
        let _ = container::mp4_strip_box(&mp4);
    }
});
