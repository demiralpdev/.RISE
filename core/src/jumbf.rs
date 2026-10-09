//! JUMBF (ISO 19566-5) APP11 embedding — the container C2PA 2.4 uses for JPEG.
//! The manifest rides in APP11 segments as a `jumb` superbox carrying a `json` box.

use crate::RiseError;

/// C2PA manifest store type UUID (`c2pa`, JUMBF registry).
const C2PA_UUID: [u8; 16] = [
    0x63, 0x32, 0x70, 0x61, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// APP11 segment payload prefix.
const JUMBF_PREFIX: &[u8] = b"JUMBF\x00";

/// Max bytes a JPEG segment can carry: 65535 len field minus marker+len+prefix.
const MAX_PAYLOAD: usize = 65535 - 2 - 2 - 6;

fn be32(n: usize) -> [u8; 4] {
    (n as u32).to_be_bytes()
}

/// Builds a JUMBF superbox: [len][jumb][desc box][json box].
pub fn build_superbox(json_payload: &[u8]) -> Vec<u8> {
    // description box: len + "jumd" + UUID + toggles
    let desc_len = 4 + 4 + 16 + 1;
    let mut desc = Vec::with_capacity(desc_len);
    desc.extend_from_slice(&be32(desc_len));
    desc.extend_from_slice(b"jumd");
    desc.extend_from_slice(&C2PA_UUID);
    desc.push(0x00); // toggles: no label/id/signature

    // json content box: len + "json" + payload
    let json_len = 4 + 4 + json_payload.len();
    let mut json_box = Vec::with_capacity(json_len);
    json_box.extend_from_slice(&be32(json_len));
    json_box.extend_from_slice(b"json");
    json_box.extend_from_slice(json_payload);

    // superbox: len + "jumb" + content
    let super_len = 4 + 4 + desc.len() + json_box.len();
    let mut out = Vec::with_capacity(super_len);
    out.extend_from_slice(&be32(super_len));
    out.extend_from_slice(b"jumb");
    out.extend_from_slice(&desc);
    out.extend_from_slice(&json_box);
    out
}

/// Extracts the json box payload from JUMBF superbox bytes.
pub fn parse_json_payload(superbox: &[u8]) -> Result<Vec<u8>, RiseError> {
    // header: len + "jumb"
    if superbox.len() < 8 || &superbox[4..8] != b"jumb" {
        return Err(RiseError::Json(
            serde_json::from_str::<serde_json::Value>("").unwrap_err(),
        ));
    }
    let mut pos = 8usize;
    // walk content boxes
    while pos + 8 <= superbox.len() {
        let blen = u32::from_be_bytes([
            superbox[pos],
            superbox[pos + 1],
            superbox[pos + 2],
            superbox[pos + 3],
        ]) as usize;
        if blen < 8 || pos + blen > superbox.len() {
            break;
        }
        let btype = &superbox[pos + 4..pos + 8];
        if btype == b"json" {
            return Ok(superbox[pos + 8..pos + blen].to_vec());
        }
        pos += blen;
    }
    Err(RiseError::RootMismatch)
}

/// Embeds the JUMBF payload into a JPEG as APP11 segment(s), right after SOI.
pub fn jpeg_embed(jpg: &[u8], json_payload: &[u8]) -> Result<Vec<u8>, RiseError> {
    // JPEG must start with SOI
    if jpg.len() < 2 || jpg[0] != 0xFF || jpg[1] != 0xD8 {
        return Err(RiseError::RootMismatch);
    }
    let superbox = build_superbox(json_payload);
    let mut out = Vec::with_capacity(jpg.len() + superbox.len() + 64);
    out.extend_from_slice(&jpg[..2]); // SOI
                                      // chunk the superbox into APP11 segments
    for chunk in superbox.chunks(MAX_PAYLOAD) {
        out.extend_from_slice(&[0xFF, 0xEB]);
        let seg_len = 2 + 6 + chunk.len();
        out.extend_from_slice(&(seg_len as u16).to_be_bytes());
        out.extend_from_slice(JUMBF_PREFIX);
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&jpg[2..]);
    Ok(out)
}

/// Extracts the concatenated JUMBF superbox from all APP11 segments.
pub fn jpeg_extract(jpg: &[u8]) -> Option<Vec<u8>> {
    // not a JPEG
    if jpg.len() < 2 || jpg[0] != 0xFF || jpg[1] != 0xD8 {
        return None;
    }
    let mut jumbf: Vec<u8> = Vec::new();
    let mut pos = 2usize;
    // walk segments
    while pos + 4 <= jpg.len() {
        if jpg[pos] != 0xFF {
            break;
        }
        let marker = jpg[pos + 1];
        // SOS: entropy-coded data follows, stop scanning
        if marker == 0xDA {
            break;
        }
        let seg_len = u16::from_be_bytes([jpg[pos + 2], jpg[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > jpg.len() {
            break;
        }
        if marker == 0xEB {
            // APP11: strip the JUMBF prefix and collect
            let payload = &jpg[pos + 4..pos + 2 + seg_len];
            if payload.starts_with(JUMBF_PREFIX) {
                jumbf.extend_from_slice(&payload[6..]);
            }
        }
        pos += 2 + seg_len;
    }
    if jumbf.is_empty() {
        None
    } else {
        Some(jumbf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superbox_carries_c2pa_uuid() {
        let sb = build_superbox(b"{}");
        assert_eq!(&sb[4..8], b"jumb");
        // c2pa UUID sits inside the description box (8+4+4 offset)
        assert!(sb[16..32].iter().zip(C2PA_UUID.iter()).all(|(a, b)| a == b));
    }

    #[test]
    fn json_payload_roundtrip() {
        let payload = br#"{"rise_version":1}"#;
        let sb = build_superbox(payload);
        assert_eq!(parse_json_payload(&sb).unwrap(), payload.to_vec());
    }

    #[test]
    fn jpeg_embed_extract_roundtrip() {
        // minimal JPEG: SOI + APP0-ish + SOS-ish tail
        let mut jpg = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22,
        ];
        let payload = br#"{"merkle_root":"aa"}"#;
        let embedded = jpeg_embed(&jpg, payload).unwrap();
        // APP11 marker present right after SOI
        assert_eq!(&embedded[2..4], &[0xFF, 0xEB]);
        let extracted = jpeg_extract(&embedded).unwrap();
        assert_eq!(parse_json_payload(&extracted).unwrap(), payload.to_vec());
        // the original image bytes survive after the embedded segments
        jpg[0] = 0xFF;
        assert!(embedded.ends_with(&jpg[2..]));
    }

    #[test]
    fn extract_unmarked_returns_none() {
        let jpg = vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02];
        assert!(jpeg_extract(&jpg).is_none());
        // not a JPEG at all
        assert!(jpeg_extract(b"notjpeg").is_none());
    }
}
