//! RISE container embedding (RISE-01 §2): ONE self-contained file per media
//! type — the proof lives inside the media, never in a sidecar.
//!
//! | Media | Container mechanism | Compatibility |
//! |---|---|---|
//! | JPEG | APP11 JUMBF segment (see `jumbf`) | C2PA 2.4, c2patool-validated |
//! | PNG | ancillary `risE` chunk, inserted after IHDR | decoders skip unknown chunks |
//! | MP4 | top-level `rise` box, appended at EOF | players skip unknown boxes |
//!
//! Extraction rule: the FIRST embedded manifest wins. A second `rise` box or
//! `risE` chunk appended by an attacker is ignored; removing the original one
//! leaves the file with no manifest (fail-closed).

/// Detected media container of a byte stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// JPEG (SOI magic `FF D8 FF`).
    Jpeg,
    /// PNG (signature `89 50 4E 47 0D 0A 1A 0A`).
    Png,
    /// ISO BMFF / MP4 (`ftyp` box at offset 4).
    Mp4,
    /// Anything else.
    Unknown,
}

/// Detects the container by magic bytes.
pub fn detect(data: &[u8]) -> Container {
    if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        Container::Jpeg
    } else if data.len() >= 8 && data[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        Container::Png
    } else if data.len() >= 8 && &data[4..8] == b"ftyp" {
        Container::Mp4
    } else {
        Container::Unknown
    }
}

// ---------------------------------------------------------------------------
// MP4 — top-level `uuid` box (C2PA convention) with a legacy `rise` fallback
// ---------------------------------------------------------------------------

/// RISE manifest box UUID (RFC 4122 layout, "rise"…​"RISE" byte marks).
/// Embedded in a top-level `uuid` box, matching how C2PA stores manifests
/// in MP4; strict BMFF scanners handle `uuid` universally.
pub const RISE_MP4_UUID: [u8; 16] = [
    0x72, 0x69, 0x73, 0x65, 0x00, 0x00, 0x40, 0x80, 0x00, 0x00, 0x00, 0x00, 0x52, 0x49, 0x53, 0x45,
];

/// Appends a top-level `uuid` box containing the manifest to an MP4.
/// The result is still a valid, playable MP4: players skip unknown uuid boxes.
pub fn mp4_append_box(mp4: &[u8], manifest: &[u8]) -> Result<Vec<u8>, String> {
    if detect(mp4) != Container::Mp4 {
        return Err("not an MP4 (no ftyp box)".to_string());
    }
    let payload_len = manifest.len() + RISE_MP4_UUID.len();
    if payload_len + 8 > u32::MAX as usize {
        return Err("manifest too large for a single box".to_string());
    }
    let box_len = (payload_len + 8) as u32;
    let mut out = Vec::with_capacity(mp4.len() + box_len as usize);
    out.extend_from_slice(mp4);
    out.extend_from_slice(&box_len.to_be_bytes());
    out.extend_from_slice(b"uuid");
    out.extend_from_slice(&RISE_MP4_UUID);
    out.extend_from_slice(manifest);
    Ok(out)
}

/// A RISE manifest box is either the `uuid` form carrying OUR UUID, or the
/// legacy `rise` 4CC written by v0.2.2/0.2.3. Foreign uuid boxes are not
/// RISE manifests and never match. Returns the manifest payload slice.
fn split_rise_box<'a>(kind: &[u8], body: &'a [u8]) -> Option<&'a [u8]> {
    if kind == b"rise" {
        return Some(body);
    }
    if kind == b"uuid" && body.len() >= 16 && body[..16] == RISE_MP4_UUID {
        return Some(&body[16..]);
    }
    None
}

/// Walks top-level boxes and returns the FIRST RISE manifest payload
/// (uuid-with-our-UUID or legacy `rise` box). Malformed boxes terminate
/// the walk (never panics).
pub fn mp4_extract_box(data: &[u8]) -> Option<Vec<u8>> {
    find_rise_box(data).map(|(_, payload)| payload.to_vec())
}

/// Removes the FIRST RISE box, reproducing the pre-seal byte stream
/// (used for hash verification of the original media).
pub fn mp4_strip_box(data: &[u8]) -> Option<Vec<u8>> {
    let (range, _) = find_rise_box(data)?;
    let mut out = Vec::with_capacity(data.len() - range.len());
    out.extend_from_slice(&data[..range.start]);
    out.extend_from_slice(&data[range.end..]);
    Some(out)
}

/// Locates the first RISE box: returns (byte range of the whole box,
/// manifest payload slice). Handles size==0 and 64-bit sizes safely.
fn find_rise_box(data: &[u8]) -> Option<(std::ops::Range<usize>, &[u8])> {
    let mut i = 0usize;
    while i + 8 <= data.len() {
        let size = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        let kind = &data[i + 4..i + 8];
        if size == 0 {
            break; // box extends to EOF and it is not ours
        }
        if size == 1 {
            // 64-bit largesize box: skip via the extended size. Checked
            // BEFORE size < 8 (a 1-sized head is valid here — the real
            // length lives in the 8-byte largesize that follows).
            if i + 16 > data.len() {
                break;
            }
            let mut b8 = [0u8; 8];
            b8.copy_from_slice(&data[i + 8..i + 16]);
            let large = u64::from_be_bytes(b8) as usize;
            if large < 16 || large > data.len() - i {
                break;
            }
            i += large;
            continue;
        }
        if size < 8 {
            break; // malformed; stop walking
        }
        if size > data.len() - i {
            break; // declared box runs past EOF; stop
        }
        let body = &data[i + 8..i + size];
        if let Some(payload) = split_rise_box(kind, body) {
            return Some((i..i + size, payload));
        }
        i += size;
    }
    None
}

// ---------------------------------------------------------------------------
// PNG — ancillary `risE` chunk
// ---------------------------------------------------------------------------

const PNG_SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// CRC-32 (IEEE 802.3, reflected, poly 0xEDB88320) — PNG chunk checksums.
/// Hand-rolled to avoid a new dependency; test vectors pin correctness.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Inserts the manifest as an ancillary `risE` chunk immediately before the
/// first IDAT chunk (the canonical ancillary slot). The result is still a
/// valid, viewable PNG: decoders ignore unknown ancillary chunks.
/// First-inserted seal stays first in file order, so later inserts
/// (including attacker transplants) never displace it.
pub fn png_insert_chunk(png: &[u8], manifest: &[u8]) -> Result<Vec<u8>, String> {
    if png.len() < 8 || png[..8] != PNG_SIG {
        return Err("not a PNG (bad signature)".to_string());
    }
    if manifest.len() > u32::MAX as usize {
        return Err("manifest too large for a chunk".to_string());
    }
    // PNG spec: the first chunk MUST be a 13-byte IHDR.
    if png.len() < 8 + 12 + 13 || &png[12..16] != b"IHDR" {
        return Err("malformed PNG (IHDR not first chunk)".to_string());
    }
    let mut i = 8usize;
    loop {
        if i + 8 > png.len() {
            return Err("malformed PNG (no IDAT chunk)".to_string());
        }
        let len = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
        let end = i + 8 + len;
        if end + 4 > png.len() {
            return Err("malformed PNG (truncated chunk)".to_string());
        }
        if &png[i + 4..i + 8] == b"IDAT" {
            break;
        }
        i = end + 4;
    }
    let mut chunk = Vec::with_capacity(manifest.len() + 12);
    chunk.extend_from_slice(&(manifest.len() as u32).to_be_bytes());
    chunk.extend_from_slice(b"risE");
    chunk.extend_from_slice(manifest);
    let mut crc_input = Vec::with_capacity(manifest.len() + 4);
    crc_input.extend_from_slice(b"risE");
    crc_input.extend_from_slice(manifest);
    chunk.extend_from_slice(&crc32(&crc_input).to_be_bytes());

    let mut out = Vec::with_capacity(png.len() + chunk.len());
    out.extend_from_slice(&png[..i]);
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&png[i..]);
    Ok(out)
}

/// Walks chunks until IEND and returns the FIRST valid `risE` chunk data.
/// Corrupted chunks (CRC mismatch) are skipped, not trusted.
pub fn png_extract_chunk(data: &[u8]) -> Option<Vec<u8>> {
    png_find_chunk(data)
}

fn png_find_chunk(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 8 || data[..8] != PNG_SIG {
        return None;
    }
    let mut i = 8usize;
    while i + 8 <= data.len() {
        let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        let end = i + 8 + len;
        if end + 4 > data.len() {
            return None; // truncated chunk
        }
        let kind = &data[i + 4..i + 8];
        if kind == b"risE" {
            let mut crc_input = Vec::with_capacity(len + 4);
            crc_input.extend_from_slice(kind);
            crc_input.extend_from_slice(&data[i + 8..end]);
            let stored =
                u32::from_be_bytes([data[end], data[end + 1], data[end + 2], data[end + 3]]);
            if crc32(&crc_input) != stored {
                return None; // corrupted proof chunk: fail closed
            }
            return Some(data[i + 8..end].to_vec());
        }
        if kind == b"IEND" {
            return None;
        }
        i = end + 4;
    }
    None
}

/// Removes the FIRST `risE` chunk, reproducing the pre-seal byte stream
/// (used for hash verification of the original image).
pub fn png_strip_chunk(data: &[u8]) -> Option<Vec<u8>> {
    png_strip_chunk_internal(data).map(|(_, stream)| stream)
}

fn png_strip_chunk_internal(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if data.len() < 8 || data[..8] != PNG_SIG {
        return None;
    }
    let mut i = 8usize;
    while i + 8 <= data.len() {
        let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        let end = i + 8 + len;
        if end + 4 > data.len() {
            return None;
        }
        let kind = &data[i + 4..i + 8];
        if kind == b"risE" {
            let chunk = data[i..end + 4].to_vec();
            let mut out = Vec::with_capacity(data.len() - chunk.len());
            out.extend_from_slice(&data[..i]);
            out.extend_from_slice(&data[end + 4..]);
            return Some((chunk, out));
        }
        if kind == b"IEND" {
            return None;
        }
        i = end + 4;
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn crc_input(kind: &[u8], data: &[u8]) -> u32 {
        let mut v = Vec::with_capacity(kind.len() + data.len());
        v.extend_from_slice(kind);
        v.extend_from_slice(data);
        crc32(&v)
    }

    #[test]
    fn crc32_known_vectors() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0x0000_0000);
    }

    #[test]
    fn detect_all_magics() {
        assert_eq!(detect(&[0xFF, 0xD8, 0xFF, 0xE0]), Container::Jpeg);
        assert_eq!(detect(&PNG_SIG), Container::Png);
        let mut mp4 = vec![0, 0, 0, 24];
        mp4.extend_from_slice(b"ftypisom");
        assert_eq!(detect(&mp4), Container::Mp4);
        assert_eq!(detect(b"RISE"), Container::Unknown);
    }

    // --- MP4 ---------------------------------------------------------------

    fn synth_mp4() -> Vec<u8> {
        let mut ftyp = b"ftypisom\x00\x00\x02\x00isomiso2".to_vec(); // 16B payload
        let mut mdat = b"mdatfake-video-bytes".to_vec(); // 16B payload
        let mut out = Vec::new();
        out.extend_from_slice(&((16 + 8) as u32).to_be_bytes());
        out.append(&mut ftyp);
        out.extend_from_slice(&((16 + 8) as u32).to_be_bytes());
        out.append(&mut mdat);
        out
    }

    #[test]
    fn mp4_roundtrip() {
        let mp4 = synth_mp4();
        let sealed = mp4_append_box(&mp4, b"{\"rise_version\":1}").unwrap();
        // sealed keeps the original prefix byte-identical
        assert_eq!(&sealed[..mp4.len()], &mp4[..]);
        assert_eq!(mp4_extract_box(&sealed).unwrap(), b"{\"rise_version\":1}");
        // the embedded box is the uuid form with our UUID
        let tail = &sealed[mp4.len()..];
        assert_eq!(&tail[4..8], b"uuid");
        assert_eq!(&tail[8..24], &RISE_MP4_UUID);
        // strip reproduces the original stream exactly
        assert_eq!(mp4_strip_box(&sealed).unwrap(), mp4);
    }

    #[test]
    fn mp4_legacy_rise_box_still_reads() {
        // v0.2.2/0.2.3 wrote a plain `rise` 4CC box; those files must verify.
        let mp4 = synth_mp4();
        let mut sealed = mp4.clone();
        sealed.extend_from_slice(&((b"legacy".len() + 8) as u32).to_be_bytes());
        sealed.extend_from_slice(b"rise");
        sealed.extend_from_slice(b"legacy");
        assert_eq!(mp4_extract_box(&sealed).unwrap(), b"legacy");
        assert_eq!(mp4_strip_box(&sealed).unwrap(), mp4);
    }

    #[test]
    fn mp4_foreign_uuid_ignored() {
        // an attacker-supplied uuid box (different UUID) never matches
        let mp4 = synth_mp4();
        let mut data = mp4.clone();
        let foreign_body: &[u8] = &[0u8; 16]; // foreign UUID
        let junk: &[u8] = b"not-rise";
        data.extend_from_slice(&((foreign_body.len() + junk.len() + 8) as u32).to_be_bytes());
        data.extend_from_slice(b"uuid");
        data.extend_from_slice(foreign_body);
        data.extend_from_slice(junk);
        assert!(mp4_extract_box(&data).is_none());
        assert!(mp4_strip_box(&data).is_none());
        // our box after a foreign one still wins
        let sealed = mp4_append_box(&data, b"ours").unwrap();
        assert_eq!(mp4_extract_box(&sealed).unwrap(), b"ours");
    }

    #[test]
    fn mp4_first_box_wins() {
        let sealed = mp4_append_box(&synth_mp4(), b"first").unwrap();
        let resealed = mp4_append_box(&sealed, b"second").unwrap();
        assert_eq!(mp4_extract_box(&resealed).unwrap(), b"first");
    }

    #[test]
    fn mp4_no_box_is_none() {
        assert!(mp4_extract_box(&synth_mp4()).is_none());
        assert!(mp4_strip_box(&synth_mp4()).is_none());
    }

    fn mp4_legacy_sealed(prefix: &[u8], payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut sealed = prefix.to_vec();
        sealed.extend_from_slice(&((payload.len() + 8) as u32).to_be_bytes());
        sealed.extend_from_slice(b"rise");
        sealed.extend_from_slice(payload);
        (prefix.to_vec(), sealed)
    }

    #[test]
    fn mp4_empty_size8_box_skipped() {
        // attacker sends an empty box (size 8, no body) ahead of the proof box
        let mut prefix = synth_mp4();
        prefix.extend_from_slice(&8u32.to_be_bytes());
        prefix.extend_from_slice(b"free");
        let (expected, sealed) = mp4_legacy_sealed(&prefix, b"proof");
        assert_eq!(mp4_extract_box(&sealed).unwrap(), b"proof");
        assert_eq!(mp4_strip_box(&sealed).unwrap(), expected);
    }

    // BUG(mutants): `find_rise_box` checks `size < 8` before `size == 1`, so a
    // 64-bit largesize box always terminates the walk instead of being skipped
    // as the doc comment claims. Fail-closed today (returns None), but the
    // A valid 64-bit largesize box must be SKIPPED via its extended size,
    // letting the walk reach the proof box that follows.
    #[test]
    fn mp4_largesize_box_is_skipped() {
        // 64-bit largesize box (size == 1) ahead of the proof box
        let mut prefix = Vec::new();
        prefix.extend_from_slice(&1u32.to_be_bytes());
        prefix.extend_from_slice(b"free");
        prefix.extend_from_slice(&24u64.to_be_bytes());
        prefix.extend_from_slice(&[0u8; 8]);
        prefix.extend_from_slice(&synth_mp4());
        let (mp4, sealed) = mp4_legacy_sealed(&prefix, b"proof");
        assert_eq!(mp4_extract_box(&sealed).unwrap(), b"proof");
        assert_eq!(mp4_strip_box(&sealed).unwrap(), mp4);
    }

    // The smallest legal largesize (16: header only) is skipped too.
    #[test]
    fn mp4_largesize_exact16_box_is_skipped() {
        let mut prefix = Vec::new();
        prefix.extend_from_slice(&1u32.to_be_bytes());
        prefix.extend_from_slice(b"free");
        prefix.extend_from_slice(&16u64.to_be_bytes());
        let (mp4, sealed) = mp4_legacy_sealed(&prefix, b"proof");
        assert_eq!(mp4_extract_box(&sealed).unwrap(), b"proof");
        assert_eq!(mp4_strip_box(&sealed).unwrap(), mp4);
    }

    #[test]
    fn mp4_largesize_truncated_header_stops_walk() {
        // size == 1 at offset 0 but fewer than 16 bytes total
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_be_bytes());
        data.extend_from_slice(b"free");
        data.extend_from_slice(&[0u8; 2]);
        assert!(mp4_extract_box(&data).is_none());
        assert!(mp4_strip_box(&data).is_none());
    }

    #[test]
    fn mp4_tiny_largesize_value_stops_walk() {
        // largesize below 16 is malformed even when it fits in the file
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_be_bytes());
        data.extend_from_slice(b"free");
        data.extend_from_slice(&12u64.to_be_bytes());
        data.extend_from_slice(b"rise");
        data.extend_from_slice(b"junk");
        assert_eq!(data.len(), 24);
        assert!(mp4_extract_box(&data).is_none());
        assert!(mp4_strip_box(&data).is_none());
    }

    #[test]
    fn mp4_big_box_size_second_byte_matters() {
        // box size above 64 KiB: the second size byte is non-zero
        let mut prefix = synth_mp4();
        let big = 70_000usize;
        prefix.extend_from_slice(&(big as u32).to_be_bytes());
        prefix.extend_from_slice(b"free");
        let filler = vec![0u8; big - 8];
        prefix.extend_from_slice(&filler);
        let (expected, sealed) = mp4_legacy_sealed(&prefix, b"proof");
        assert_eq!(mp4_extract_box(&sealed).unwrap(), b"proof");
        assert_eq!(mp4_strip_box(&sealed).unwrap(), expected);
    }

    #[test]
    fn mp4_truncated_box_stops_walk() {
        let mut sealed = mp4_append_box(&synth_mp4(), b"proof").unwrap();
        // declare a size that runs past EOF -> walk stops, no panic, None
        sealed.truncate(sealed.len() - 2);
        assert!(mp4_extract_box(&sealed).is_none());
    }

    #[test]
    fn mp4_zero_size_box_stops_walk() {
        let mut data = synth_mp4();
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(b"rise");
        data.extend_from_slice(b"fake");
        assert!(mp4_extract_box(&data).is_none());
    }

    #[test]
    fn mp4_append_rejects_non_mp4() {
        assert!(mp4_append_box(b"not-mp4", b"m").is_err());
    }

    // --- PNG ---------------------------------------------------------------

    fn synth_png() -> Vec<u8> {
        let mut out = PNG_SIG.to_vec();
        // IHDR: len 13, type, 13 zero bytes, CRC (bogus CRC fine for walker,
        // but compute a real one for cleanliness)
        let mut ihdr_data = [0u8; 13];
        ihdr_data[0] = 0;
        ihdr_data[1] = 1; // width 1
        ihdr_data[2] = 0;
        ihdr_data[3] = 1; // height 1
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&13u32.to_be_bytes());
        ihdr.extend_from_slice(b"IHDR");
        ihdr.extend_from_slice(&ihdr_data);
        ihdr.extend_from_slice(&crc_input(b"IHDR", &ihdr_data).to_be_bytes());
        out.extend_from_slice(&ihdr);
        // IDAT: 4 bytes
        let mut idat = Vec::new();
        idat.extend_from_slice(&4u32.to_be_bytes());
        idat.extend_from_slice(b"IDAT");
        idat.extend_from_slice(b"\x00\x00\x00\x00");
        idat.extend_from_slice(&crc_input(b"IDAT", b"\x00\x00\x00\x00").to_be_bytes());
        out.extend_from_slice(&idat);
        // IEND: 0 bytes
        let mut iend = Vec::new();
        iend.extend_from_slice(&0u32.to_be_bytes());
        iend.extend_from_slice(b"IEND");
        iend.extend_from_slice(&crc_input(b"IEND", b"").to_be_bytes());
        out.extend_from_slice(&iend);
        out
    }

    #[test]
    fn png_roundtrip() {
        let png = synth_png();
        let sealed = png_insert_chunk(&png, b"{\"rise_version\":1}").unwrap();
        // original bytes preserved in order (chunk inserted after IHDR)
        assert_eq!(png_extract_chunk(&sealed).unwrap(), b"{\"rise_version\":1}");
        assert_eq!(png_strip_chunk(&sealed).unwrap(), png);
        // proof chunk sits after IHDR, before IDAT
        let ihdr_pos = sealed.windows(4).position(|w| w == b"IHDR").unwrap();
        let rise_pos = sealed.windows(4).position(|w| w == b"risE").unwrap();
        let idat_pos = sealed.windows(4).position(|w| w == b"IDAT").unwrap();
        assert!(ihdr_pos < rise_pos && rise_pos < idat_pos);
    }

    #[test]
    fn png_first_chunk_wins() {
        let sealed = png_insert_chunk(&synth_png(), b"first").unwrap();
        let resealed = png_insert_chunk(&sealed, b"second").unwrap();
        assert_eq!(png_extract_chunk(&resealed).unwrap(), b"first");
    }

    #[test]
    fn png_corrupted_crc_fails_closed() {
        let mut sealed = png_insert_chunk(&synth_png(), b"proof-manifest").unwrap();
        let pos = sealed.windows(4).position(|w| w == b"risE").unwrap() + 12;
        sealed[pos] ^= 0xFF; // flip a manifest byte
        assert!(png_extract_chunk(&sealed).is_none());
    }

    #[test]
    fn png_insert_rejects_non_png() {
        assert!(png_insert_chunk(b"not-png", b"m").is_err());
        // valid sig but no IHDR
        let mut bad = PNG_SIG.to_vec();
        bad.extend_from_slice(&4u32.to_be_bytes());
        bad.extend_from_slice(b"IDAT");
        bad.extend_from_slice(b"junk");
        bad.extend_from_slice(&[0; 4]);
        assert!(png_insert_chunk(&bad, b"m").is_err());
    }

    #[test]
    fn png_no_chunk_is_none() {
        assert!(png_extract_chunk(&synth_png()).is_none());
        assert!(png_strip_chunk(&synth_png()).is_none());
    }

    fn png_test_ihdr() -> Vec<u8> {
        let mut v = PNG_SIG.to_vec();
        let d = [0u8; 13];
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&d);
        v.extend_from_slice(&crc_input(b"IHDR", &d).to_be_bytes());
        v
    }

    fn png_test_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        v.extend_from_slice(kind);
        v.extend_from_slice(payload);
        v.extend_from_slice(&crc_input(kind, payload).to_be_bytes());
        v
    }

    fn png_test_big_filler() -> Vec<u8> {
        vec![0xFFu8; 70_000]
    }

    #[test]
    fn png_insert_sig_only_reports_ihdr() {
        // exactly the 8-byte signature: passes the length guard, fails IHDR check
        assert_eq!(
            png_insert_chunk(&PNG_SIG, b"m").unwrap_err(),
            "malformed PNG (IHDR not first chunk)"
        );
        assert_eq!(
            png_insert_chunk(&PNG_SIG[..7], b"m").unwrap_err(),
            "not a PNG (bad signature)"
        );
    }

    #[test]
    fn png_insert_ihdr_only_reports_no_idat() {
        // 33 bytes: signature + complete IHDR, no room for IDAT
        let png = png_test_ihdr();
        assert_eq!(png.len(), 33);
        assert_eq!(
            png_insert_chunk(&png, b"m").unwrap_err(),
            "malformed PNG (no IDAT chunk)"
        );
    }

    #[test]
    fn png_insert_partial_chunk_reports_truncated() {
        // chunk header at EOF claims 4 data bytes that are not there
        let mut png = png_test_ihdr();
        png.extend_from_slice(&4u32.to_be_bytes());
        png.extend_from_slice(b"IDAT");
        assert_eq!(
            png_insert_chunk(&png, b"m").unwrap_err(),
            "malformed PNG (truncated chunk)"
        );
    }

    #[test]
    fn png_insert_idat_ending_at_exact_eof() {
        // IDAT chunk ends exactly at EOF (no IEND yet): still a valid slot
        let mut png = png_test_ihdr();
        png.extend_from_slice(&png_test_chunk(b"IDAT", b"\x00\x00\x00\x00"));
        let sealed = png_insert_chunk(&png, b"proof").unwrap();
        assert_eq!(png_extract_chunk(&sealed).unwrap(), b"proof");
        assert_eq!(png_strip_chunk(&sealed).unwrap(), png);
    }

    #[test]
    fn png_insert_skips_big_chunk_before_idat() {
        // chunk length above 64 KiB: the second length byte is non-zero
        let mut png = png_test_ihdr();
        png.extend_from_slice(&png_test_chunk(b"tEXt", &png_test_big_filler()));
        png.extend_from_slice(&png_test_chunk(b"IDAT", b"\x00\x00\x00\x00"));
        png.extend_from_slice(&png_test_chunk(b"IEND", b""));
        let sealed = png_insert_chunk(&png, b"proof").unwrap();
        assert_eq!(png_extract_chunk(&sealed).unwrap(), b"proof");
        assert_eq!(png_strip_chunk(&sealed).unwrap(), png);
        let ihdr_pos = sealed.windows(4).position(|w| w == b"IHDR").unwrap();
        let rise_pos = sealed.windows(4).position(|w| w == b"risE").unwrap();
        let idat_pos = sealed.windows(4).position(|w| w == b"IDAT").unwrap();
        assert!(ihdr_pos < rise_pos && rise_pos < idat_pos);
    }

    #[test]
    fn png_find_rise_chunk_at_exact_eof() {
        // risE chunk ends exactly at EOF with no IEND after it
        let mut png = png_test_ihdr();
        png.extend_from_slice(&png_test_chunk(b"risE", b"proof"));
        assert_eq!(png_extract_chunk(&png).unwrap(), b"proof");
    }

    #[test]
    fn png_find_truncated_rise_is_none() {
        // risE header + partial body, missing CRC bytes
        let mut png = png_test_ihdr();
        png.extend_from_slice(&5u32.to_be_bytes());
        png.extend_from_slice(b"risE");
        png.extend_from_slice(b"proof");
        assert!(png_extract_chunk(&png).is_none());
    }

    #[test]
    fn png_find_skips_big_chunk_before_rise() {
        // chunk length above 64 KiB: the second length byte is non-zero
        let mut png = png_test_ihdr();
        png.extend_from_slice(&png_test_chunk(b"tEXt", &png_test_big_filler()));
        png.extend_from_slice(&png_test_chunk(b"risE", b"proof"));
        png.extend_from_slice(&png_test_chunk(b"IEND", b""));
        assert_eq!(png_extract_chunk(&png).unwrap(), b"proof");
    }

    #[test]
    fn png_strip_rise_chunk_at_exact_eof() {
        // risE chunk ends exactly at EOF with no IEND after it
        let prefix = png_test_ihdr();
        let mut sealed = prefix.clone();
        sealed.extend_from_slice(&png_test_chunk(b"risE", b"proof"));
        assert_eq!(png_strip_chunk(&sealed).unwrap(), prefix);
    }

    #[test]
    fn png_strip_truncated_rise_is_none() {
        // risE header + partial body, missing CRC bytes
        let mut png = png_test_ihdr();
        png.extend_from_slice(&5u32.to_be_bytes());
        png.extend_from_slice(b"risE");
        png.extend_from_slice(b"proof");
        assert!(png_strip_chunk(&png).is_none());
    }

    #[test]
    fn png_strip_skips_big_chunk_before_rise() {
        // chunk length above 64 KiB: the second length byte is non-zero
        let big = png_test_chunk(b"tEXt", &png_test_big_filler());
        let rise = png_test_chunk(b"risE", b"proof");
        let iend = png_test_chunk(b"IEND", b"");
        let mut preseal = png_test_ihdr();
        preseal.extend_from_slice(&big);
        preseal.extend_from_slice(&iend);
        let mut sealed = png_test_ihdr();
        sealed.extend_from_slice(&big);
        sealed.extend_from_slice(&rise);
        sealed.extend_from_slice(&iend);
        assert_eq!(png_strip_chunk(&sealed).unwrap(), preseal);
    }

    #[test]
    fn png_strip_internal_returns_full_chunk() {
        // the removed chunk covers length + type + body + CRC
        let prefix = png_test_ihdr();
        let rise = png_test_chunk(b"risE", b"proof");
        let idat = png_test_chunk(b"IDAT", b"\x00\x00\x00\x00");
        let iend = png_test_chunk(b"IEND", b"");
        let mut sealed = prefix.clone();
        sealed.extend_from_slice(&rise);
        sealed.extend_from_slice(&idat);
        sealed.extend_from_slice(&iend);
        let (chunk, stream) = png_strip_chunk_internal(&sealed).unwrap();
        assert_eq!(chunk, rise);
        let mut expected = prefix;
        expected.extend_from_slice(&idat);
        expected.extend_from_slice(&iend);
        assert_eq!(stream, expected);
    }
}
