//! RISE core logic — single shared source.
//! Formula: SHA256(raw frame) at shutter.
//! MS1: ManifestV1 schema (RISE-01 §5) + streaming hash.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use thiserror::Error;

pub mod jumbf;
pub mod signer;
pub mod timestamp;
pub mod transparency;

/// Core error type.
#[derive(Debug, Error)]
pub enum RiseError {
    /// I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Frame hash did not match.
    #[error("frame hash mismatch")]
    FrameMismatch,
    /// Tile hashes did not match.
    #[error("tile hashes mismatch")]
    TileMismatch,
    /// Merkle root did not match.
    #[error("merkle root mismatch")]
    RootMismatch,
    /// Crypto operation failed.
    #[error("crypto error: {0}")]
    Crypto(String),
    /// Signature missing or did not verify.
    #[error("invalid signature")]
    InvalidSignature,
}

/// Signature algorithm (RISE-01 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SigAlg {
    /// Default ECDSA P-256.
    #[serde(rename = "ES256")]
    Es256,
    /// Option for constrained devices.
    #[serde(rename = "Ed25519")]
    Ed25519,
}

/// Default signature is ES256.
impl Default for SigAlg {
    fn default() -> Self {
        Self::Es256
    }
}

/// Manifest v1 (RISE-01 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestV1 {
    /// Format version (1).
    pub rise_version: u8,
    /// Frame SHA-256 list.
    pub frame_hashes: Vec<String>,
    /// Tile SHA-256 list.
    pub tile_hashes: Vec<String>,
    /// Merkle root (hex).
    pub merkle_root: String,
    /// Signature algorithm.
    pub sig_alg: SigAlg,
    /// Time (milliseconds).
    pub timestamp_ms: u64,
    /// Anonymous device id.
    pub device_id: String,
    /// Trust tier: silver, gold-L2, gold-L4 (RISE-01 §5).
    #[serde(default = "default_assurance")]
    pub assurance: String,
    /// RFC 3161 token (None until MS3 wires a TSA).
    #[serde(default)]
    pub timestamp_token: Option<String>,
    /// Signature over the canonical JSON (hex, None while unsigned).
    #[serde(default)]
    pub signature: Option<String>,
    /// Key id used to sign.
    #[serde(default)]
    pub signing_key_id: Option<String>,
}

/// Default tier is silver.
fn default_assurance() -> String {
    "silver".to_string()
}

/// SHA256 of the frame bytes (hex).
pub fn sha256_frame(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex_encode(h.finalize())
}

/// Streaming SHA256 (reads large frames in chunks).
pub fn sha256_stream<R: Read>(mut reader: R) -> Result<String, RiseError> {
    // 8KB chunks keep memory flat
    let mut h = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        // finalize when done
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex_encode(h.finalize()))
}

/// Splits the frame into at most 4 equal tiles.
fn dilimler(data: &[u8]) -> Vec<&[u8]> {
    // empty frame has no tiles
    if data.is_empty() {
        return Vec::new();
    }
    // at most 4 tiles
    let n = data.len().min(4);
    // tile size via ceiling division
    let boy = data.len().div_ceil(n);
    data.chunks(boy).collect()
}

/// Builds a manifest v1 (multi-frame supported).
pub fn create_manifest_v1(
    frames: &[&[u8]],
    device_id: &str,
    timestamp_ms: u64,
    sig_alg: SigAlg,
    assurance: &str,
) -> ManifestV1 {
    // hash every frame
    let frame_hashes: Vec<String> = frames.iter().map(|f| sha256_frame(f)).collect();
    // collect tiles of every frame
    let mut tile_hashes = Vec::new();
    for f in frames {
        for d in dilimler(f) {
            tile_hashes.push(tile_hash(d));
        }
    }
    // derive the root from tiles
    let merkle_root = merkle_root(&tile_hashes);
    ManifestV1 {
        rise_version: 1,
        frame_hashes,
        tile_hashes,
        merkle_root,
        sig_alg,
        timestamp_ms,
        device_id: device_id.to_string(),
        assurance: assurance.to_string(),
        timestamp_token: None,
        signature: None,
        signing_key_id: None,
    }
}

/// Serializes the manifest to a JSON string.
pub fn manifest_to_json(m: &ManifestV1) -> Result<String, RiseError> {
    Ok(serde_json::to_string(m)?)
}

/// Parses a manifest from a JSON string.
pub fn manifest_from_json(s: &str) -> Result<ManifestV1, RiseError> {
    Ok(serde_json::from_str(s)?)
}

/// Verifies a manifest v1 (recompute + compare).
pub fn verify_manifest_v1(manifest: &ManifestV1, frames: &[&[u8]]) -> Result<(), RiseError> {
    // recompute frame hashes
    let taze: Vec<String> = frames.iter().map(|f| sha256_frame(f)).collect();
    if taze != manifest.frame_hashes {
        return Err(RiseError::FrameMismatch);
    }
    // recompute tiles
    let mut doseme = Vec::new();
    for f in frames {
        for d in dilimler(f) {
            doseme.push(tile_hash(d));
        }
    }
    if doseme != manifest.tile_hashes {
        return Err(RiseError::TileMismatch);
    }
    // recompute the root
    if merkle_root(&doseme) != manifest.merkle_root {
        return Err(RiseError::RootMismatch);
    }
    Ok(())
}

/// Legacy single-frame manifest (to be removed).
#[deprecated(note = "use create_manifest_v1")]
pub fn create_manifest(frame: &[u8], cihaz: &str, zaman: u64) -> String {
    let kare = sha256_frame(frame);
    let dosemeler: Vec<String> = dilimler(frame).iter().map(|d| tile_hash(d)).collect();
    let kok = merkle_root(&dosemeler);
    serde_json::json!({
        "frame_hash": kare,
        "tile_hashes": dosemeler,
        "merkle_root": kok,
        "timestamp": zaman,
        "device": cihaz,
    })
    .to_string()
}

/// Trust decision (MS3, R-205). Fail-closed: suspicion downgrades,
/// an unreachable trust source yields Unknown, never Valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyDecision {
    /// Hash chain holds and the signature verified.
    Valid,
    /// Tampering or a bad signature, with a one-line reason.
    Red(String),
    /// Chain holds but the claim is unverifiable (unsigned, or no
    /// pubkey to check the signature against), with a reason.
    Unknown(String),
}

/// Maps hash/Merkle/signature states to a trust decision.
/// The signature check is informational until a pubkey is passed:
/// without a verifier an intact chain yields Unknown, never Valid.
pub fn verify_decision(
    manifest: &ManifestV1,
    frames: &[&[u8]],
    verifier: Option<&dyn signer::Verifier>,
) -> VerifyDecision {
    if let Err(e) = verify_manifest_v1(manifest, frames) {
        return VerifyDecision::Red(format!("hash chain: {e}"));
    }
    match verifier {
        Some(v) => match verify_signature(manifest, v) {
            Ok(()) => VerifyDecision::Valid,
            Err(e) => VerifyDecision::Red(format!("signature: {e}")),
        },
        None => {
            if manifest.signature.is_some() {
                VerifyDecision::Unknown("signature present but no pubkey given".to_string())
            } else {
                VerifyDecision::Unknown("unsigned manifest: not evidence".to_string())
            }
        }
    }
}

/// Signs the manifest in place (MS2 real signing).
/// The signature covers the canonical JSON: signing_key_id set, signature empty.
pub fn sign_manifest_v1(m: &mut ManifestV1, signer: &dyn signer::Signer) -> Result<(), RiseError> {
    // already signed: refuse double signing
    if m.signature.is_some() {
        return Err(RiseError::InvalidSignature);
    }
    // canonical form: key id set, signature still empty
    m.signing_key_id = Some(signer.key_id());
    let canon = manifest_to_json(m)?;
    let sig = signer.sign(canon.as_bytes())?;
    m.signature = Some(hex_encode(sig));
    Ok(())
}

/// Verifies the manifest signature over the canonical JSON.
pub fn verify_signature(m: &ManifestV1, verifier: &dyn signer::Verifier) -> Result<(), RiseError> {
    let Some(sig_hex) = &m.signature else {
        return Err(RiseError::InvalidSignature);
    };
    let sig_bytes = hex_decode(sig_hex)?;
    // rebuild the exact canonical form the signer used
    let mut tmp = m.clone();
    tmp.signature = None;
    let canon = manifest_to_json(&tmp)?;
    verifier.verify(canon.as_bytes(), &sig_bytes)
}

/// Signs the manifest (unnamed stub).
/// TODO: add a real key and signature (offline).
pub fn sign_manifest(manifest: &str) -> String {
    // TODO: finalize the signature format.
    format!("UNSIGNED:{}", manifest)
}

/// Verifies the legacy manifest by local recomputation.
pub fn verify_manifest(manifest: &str, frame: &[u8]) -> bool {
    let v: serde_json::Value = match serde_json::from_str(manifest) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let Some(kare) = v.get("frame_hash").and_then(|x| x.as_str()) else {
        return false;
    };
    let Some(dizi) = v.get("tile_hashes") else {
        return false;
    };
    let Some(kok) = v.get("merkle_root").and_then(|x| x.as_str()) else {
        return false;
    };
    if sha256_frame(frame) != kare {
        return false;
    }
    let taze: Vec<String> = dilimler(frame).iter().map(|d| tile_hash(d)).collect();
    let bek: Vec<String> = match serde_json::from_value(dizi.clone()) {
        Ok(x) => x,
        Err(_) => return false,
    };
    if taze != bek {
        return false;
    }
    merkle_root(&taze) == kok
}

/// Hash of a tile (raw frame slice).
/// Formula: SHA256(raw tile).
pub fn tile_hash(data: &[u8]) -> String {
    // same formula as the frame hash
    sha256_frame(data)
}

/// Merkle root from leaf hashes (hex).
/// A single leaf is carried up unchanged.
pub fn merkle_root(hashes: &[String]) -> String {
    // empty set: hash of the empty input
    if hashes.is_empty() {
        return sha256_frame(b"");
    }
    let mut kat: Vec<String> = hashes.to_vec();
    // fold to the root
    while kat.len() > 1 {
        let mut ust = Vec::with_capacity(kat.len().div_ceil(2));
        let mut i = 0;
        while i < kat.len() {
            // repeat the left leaf if the right one is missing
            let sag = if i + 1 < kat.len() {
                &kat[i + 1]
            } else {
                &kat[i]
            };
            // hash the pair of hashes
            let mut h = Sha256::new();
            h.update(kat[i].as_bytes());
            h.update(sag.as_bytes());
            ust.push(hex_encode(h.finalize()));
            i += 2;
        }
        kat = ust;
    }
    kat.into_iter().next().unwrap()
}

/// Encodes bytes to lowercase hex.
fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    let b = bytes.as_ref();
    let mut s = String::with_capacity(b.len() * 2);
    for v in b {
        s.push(char::from_digit((v >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((v & 0xf) as u32, 16).unwrap());
    }
    s
}

/// Decodes lowercase hex back to bytes; odd length or bad chars error out.
fn hex_decode(s: &str) -> Result<Vec<u8>, RiseError> {
    if !s.len().is_multiple_of(2) {
        return Err(RiseError::Crypto("hex: odd length".to_string()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| RiseError::Crypto("hex: invalid char".to_string()))
        })
        .collect()
}

/// FFI: SHA256(raw frame); the caller allocates a 65-byte buffer (64 hex + NUL).
/// # Safety
/// The caller must guarantee valid pointers and a 65-byte output buffer per core.h.
#[no_mangle]
pub unsafe extern "C" fn rise_hash_frame(bytes: *const u8, len: usize, out_hex65: *mut u8) {
    if bytes.is_null() || out_hex65.is_null() {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let hex = sha256_frame(slice);
    let out = unsafe { std::slice::from_raw_parts_mut(out_hex65, 65) };
    for (i, b) in hex.as_bytes().iter().take(64).enumerate() {
        out[i] = *b;
    }
    out[64] = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_hash_expected() {
        // known hash of the empty input
        assert_eq!(
            sha256_frame(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
    #[test]
    fn stream_hash_correct() {
        // streaming read matches the slice hash
        let veri = b"rise-stream-test-12345";
        let bek = sha256_frame(veri);
        let akis = sha256_stream(&veri[..]).unwrap();
        assert_eq!(akis, bek);
        // empty stream yields the empty hash
        assert_eq!(sha256_stream(&b""[..]).unwrap(), sha256_frame(b""));
    }
    #[test]
    fn sig_alg_default() {
        // the default is ES256
        assert_eq!(SigAlg::default(), SigAlg::Es256);
        // serde names match the spec
        assert_eq!(serde_json::to_string(&SigAlg::Es256).unwrap(), "\"ES256\"");
        assert_eq!(
            serde_json::to_string(&SigAlg::Ed25519).unwrap(),
            "\"Ed25519\""
        );
    }
    #[test]
    fn manifest_v1_multi_frame() {
        // a three-frame manifest
        let k1 = b"frame-one";
        let k2 = b"frame-two";
        let k3 = b"frame-three";
        let m = create_manifest_v1(
            &[&k1[..], &k2[..], &k3[..]],
            "device-1",
            999,
            SigAlg::Es256,
            "silver",
        );
        // version and fields are complete
        assert_eq!(m.rise_version, 1);
        assert_eq!(m.frame_hashes.len(), 3);
        assert_eq!(m.frame_hashes[0], sha256_frame(k1));
        assert_eq!(m.device_id, "device-1");
        assert_eq!(m.timestamp_ms, 999);
        // passes with the correct frames
        assert!(verify_manifest_v1(&m, &[&k1[..], &k2[..], &k3[..]]).is_ok());
        // fails with a wrong frame
        assert!(verify_manifest_v1(&m, &[&k1[..], &k2[..], b"wrong"]).is_err());
    }
    #[test]
    fn manifest_json_roundtrip() {
        // build + write + read
        let m = create_manifest_v1(&[&b"a"[..]], "test-device", 123, SigAlg::Ed25519, "silver");
        let s = manifest_to_json(&m).unwrap();
        // expected field names are present
        assert!(s.contains("rise_version"));
        assert!(s.contains("frame_hashes"));
        assert!(s.contains("tile_hashes"));
        assert!(s.contains("merkle_root"));
        assert!(s.contains("sig_alg"));
        assert!(s.contains("timestamp_ms"));
        assert!(s.contains("device_id"));
        // read back
        let geri = manifest_from_json(&s).unwrap();
        assert_eq!(m, geri);
        // corrupt JSON errors
        assert!(manifest_from_json("broken").is_err());
    }
    #[test]
    fn manifest_v1_empty_frame() {
        // an empty list yields the empty root
        let m = create_manifest_v1(&[], "d", 0, SigAlg::Es256, "silver");
        assert!(m.frame_hashes.is_empty());
        assert_eq!(m.merkle_root, sha256_frame(b""));
        assert!(verify_manifest_v1(&m, &[]).is_ok());
    }
    #[test]
    fn manifest_sign_verify_roundtrip() {
        // generate a keypair, sign the manifest, verify it
        let (priv_pem, pub_pem) = signer::generate_p256().unwrap();
        std::fs::write(std::env::temp_dir().join("rise-mv.pem").to_str().unwrap(), &priv_pem).unwrap();
        std::fs::write(std::env::temp_dir().join("rise-mv.pub.pem").to_str().unwrap(), &pub_pem).unwrap();
        let kare = b"rise-signed-frame";
        let mut m = create_manifest_v1(&[&kare[..]], "test-device", 123, SigAlg::Es256, "silver");
        let s = signer::P256Signer::from_pem_file(std::env::temp_dir().join("rise-mv.pem").to_str().unwrap()).unwrap();
        sign_manifest_v1(&mut m, &s).unwrap();
        // signature present, no stub prefix anywhere
        assert!(m.signature.is_some());
        let json = manifest_to_json(&m).unwrap();
        assert!(!json.contains("UNSIGNED"));
        // the correct signature verifies
        let v = signer::P256Verifier::from_pem_file(std::env::temp_dir().join("rise-mv.pub.pem").to_str().unwrap()).unwrap();
        verify_signature(&m, &v).unwrap();
        // tampering the signed content breaks it
        let mut bad = m.clone();
        bad.device_id = "attacker".into();
        assert!(verify_signature(&bad, &v).is_err());
        std::fs::remove_file(std::env::temp_dir().join("rise-mv.pem").to_str().unwrap()).unwrap();
        std::fs::remove_file(std::env::temp_dir().join("rise-mv.pub.pem").to_str().unwrap()).unwrap();
    }
    #[test]
    fn double_sign_refused() {
        let (priv_pem, _pub_pem) = signer::generate_p256().unwrap();
        std::fs::write(std::env::temp_dir().join("rise-ds.pem").to_str().unwrap(), &priv_pem).unwrap();
        let kare = b"rise-ds-frame";
        let mut m = create_manifest_v1(&[&kare[..]], "d", 0, SigAlg::Es256, "silver");
        let s = signer::P256Signer::from_pem_file(std::env::temp_dir().join("rise-ds.pem").to_str().unwrap()).unwrap();
        sign_manifest_v1(&mut m, &s).unwrap();
        assert!(sign_manifest_v1(&mut m, &s).is_err());
        std::fs::remove_file(std::env::temp_dir().join("rise-ds.pem").to_str().unwrap()).unwrap();
    }
    #[test]
    fn tile_merkle_consistent() {
        // tile uses the same formula as the frame
        assert_eq!(tile_hash(b"abc"), sha256_frame(b"abc"));
        // empty set yields the empty hash
        assert_eq!(merkle_root(&[]), sha256_frame(b""));
        // a single leaf returns unchanged
        let tek = vec![tile_hash(b"a")];
        assert_eq!(merkle_root(&tek), tek[0]);
        // two leaves fold into a 64-char root
        let iki = vec![tile_hash(b"a"), tile_hash(b"b")];
        let kok = merkle_root(&iki);
        assert_eq!(kok.len(), 64);
        assert_ne!(kok, iki[0]);
    }
    #[test]
    fn legacy_verification_roundtrip() {
        // fixed input
        let kare = b"rise-test";
        // the legacy API still works
        #[allow(deprecated)]
        let m = create_manifest(kare, "test-device", 123);
        // the correct frame passes
        assert!(verify_manifest(&m, kare));
        // a wrong frame fails
        assert!(!verify_manifest(&m, b"wrong"));
        // corrupt JSON fails
        assert!(!verify_manifest("broken", kare));
    }
    #[test]
    fn reordered_tiles_fail_verification() {
        // R-204: the tile list order is bound to the manifest; swapping
        // two tiles must fail verification (tile-swap protection)
        let k1 = b"tile-frame-one-payload";
        let k2 = b"tile-frame-two-payload";
        let m = create_manifest_v1(&[&k1[..], &k2[..]], "d", 0, SigAlg::Es256, "silver");
        assert!(m.tile_hashes.len() >= 2);
        assert!(verify_manifest_v1(&m, &[&k1[..], &k2[..]]).is_ok());
        let mut bad = m.clone();
        bad.tile_hashes.swap(0, 1);
        assert!(matches!(
            verify_manifest_v1(&bad, &[&k1[..], &k2[..]]),
            Err(RiseError::TileMismatch)
        ));
        assert!(matches!(
            verify_decision(&bad, &[&k1[..], &k2[..]], None),
            VerifyDecision::Red(_)
        ));
    }
    #[test]
    fn verify_decision_states() {
        // intact chain, unsigned: Unknown (not evidence), never Valid
        let kare = b"rise-decision-frame";
        let m = create_manifest_v1(&[&kare[..]], "d", 0, SigAlg::Es256, "silver");
        assert!(matches!(
            verify_decision(&m, &[&kare[..]], None),
            VerifyDecision::Unknown(_)
        ));
        // intact chain, signed, no pubkey: still Unknown (informational)
        let (priv_pem, pub_pem) = signer::generate_p256().unwrap();
        std::fs::write(std::env::temp_dir().join("rise-dec.pem").to_str().unwrap(), &priv_pem).unwrap();
        std::fs::write(std::env::temp_dir().join("rise-dec.pub.pem").to_str().unwrap(), &pub_pem).unwrap();
        let mut signed = m.clone();
        let s = signer::P256Signer::from_pem_file(std::env::temp_dir().join("rise-dec.pem").to_str().unwrap()).unwrap();
        sign_manifest_v1(&mut signed, &s).unwrap();
        assert!(matches!(
            verify_decision(&signed, &[&kare[..]], None),
            VerifyDecision::Unknown(_)
        ));
        // signed + correct pubkey: Valid
        let v = signer::P256Verifier::from_pem_file(std::env::temp_dir().join("rise-dec.pub.pem").to_str().unwrap()).unwrap();
        assert_eq!(
            verify_decision(&signed, &[&kare[..]], Some(&v)),
            VerifyDecision::Valid
        );
        // tampered frame: Red even with a pubkey
        assert!(matches!(
            verify_decision(&signed, &[b"wrong"], Some(&v)),
            VerifyDecision::Red(_)
        ));
        std::fs::remove_file(std::env::temp_dir().join("rise-dec.pem").to_str().unwrap()).unwrap();
        std::fs::remove_file(std::env::temp_dir().join("rise-dec.pub.pem").to_str().unwrap()).unwrap();
    }
    #[test]
    fn stamp_then_sign_recorded_token_roundtrip() {
        // production order: stamp BEFORE sign so the signature covers
        // the token field. Uses a recorded FreeTSA token offline; the
        // recorded imprint belongs to its seed message, so no digest
        // check runs here — signature coverage is what this proves.
        use base64::Engine as _;
        let token_der: &[u8] = include_bytes!("../tests/vectors/freetsa_a.der");
        let token_b64 = base64::engine::general_purpose::STANDARD.encode(token_der);
        let frame = b"rise-stamp-then-sign-frame";
        let mut m = create_manifest_v1(&[&frame[..]], "d", 0, SigAlg::Es256, "silver");
        m.timestamp_token = Some(token_b64);
        let (priv_pem, pub_pem) = signer::generate_p256().unwrap();
        std::fs::write(std::env::temp_dir().join("rise-sts.pem").to_str().unwrap(), &priv_pem).unwrap();
        std::fs::write(std::env::temp_dir().join("rise-sts.pub.pem").to_str().unwrap(), &pub_pem).unwrap();
        let s = signer::P256Signer::from_pem_file(std::env::temp_dir().join("rise-sts.pem").to_str().unwrap()).unwrap();
        sign_manifest_v1(&mut m, &s).unwrap();
        let v = signer::P256Verifier::from_pem_file(std::env::temp_dir().join("rise-sts.pub.pem").to_str().unwrap()).unwrap();
        verify_signature(&m, &v).unwrap();
        assert_eq!(
            verify_decision(&m, &[&frame[..]], Some(&v)),
            VerifyDecision::Valid
        );
        // the fixed bug: stamping AFTER signing mutates the signed bytes,
        // so a post-sign token change must break verification
        let mut restamped = m.clone();
        restamped.timestamp_token = Some("AA==".to_string());
        assert!(verify_signature(&restamped, &v).is_err());
        std::fs::remove_file(std::env::temp_dir().join("rise-sts.pem").to_str().unwrap()).unwrap();
        std::fs::remove_file(std::env::temp_dir().join("rise-sts.pub.pem").to_str().unwrap()).unwrap();
    }
    #[test]
    #[ignore]
    fn live_stamp_then_sign_roundtrip() {
        // live FreeTSA stamp-then-sign over real canonical bytes
        use base64::Engine as _;
        let frame = b"rise-live-stamp-then-sign";
        let mut m = create_manifest_v1(&[&frame[..]], "d", 0, SigAlg::Es256, "silver");
        let canon = manifest_to_json(&m).unwrap();
        let digest = timestamp::sha256_of(canon.as_bytes());
        let tok =
            timestamp::request_timestamp(timestamp::DEFAULT_TSA_URL, canon.as_bytes()).unwrap();
        timestamp::verify_token_against_digest(&tok, &digest).unwrap();
        m.timestamp_token = Some(base64::engine::general_purpose::STANDARD.encode(&tok));
        let (priv_pem, pub_pem) = signer::generate_p256().unwrap();
        std::fs::write(std::env::temp_dir().join("rise-sts-live.pem").to_str().unwrap(), &priv_pem).unwrap();
        std::fs::write(std::env::temp_dir().join("rise-sts-live.pub.pem").to_str().unwrap(), &pub_pem).unwrap();
        let s = signer::P256Signer::from_pem_file(std::env::temp_dir().join("rise-sts-live.pem").to_str().unwrap()).unwrap();
        sign_manifest_v1(&mut m, &s).unwrap();
        let v = signer::P256Verifier::from_pem_file(std::env::temp_dir().join("rise-sts-live.pub.pem").to_str().unwrap()).unwrap();
        verify_signature(&m, &v).unwrap();
        std::fs::remove_file(std::env::temp_dir().join("rise-sts-live.pem").to_str().unwrap()).unwrap();
        std::fs::remove_file(std::env::temp_dir().join("rise-sts-live.pub.pem").to_str().unwrap()).unwrap();
    }
}
