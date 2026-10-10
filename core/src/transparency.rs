//! Sigstore Rekor v2 transparency client (MS3: R-203).
//!
//! Live choice, probed 2026-10-10 against the public Sigstore instance:
//! the v1 endpoints on `rekor.sigstore.dev` are gone (404), the active
//! log is the yearly tile shard `https://log2025-1.rekor.sigstore.dev`
//! (Sigstore rotates shards ~yearly; read the current URL from the
//! TUF SigningConfig instead of hardcoding it elsewhere). The only write
//! endpoint is `POST /api/v2/log/entries` with a `hashedRekordRequestV002`
//! body: base64(SHA256(artifact)) + base64(signature) + base64(DER
//! verifier key) + keyDetails. A hash-only entry is rejected by the live
//! API ("invalid type, must be hashedrekord"), so submission always
//! carries the manifest signature. Rekor gives inclusion proof, NOT a
//! time proof (PLAN.md G2 red line); the checkpoint envelope is parsed
//! and returned for the caller to persist.

use crate::RiseError;
use std::time::Duration;

/// Active Rekor v2 tile shard (rotates yearly; see module docs).
pub const DEFAULT_REKOR_URL: &str = "https://log2025-1.rekor.sigstore.dev";
/// Sigstore keyDetails for P-256 (live-probed with HTTP 201).
pub const KEY_DETAILS_P256: &str = "PKIX_ECDSA_P256_SHA_256";
/// Sigstore keyDetails for Ed25519 (enum value per Sigstore docs;
/// P-256 only was live-probed).
pub const KEY_DETAILS_ED25519: &str = "PKIX_ED25519";

/// Parsed Rekor inclusion response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RekorEntry {
    /// Log index assigned by the shard.
    pub log_index: String,
    /// Raw C2SP checkpoint envelope (contains tree size + root hash).
    pub checkpoint: String,
    /// Artifact digest echoed back in the canonicalized body (base64).
    pub digest_b64: String,
}

/// Builds the Rekor v2 `hashedRekordRequestV002` submit body.
pub fn build_submit_body(
    digest_b64: &str,
    signature_b64: &str,
    pubkey_der_b64: &str,
    key_details: &str,
) -> serde_json::Value {
    serde_json::json!({
        "hashedRekordRequestV002": {
            "digest": digest_b64,
            "signature": {
                "content": signature_b64,
                "verifier": {
                    "publicKey": { "rawBytes": pubkey_der_b64 },
                    "keyDetails": key_details
                }
            }
        }
    })
}

/// Parses a Rekor v2 `TransparencyLogEntry` response: requires the
/// hashedrekord kind, a non-empty checkpoint envelope, and the echoed
/// canonicalized-body digest.
pub fn parse_submit_response(body: &[u8]) -> Result<RekorEntry, RiseError> {
    let bad = |why: &str| RiseError::Crypto(format!("rekor: {why}"));
    let v: serde_json::Value = serde_json::from_slice(body).map_err(|_| bad("bad json"))?;
    let log_index = v
        .get("logIndex")
        .and_then(|x| x.as_str())
        .ok_or_else(|| bad("missing logIndex"))?
        .to_string();
    let checkpoint = v
        .get("inclusionProof")
        .and_then(|x| x.get("checkpoint"))
        .and_then(|x| x.get("envelope"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| bad("missing checkpoint"))?;
    if checkpoint.is_empty() {
        return Err(bad("empty checkpoint"));
    }
    let canon_b64 = v
        .get("canonicalizedBody")
        .and_then(|x| x.as_str())
        .ok_or_else(|| bad("missing canonicalizedBody"))?;
    let canon_raw = base64_decode(canon_b64).map_err(|_| bad("bad canonicalizedBody"))?;
    let canon: serde_json::Value =
        serde_json::from_slice(&canon_raw).map_err(|_| bad("bad canonicalizedBody json"))?;
    if canon.get("kind").and_then(|x| x.as_str()) != Some("hashedrekord") {
        return Err(bad("unexpected kind"));
    }
    let digest_b64 = canon
        .get("spec")
        .and_then(|x| x.get("hashedRekordV002"))
        .and_then(|x| x.get("data"))
        .and_then(|x| x.get("digest"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| bad("missing digest"))?
        .to_string();
    Ok(RekorEntry {
        log_index,
        checkpoint: checkpoint.to_string(),
        digest_b64,
    })
}

/// Confirms the log entry covers the expected artifact digest (base64).
pub fn verify_entry_digest(entry: &RekorEntry, expected_digest_b64: &str) -> Result<(), RiseError> {
    if entry.digest_b64 == expected_digest_b64 {
        Ok(())
    } else {
        Err(RiseError::Crypto("rekor: digest mismatch".to_string()))
    }
}

/// Standard-base64 decode helper (no padding surprises: strict alphabet).
pub fn base64_decode(s: &str) -> Result<Vec<u8>, RiseError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| RiseError::Crypto(format!("base64: {e}")))
}

/// Standard-base64 encode helper.
pub fn base64_encode(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}

/// Submits a hashedrekord entry (network path: failures map to
/// [`RiseError::Crypto`], never panics).
pub fn submit_hashed_rekord(
    base_url: &str,
    digest: &[u8; 32],
    signature: &[u8],
    pubkey_der: &[u8],
    key_details: &str,
) -> Result<RekorEntry, RiseError> {
    use base64::Engine;
    let body = build_submit_body(
        &base64::engine::general_purpose::STANDARD.encode(digest),
        &base64::engine::general_purpose::STANDARD.encode(signature),
        &base64::engine::general_purpose::STANDARD.encode(pubkey_der),
        key_details,
    );
    let raw = serde_json::to_vec(&body).map_err(RiseError::Json)?;
    let url = format!("{base_url}/api/v2/log/entries");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into();
    let mut resp = agent
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send(&raw[..])
        .map_err(|e| RiseError::Crypto(format!("rekor submit: {e}")))?;
    let out = resp
        .body_mut()
        .read_to_vec()
        .map_err(|e| RiseError::Crypto(format!("rekor read: {e}")))?;
    parse_submit_response(&out)
}

/// Fetches the latest C2SP checkpoint envelope (read-only).
pub fn fetch_checkpoint(base_url: &str) -> Result<String, RiseError> {
    let url = format!("{base_url}/api/v2/checkpoint");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let mut resp = agent
        .get(&url)
        .header("Accept", "text/plain")
        .call()
        .map_err(|e| RiseError::Crypto(format!("rekor checkpoint: {e}")))?;
    let out = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| RiseError::Crypto(format!("rekor read: {e}")))?;
    if out.contains("rekor.sigstore.dev") {
        Ok(out)
    } else {
        Err(RiseError::Crypto("rekor: bad checkpoint".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded live Rekor v2 response (throwaway probe artifact).
    const RECORDED: &[u8] = include_bytes!("../tests/vectors/rekor_entry.json");

    #[test]
    fn submit_body_shape() {
        // request wrapper + verifier block match the live-accepted schema
        let v = build_submit_body("d", "s", "k", KEY_DETAILS_P256);
        assert_eq!(v["hashedRekordRequestV002"]["digest"], "d");
        assert_eq!(v["hashedRekordRequestV002"]["signature"]["content"], "s");
        assert_eq!(
            v["hashedRekordRequestV002"]["signature"]["verifier"]["publicKey"]["rawBytes"],
            "k"
        );
        assert_eq!(
            v["hashedRekordRequestV002"]["signature"]["verifier"]["keyDetails"],
            "PKIX_ECDSA_P256_SHA_256"
        );
    }

    #[test]
    fn recorded_response_parses() {
        // log index, checkpoint envelope, and echoed digest all present
        let e = parse_submit_response(RECORDED).unwrap();
        assert_eq!(e.log_index, "146163166");
        assert!(e.checkpoint.contains("rekor.sigstore.dev"));
        assert!(!e.digest_b64.is_empty());
        assert!(verify_entry_digest(&e, &e.digest_b64.clone()).is_ok());
    }

    #[test]
    fn digest_mismatch_rejected() {
        let e = parse_submit_response(RECORDED).unwrap();
        assert!(verify_entry_digest(&e, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").is_err());
    }

    #[test]
    fn malformed_responses_rejected() {
        assert!(parse_submit_response(b"not json").is_err());
        assert!(parse_submit_response(b"{}").is_err());
        assert!(parse_submit_response(br#"{"logIndex":"1"}"#).is_err());
        // missing checkpoint envelope
        assert!(parse_submit_response(
            br#"{"logIndex":"1","inclusionProof":{},"canonicalizedBody":"e30="}"#
        )
        .is_err());
    }

    #[test]
    fn base64_helpers_roundtrip() {
        assert_eq!(base64_encode(b"rise"), "cmlzZQ==");
        assert_eq!(base64_decode("cmlzZQ==").unwrap(), b"rise");
        assert!(base64_decode("!!!").is_err());
    }

    /// Live checkpoint fetch (read-only; needs network, run with --ignored).
    #[test]
    #[ignore]
    fn live_checkpoint_fetch() {
        let cp = fetch_checkpoint(DEFAULT_REKOR_URL).unwrap();
        assert!(cp.contains("rekor.sigstore.dev"));
    }
}
