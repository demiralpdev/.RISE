//! RISE core logic — single shared source.
//! Formula: SHA256(raw frame) at shutter.
//! MS1: ManifestV1 schema (RISE-01 §5) + streaming hash.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use thiserror::Error;

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
        let m = create_manifest_v1(&[&k1[..], &k2[..], &k3[..]], "device-1", 999, SigAlg::Es256);
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
        let m = create_manifest_v1(&[&b"a"[..]], "test-device", 123, SigAlg::Ed25519);
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
        let m = create_manifest_v1(&[], "d", 0, SigAlg::Es256);
        assert!(m.frame_hashes.is_empty());
        assert_eq!(m.merkle_root, sha256_frame(b""));
        assert!(verify_manifest_v1(&m, &[]).is_ok());
    }
    #[test]
    fn sign_stub_stands() {
        // the stub prefix is preserved
        let cikti = sign_manifest("{}");
        assert!(cikti.starts_with("UNSIGNED:"));
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
}
