//! RFC 3161 timestamp client (MS3: R-201 single TSA, R-202 multi-TSA).
//!
//! The request is a DER `TimeStampReq` whose messageImprint carries
//! SHA256(canonical manifest JSON). The response is a DER `TimeStampResp`;
//! the stored token is base64(DER response). Verification re-extracts the
//! imprint and compares it to the expected digest — the TSA signature
//! itself is not chain-verified here (verify-web owns Trust List pinning).
//! Multi-TSA rule (`spec/solutions-root.md` §3): two tokens must agree on
//! the imprint and be within 5 minutes of each other, else reject.

use crate::RiseError;
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Default TSA endpoint (FreeTSA public service).
pub const DEFAULT_TSA_URL: &str = "https://freetsa.org/tsr";
/// Maximum accepted skew between two TSA tokens, in seconds (5 minutes).
pub const MAX_TSA_SKEW_SECS: i64 = 300;
/// SHA-256 algorithm OID value bytes (2.16.840.1.101.3.4.2.1).
const SHA256_OID_VALUE: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];

/// SHA256 of arbitrary bytes, returned as a fixed array.
pub fn sha256_of(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

/// DER length octets for `n`.
fn der_len(n: usize) -> Vec<u8> {
    if n < 128 {
        vec![n as u8]
    } else {
        let mut b = n.to_be_bytes().to_vec();
        while b.len() > 1 && b[0] == 0 {
            b.remove(0);
        }
        let mut out = vec![0x80 | (b.len() as u8)];
        out.extend_from_slice(&b);
        out
    }
}

/// DER TLV wrapper for `tag`.
fn der_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend_from_slice(&der_len(content.len()));
    out.extend_from_slice(content);
    out
}

/// DER positive INTEGER from a u64.
fn der_integer_u64(n: u64) -> Vec<u8> {
    let mut b = n.to_be_bytes().to_vec();
    while b.len() > 1 && b[0] == 0 {
        b.remove(0);
    }
    if b[0] & 0x80 != 0 {
        b.insert(0, 0);
    }
    der_tlv(0x02, &b)
}

/// Builds a DER TimeStampReq over `message` with an explicit nonce
/// (deterministic; the message itself is hashed with SHA-256).
pub fn build_timestamp_req_with_nonce(message: &[u8], nonce: u64) -> Vec<u8> {
    let digest = sha256_of(message);
    let mut oid = vec![0x06, SHA256_OID_VALUE.len() as u8];
    oid.extend_from_slice(SHA256_OID_VALUE);
    let mut alg = oid;
    alg.extend_from_slice(&der_tlv(0x05, b""));
    let alg_seq = der_tlv(0x30, &alg);
    let mut imprint = alg_seq;
    imprint.extend_from_slice(&der_tlv(0x04, &digest));
    let imprint_seq = der_tlv(0x30, &imprint);
    let mut body = der_integer_u64(1);
    body.extend_from_slice(&imprint_seq);
    body.extend_from_slice(&der_integer_u64(nonce));
    body.extend_from_slice(&der_tlv(0x01, &[0xff]));
    der_tlv(0x30, &body)
}

/// Builds a DER TimeStampReq over `message` with a fresh random nonce.
pub fn build_timestamp_req(message: &[u8]) -> Vec<u8> {
    // rand::random needs no handle; map failure is impossible here (OS RNG
    // panics only on total entropy failure, matching keygen behavior).
    let nonce: u64 = rand::random();
    build_timestamp_req_with_nonce(message, nonce)
}

/// Reads one DER TLV at `off`; returns (tag, value offset, value length).
fn read_tlv(der: &[u8], off: usize) -> Result<(u8, usize, usize), RiseError> {
    let bad = || RiseError::Crypto("tsa: malformed DER".to_string());
    if off + 2 > der.len() {
        return Err(bad());
    }
    let tag = der[off];
    let b0 = der[off + 1];
    let (len, hdr) = if b0 & 0x80 == 0 {
        (b0 as usize, 2)
    } else {
        let n = (b0 & 0x7f) as usize;
        if n == 0 || n > 4 || off + 2 + n > der.len() {
            return Err(bad());
        }
        let mut len = 0usize;
        for b in der.iter().skip(off + 2).take(n) {
            len = (len << 8) | (*b as usize);
        }
        (len, 2 + n)
    };
    if off + hdr + len > der.len() {
        return Err(bad());
    }
    Ok((tag, off + hdr, len))
}

/// Splits a constructed value into its direct children.
fn child_tlvs(der: &[u8], voff: usize, vlen: usize) -> Result<Vec<(u8, usize, usize)>, RiseError> {
    let mut out = Vec::new();
    let mut p = voff;
    while p < voff + vlen {
        let (tag, vo, vl) = read_tlv(der, p)?;
        out.push((tag, vo, vl));
        p = vo + vl;
    }
    if p != voff + vlen {
        return Err(RiseError::Crypto("tsa: malformed DER".to_string()));
    }
    Ok(out)
}

/// Checks the TimeStampResp outer status (0 granted or 1 grantedWithMods).
pub fn check_status_ok(resp: &[u8]) -> Result<(), RiseError> {
    let bad = || RiseError::Crypto("tsa: bad status".to_string());
    let (tag, vo, vl) = read_tlv(resp, 0).map_err(|_| bad())?;
    if tag != 0x30 {
        return Err(bad());
    }
    let top = child_tlvs(resp, vo, vl).map_err(|_| bad())?;
    let (stag, svo, svl) = *top.first().ok_or_else(bad)?;
    if stag != 0x30 {
        return Err(bad());
    }
    let info = child_tlvs(resp, svo, svl).map_err(|_| bad())?;
    let (itag, ivo, ivl) = *info.first().ok_or_else(bad)?;
    if itag != 0x02 || ivl == 0 || ivl > 2 {
        return Err(bad());
    }
    let mut v = 0u32;
    for b in resp.iter().skip(ivo).take(ivl) {
        v = (v << 8) | (*b as u32);
    }
    if v == 0 || v == 1 {
        Ok(())
    } else {
        Err(RiseError::Crypto(format!("tsa: status {v}")))
    }
}

/// Recursively finds MessageImprint: SEQ { SEQ { OID sha256, ... },
/// OCTET STRING(32) }.
fn find_imprint(der: &[u8], voff: usize, vlen: usize) -> Option<[u8; 32]> {
    let kids = child_tlvs(der, voff, vlen).ok()?;
    if kids.len() == 2 {
        let (t0, v0, l0) = kids[0];
        let (t1, v1, l1) = kids[1];
        if t0 == 0x30 && t1 == 0x04 && l1 == 32 {
            if let Ok(alg) = child_tlvs(der, v0, l0) {
                if let Some((a0, av0, al0)) = alg.first() {
                    if *a0 == 0x06 && der.get(*av0..av0 + al0) == Some(SHA256_OID_VALUE) {
                        let mut out = [0u8; 32];
                        out.copy_from_slice(&der[v1..v1 + 32]);
                        return Some(out);
                    }
                }
            }
        }
    }
    for (tag, vo, vl) in kids {
        // SignedData/TSTInfo travel inside OCTET STRINGs (tag 0x04), so
        // descend opportunistically; child_tlvs fails cleanly on raw bytes.
        if tag == 0x30 || tag == 0x31 || tag == 0x04 || (tag & 0xc0) == 0x80 {
            if let Some(hit) = find_imprint(der, vo, vl) {
                return Some(hit);
            }
        }
    }
    None
}

/// Extracts the messageImprint digest from a TimeStampResp.
pub fn extract_imprint(resp: &[u8]) -> Result<[u8; 32], RiseError> {
    check_status_ok(resp)?;
    let (tag, vo, vl) =
        read_tlv(resp, 0).map_err(|_| RiseError::Crypto("tsa: malformed response".to_string()))?;
    if tag != 0x30 {
        return Err(RiseError::Crypto("tsa: malformed response".to_string()));
    }
    find_imprint(resp, vo, vl)
        .ok_or_else(|| RiseError::Crypto("tsa: messageImprint not found".to_string()))
}

/// Recursively finds the first GeneralizedTime (tag 0x18) value.
fn find_time(der: &[u8], voff: usize, vlen: usize) -> Option<String> {
    let kids = child_tlvs(der, voff, vlen).ok()?;
    for (tag, vo, vl) in kids {
        if tag == 0x18 && (13..=20).contains(&vl) {
            if let Ok(s) = std::str::from_utf8(&der[vo..vo + vl]) {
                if s.ends_with('Z')
                    && s[..s.len() - 1]
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '.')
                {
                    return Some(s.to_string());
                }
            }
        }
        if tag == 0x30 || tag == 0x31 || tag == 0x04 || (tag & 0xc0) == 0x80 {
            if let Some(hit) = find_time(der, vo, vl) {
                return Some(hit);
            }
        }
    }
    None
}

/// Extracts the token genTime as a raw GeneralizedTime string.
pub fn extract_gen_time(resp: &[u8]) -> Result<String, RiseError> {
    check_status_ok(resp)?;
    let (tag, vo, vl) =
        read_tlv(resp, 0).map_err(|_| RiseError::Crypto("tsa: malformed response".to_string()))?;
    if tag != 0x30 {
        return Err(RiseError::Crypto("tsa: malformed response".to_string()));
    }
    find_time(resp, vo, vl).ok_or_else(|| RiseError::Crypto("tsa: genTime not found".to_string()))
}

/// Converts "YYYYMMDDHHMMSS[.fff]Z" to unix seconds.
pub fn gen_time_to_unix(s: &str) -> Result<i64, RiseError> {
    let bad = || RiseError::Crypto(format!("tsa: bad genTime {s}"));
    let s = s.strip_suffix('Z').ok_or_else(bad)?;
    let s = match s.find('.') {
        Some(i) => &s[..i],
        None => s,
    };
    if s.len() != 14 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let num = |i: usize, j: usize| s[i..j].parse::<i64>().map_err(|_| bad());
    let (year, mon, day, hour, min, sec) = (
        num(0, 4)?,
        num(4, 6)?,
        num(6, 8)?,
        num(8, 10)?,
        num(10, 12)?,
        num(12, 14)?,
    );
    if !(1..=12).contains(&mon) || !(1..=31).contains(&day) || hour > 23 || min > 59 || sec > 60 {
        return Err(bad());
    }
    Ok(days_from_civil(year, mon, day) * 86400 + hour * 3600 + min * 60 + sec)
}

/// Days since unix epoch (Howard Hinnant's civil-days algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Verifies that the token imprint equals the expected digest.
pub fn verify_token_against_digest(resp: &[u8], digest: &[u8; 32]) -> Result<(), RiseError> {
    let got = extract_imprint(resp)?;
    if &got == digest {
        Ok(())
    } else {
        Err(RiseError::Crypto("tsa: imprint mismatch".to_string()))
    }
}

/// Multi-TSA cross-check: both tokens must carry the expected imprint and
/// their genTimes must be within [`MAX_TSA_SKEW_SECS`].
pub fn cross_check_tokens(a: &[u8], b: &[u8], digest: &[u8; 32]) -> Result<(), RiseError> {
    verify_token_against_digest(a, digest)?;
    verify_token_against_digest(b, digest)?;
    let ta = gen_time_to_unix(&extract_gen_time(a)?)?;
    let tb = gen_time_to_unix(&extract_gen_time(b)?)?;
    if (ta - tb).abs() > MAX_TSA_SKEW_SECS {
        return Err(RiseError::Crypto(format!(
            "tsa: token times differ by {}s",
            (ta - tb).abs()
        )));
    }
    Ok(())
}

/// Requests a timestamp token from a TSA over `message` (network path:
/// every failure maps to [`RiseError::Crypto`], never panics).
pub fn request_timestamp(tsa_url: &str, message: &[u8]) -> Result<Vec<u8>, RiseError> {
    let req = build_timestamp_req(message);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let mut resp = agent
        .post(tsa_url)
        .header("Content-Type", "application/timestamp-query")
        .header("Accept", "application/timestamp-reply")
        .send(&req[..])
        .map_err(|e| RiseError::Crypto(format!("tsa request: {e}")))?;
    let body = resp
        .body_mut()
        .read_to_vec()
        .map_err(|e| RiseError::Crypto(format!("tsa read: {e}")))?;
    check_status_ok(&body)?;
    verify_token_against_digest(&body, &sha256_of(message))?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded live FreeTSA responses (fetched once; tests parse offline).
    const VEC_A: &[u8] = include_bytes!("../tests/vectors/freetsa_a.der");
    const VEC_B: &[u8] = include_bytes!("../tests/vectors/freetsa_b.der");
    const SEED_DIGEST: &[u8] = include_bytes!("../tests/vectors/imprint_seed.sha256");

    fn seed_digest() -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(SEED_DIGEST);
        out
    }

    #[test]
    fn req_carries_imprint_and_nonce() {
        // deterministic build: imprint present, nonce present
        let req = build_timestamp_req_with_nonce(b"rise-req", 7);
        assert_eq!(req[0], 0x30);
        assert!(req.windows(9).any(|w| w == SHA256_OID_VALUE));
        assert!(req.windows(32).any(|w| w == sha256_of(b"rise-req")));
        assert!(req.windows(3).any(|w| w == [0x02, 0x01, 0x07]));
        // same nonce twice is byte-identical; random builds differ
        assert_eq!(req, build_timestamp_req_with_nonce(b"rise-req", 7));
        assert_ne!(build_timestamp_req(b"x"), build_timestamp_req(b"x"));
    }

    #[test]
    fn recorded_tokens_parse_and_verify() {
        // both recorded tokens are granted and carry the seed imprint
        for v in [VEC_A, VEC_B] {
            assert!(check_status_ok(v).is_ok());
            assert_eq!(extract_imprint(v).unwrap(), seed_digest());
            assert!(verify_token_against_digest(v, &seed_digest()).is_ok());
        }
        // genTimes parse to a plausible unix time (after 2024)
        for v in [VEC_A, VEC_B] {
            let t = extract_gen_time(v).unwrap();
            assert!(gen_time_to_unix(&t).unwrap() > 1_700_000_000);
        }
    }

    #[test]
    fn wrong_digest_rejected() {
        // a different expected digest fails verification
        assert!(verify_token_against_digest(VEC_A, &[9u8; 32]).is_err());
        assert!(cross_check_tokens(VEC_A, VEC_B, &[9u8; 32]).is_err());
    }

    #[test]
    fn cross_check_accepts_pair_and_rejects_skew() {
        // two independent live tokens agree: imprint match, times close
        assert!(cross_check_tokens(VEC_A, VEC_B, &seed_digest()).is_ok());
        // patch the genTime hour +6 in a copy: imprint still matches,
        // but the skew check must fire
        let t = extract_gen_time(VEC_A).unwrap();
        let mut patched = VEC_A.to_vec();
        let pos = patched
            .windows(t.len())
            .position(|w| w == t.as_bytes())
            .unwrap();
        let hour: u32 = t[8..10].parse().unwrap();
        let hour = (hour + 6) % 24;
        patched[pos + 8] = b'0' + (hour / 10) as u8;
        patched[pos + 9] = b'0' + (hour % 10) as u8;
        assert!(verify_token_against_digest(&patched, &seed_digest()).is_ok());
        assert!(cross_check_tokens(VEC_A, &patched, &seed_digest()).is_err());
    }

    #[test]
    fn corrupted_imprint_rejected() {
        // flip one imprint byte in a copy: extraction must disagree
        let digest = seed_digest();
        let mut patched = VEC_A.to_vec();
        let pos = patched.windows(32).position(|w| w == digest).unwrap();
        patched[pos] ^= 0xff;
        assert!(verify_token_against_digest(&patched, &digest).is_err());
        assert!(cross_check_tokens(&patched, VEC_B, &digest).is_err());
    }

    #[test]
    fn gen_time_parser_edges() {
        // known instant: 2026-01-01T00:00:00Z
        assert_eq!(gen_time_to_unix("20260101000000Z").unwrap(), 1_767_225_600);
        assert!(gen_time_to_unix("20260101000000").is_err());
        assert!(gen_time_to_unix("not-a-timeZ").is_err());
        assert!(gen_time_to_unix("20261301000000Z").is_err());
    }

    #[test]
    fn malformed_der_rejected() {
        assert!(check_status_ok(b"").is_err());
        assert!(check_status_ok(b"\x30\x03\x02\x01\x00").is_err());
        assert!(extract_imprint(b"\x30\x03\x02\x01\x00").is_err());
        assert!(extract_gen_time(b"\x30\x03\x02\x01\x00").is_err());
    }

    /// Live TSA round-trip (needs network; run with --ignored).
    #[test]
    #[ignore]
    fn live_tsa_roundtrip() {
        let msg = b"rise-ms3-live-probe";
        let tok = request_timestamp(DEFAULT_TSA_URL, msg).unwrap();
        assert!(verify_token_against_digest(&tok, &sha256_of(msg)).is_ok());
    }
}
