//! Signing and verification (MS2). Key material never gets logged.

use crate::SigAlg;
use ml_dsa::signature::SignatureEncoding as _;
use ml_dsa::Keypair as _;

/// Signing side of the split.
pub trait Signer {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, crate::RiseError>;
    fn alg(&self) -> SigAlg;
    fn key_id(&self) -> String;
}

/// Verification side of the split.
pub trait Verifier {
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), crate::RiseError>;
}

/// P-256 software signer; key loaded from a PKCS#8 PEM file.
pub struct P256Signer {
    key: p256::ecdsa::SigningKey,
}

impl P256Signer {
    pub fn from_pem_file(path: &str) -> Result<Self, crate::RiseError> {
        use p256::pkcs8::DecodePrivateKey;
        let pem = std::fs::read_to_string(path)?;
        let key = p256::ecdsa::SigningKey::from_pkcs8_pem(&pem)
            .map_err(|e| crate::RiseError::Crypto(format!("key load: {e}")))?;
        Ok(Self { key })
    }

    /// SPKI PEM of the matching public key.
    pub fn public_pem(&self) -> Result<String, crate::RiseError> {
        use p256::pkcs8::EncodePublicKey;
        let vk = p256::ecdsa::VerifyingKey::from(&self.key);
        vk.to_public_key_pem(p256::pkcs8::LineEnding::LF)
            .map_err(|e| crate::RiseError::Crypto(format!("pubkey encode: {e}")))
    }
}

impl Signer for P256Signer {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, crate::RiseError> {
        use p256::ecdsa::signature::Signer as _;
        // NOTE: explicit type guides ecdsa's generic Signer output (E0282 otherwise).
        let sig: p256::ecdsa::Signature = self.key.sign(message);
        Ok(sig.to_bytes().to_vec())
    }
    fn alg(&self) -> SigAlg {
        SigAlg::Es256
    }
    fn key_id(&self) -> String {
        "p256-soft".to_string()
    }
}

/// P-256 verifier; key loaded from a SPKI PEM file.
pub struct P256Verifier {
    key: p256::ecdsa::VerifyingKey,
}

impl P256Verifier {
    pub fn from_pem_file(path: &str) -> Result<Self, crate::RiseError> {
        use p256::pkcs8::DecodePublicKey;
        let pem = std::fs::read_to_string(path)?;
        let key = p256::ecdsa::VerifyingKey::from_public_key_pem(&pem)
            .map_err(|e| crate::RiseError::Crypto(format!("pubkey load: {e}")))?;
        Ok(Self { key })
    }
}

impl Verifier for P256Verifier {
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), crate::RiseError> {
        use p256::ecdsa::signature::Verifier as _;
        let sig = p256::ecdsa::Signature::from_slice(signature)
            .map_err(|e| crate::RiseError::Crypto(format!("sig parse: {e}")))?;
        self.key
            .verify(message, &sig)
            .map_err(|_| crate::RiseError::InvalidSignature)
    }
}

/// Ed25519 software signer; raw 32-byte seed file.
pub struct Ed25519Signer {
    key: ed25519_dalek::SigningKey,
}

impl Ed25519Signer {
    pub fn from_file(path: &str) -> Result<Self, crate::RiseError> {
        let seed = std::fs::read(path)?;
        let arr: [u8; 32] = seed
            .as_slice()
            .try_into()
            .map_err(|_| crate::RiseError::Crypto("ed25519 seed must be 32 bytes".into()))?;
        Ok(Self {
            key: ed25519_dalek::SigningKey::from_bytes(&arr),
        })
    }

    /// Raw 32-byte public key.
    pub fn public_raw(&self) -> Vec<u8> {
        self.key.verifying_key().to_bytes().to_vec()
    }
}

impl Signer for Ed25519Signer {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, crate::RiseError> {
        use ed25519_dalek::Signer as _;
        Ok(self.key.sign(message).to_bytes().to_vec())
    }
    fn alg(&self) -> SigAlg {
        SigAlg::Ed25519
    }
    fn key_id(&self) -> String {
        "ed25519-soft".to_string()
    }
}

/// Ed25519 verifier; raw 32-byte public key file.
pub struct Ed25519Verifier {
    key: ed25519_dalek::VerifyingKey,
}

impl Ed25519Verifier {
    pub fn from_file(path: &str) -> Result<Self, crate::RiseError> {
        let raw = std::fs::read(path)?;
        let arr: [u8; 32] = raw
            .as_slice()
            .try_into()
            .map_err(|_| crate::RiseError::Crypto("ed25519 pubkey must be 32 bytes".into()))?;
        let key = ed25519_dalek::VerifyingKey::from_bytes(&arr)
            .map_err(|e| crate::RiseError::Crypto(format!("pubkey load: {e}")))?;
        Ok(Self { key })
    }
}

impl Verifier for Ed25519Verifier {
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), crate::RiseError> {
        use ed25519_dalek::Verifier as _;
        let sig = ed25519_dalek::Signature::from_slice(signature)
            .map_err(|e| crate::RiseError::Crypto(format!("sig parse: {e}")))?;
        self.key
            .verify(message, &sig)
            .map_err(|_| crate::RiseError::InvalidSignature)
    }
}

/// Generates a P-256 keypair, returns (private PEM, public PEM).
pub fn generate_p256() -> Result<(String, String), crate::RiseError> {
    use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    use rand::TryRngCore;
    // fresh private scalar from the OS RNG
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|e| crate::RiseError::Crypto(format!("keygen rng: {e}")))?;
    let key = p256::ecdsa::SigningKey::from_bytes(&bytes.into())
        .map_err(|e| crate::RiseError::Crypto(format!("keygen: {e}")))?;
    let priv_pem = key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| crate::RiseError::Crypto(format!("keygen: {e}")))?
        .to_string();
    let vk = p256::ecdsa::VerifyingKey::from(&key);
    let pub_pem = vk
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| crate::RiseError::Crypto(format!("keygen: {e}")))?;
    Ok((priv_pem, pub_pem))
}

/// Generates an Ed25519 keypair, returns (seed, pubkey) raw bytes.
pub fn generate_ed25519() -> Result<(Vec<u8>, Vec<u8>), crate::RiseError> {
    use rand::TryRngCore;
    let mut seed = [0u8; 32];
    rand::rngs::OsRng
        .try_fill_bytes(&mut seed)
        .map_err(|e| crate::RiseError::Crypto(format!("keygen rng: {e}")))?;
    let key = ed25519_dalek::SigningKey::from_bytes(&seed);
    Ok((seed.to_vec(), key.verifying_key().to_bytes().to_vec()))
}

/// Rotation rule: a key older than 90 days is expired.
pub fn key_expired(created_ms: u64, now_ms: u64) -> bool {
    const ROTATION_MS: u64 = 90 * 24 * 3600 * 1000;
    now_ms.saturating_sub(created_ms) > ROTATION_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p256_sign_verify_roundtrip() {
        let (priv_pem, pub_pem) = generate_p256().unwrap();
        std::fs::write(
            std::env::temp_dir()
                .join("rise-test-p256.pem")
                .to_str()
                .unwrap(),
            &priv_pem,
        )
        .unwrap();
        std::fs::write(
            std::env::temp_dir()
                .join("rise-test-p256.pub.pem")
                .to_str()
                .unwrap(),
            &pub_pem,
        )
        .unwrap();
        let s = P256Signer::from_pem_file(
            std::env::temp_dir()
                .join("rise-test-p256.pem")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let v = P256Verifier::from_pem_file(
            std::env::temp_dir()
                .join("rise-test-p256.pub.pem")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let msg = b"rise-p256";
        let sig = s.sign(msg).unwrap();
        assert_eq!(s.alg(), SigAlg::Es256);
        v.verify(msg, &sig).unwrap();
        std::fs::remove_file(
            std::env::temp_dir()
                .join("rise-test-p256.pem")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        std::fs::remove_file(
            std::env::temp_dir()
                .join("rise-test-p256.pub.pem")
                .to_str()
                .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn ed25519_sign_verify_roundtrip() {
        let (seed, pubkey) = generate_ed25519().unwrap();
        std::fs::write(
            std::env::temp_dir()
                .join("rise-test-ed.seed")
                .to_str()
                .unwrap(),
            &seed,
        )
        .unwrap();
        std::fs::write(
            std::env::temp_dir()
                .join("rise-test-ed.pub")
                .to_str()
                .unwrap(),
            &pubkey,
        )
        .unwrap();
        let s = Ed25519Signer::from_file(
            std::env::temp_dir()
                .join("rise-test-ed.seed")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let v = Ed25519Verifier::from_file(
            std::env::temp_dir()
                .join("rise-test-ed.pub")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let msg = b"rise-ed25519";
        let sig = s.sign(msg).unwrap();
        assert_eq!(s.alg(), SigAlg::Ed25519);
        v.verify(msg, &sig).unwrap();
        std::fs::remove_file(
            std::env::temp_dir()
                .join("rise-test-ed.seed")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        std::fs::remove_file(
            std::env::temp_dir()
                .join("rise-test-ed.pub")
                .to_str()
                .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn tampered_message_fails() {
        let (seed, pubkey) = generate_ed25519().unwrap();
        std::fs::write(
            std::env::temp_dir()
                .join("rise-test-ed2.seed")
                .to_str()
                .unwrap(),
            &seed,
        )
        .unwrap();
        std::fs::write(
            std::env::temp_dir()
                .join("rise-test-ed2.pub")
                .to_str()
                .unwrap(),
            &pubkey,
        )
        .unwrap();
        let s = Ed25519Signer::from_file(
            std::env::temp_dir()
                .join("rise-test-ed2.seed")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let v = Ed25519Verifier::from_file(
            std::env::temp_dir()
                .join("rise-test-ed2.pub")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let sig = s.sign(b"original").unwrap();
        assert!(v.verify(b"tampered", &sig).is_err());
        std::fs::remove_file(
            std::env::temp_dir()
                .join("rise-test-ed2.seed")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        std::fs::remove_file(
            std::env::temp_dir()
                .join("rise-test-ed2.pub")
                .to_str()
                .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn mldsa65_sign_verify_roundtrip() {
        let (seed, pubkey) = MlDsa65Signer::generate().unwrap();
        let sp = std::env::temp_dir().join("rise-ml.seed");
        let pp = std::env::temp_dir().join("rise-ml.pub");
        std::fs::write(&sp, &seed).unwrap();
        std::fs::write(&pp, &pubkey).unwrap();
        let s = MlDsa65Signer::from_seed_file(sp.to_str().unwrap()).unwrap();
        let v = MlDsa65Verifier::from_pubkey_file(pp.to_str().unwrap()).unwrap();
        let sig = s.sign(b"rise-mldsa").unwrap();
        assert!(sig.len() > 3000); // ML-DSA-65 signature is ~3309 bytes
        v.verify(b"rise-mldsa", &sig).unwrap();
        std::fs::remove_file(sp).unwrap();
        std::fs::remove_file(pp).unwrap();
    }

    #[test]
    fn mldsa65_tampered_fails() {
        let (seed, pubkey) = MlDsa65Signer::generate().unwrap();
        let sp = std::env::temp_dir().join("rise-ml2.seed");
        let pp = std::env::temp_dir().join("rise-ml2.pub");
        std::fs::write(&sp, &seed).unwrap();
        std::fs::write(&pp, &pubkey).unwrap();
        let s = MlDsa65Signer::from_seed_file(sp.to_str().unwrap()).unwrap();
        let v = MlDsa65Verifier::from_pubkey_file(pp.to_str().unwrap()).unwrap();
        let sig = s.sign(b"original").unwrap();
        assert!(v.verify(b"tampered", &sig).is_err());
        std::fs::remove_file(sp).unwrap();
        std::fs::remove_file(pp).unwrap();
    }

    #[test]
    fn key_expired_after_90_days() {
        let now = 1_000_000_000_000u64;
        assert!(!key_expired(now - 89 * 24 * 3600 * 1000, now));
        assert!(key_expired(now - 91 * 24 * 3600 * 1000, now));
    }
}

/// ML-DSA-65 (FIPS 204) post-quantum signer — seed file = 32 raw bytes.
pub struct MlDsa65Signer {
    key: ml_dsa::SigningKey<ml_dsa::MlDsa65>,
}

impl MlDsa65Signer {
    pub fn from_seed_file(path: &str) -> Result<Self, crate::RiseError> {
        let seed = std::fs::read(path)?;
        let arr: [u8; 32] = seed
            .as_slice()
            .try_into()
            .map_err(|_| crate::RiseError::Crypto("ML-DSA seed must be 32 bytes".into()))?;
        let seed = ml_dsa::Seed::from(arr);
        Ok(Self {
            key: ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed),
        })
    }

    /// Encoded verification key (1952 bytes for ML-DSA-65).
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.key.verifying_key().encode().to_vec()
    }

    /// Deterministic ML-DSA-65 signature with an empty context.
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, crate::RiseError> {
        use ml_dsa::signature::Signer as _;
        Ok(self.key.sign(message).to_vec())
    }

    /// Generates (seed, verification-key) raw bytes.
    pub fn generate() -> Result<(Vec<u8>, Vec<u8>), crate::RiseError> {
        use rand::TryRngCore as _;
        let mut seed = [0u8; 32];
        rand::rngs::OsRng
            .try_fill_bytes(&mut seed)
            .map_err(|e| crate::RiseError::Crypto(format!("rng: {e}")))?;
        let key = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&ml_dsa::Seed::from(seed));
        Ok((seed.to_vec(), key.verifying_key().encode().to_vec()))
    }
}

/// ML-DSA-65 verifier — pubkey file = encoded verification key (1952 bytes).
pub struct MlDsa65Verifier {
    key: ml_dsa::VerifyingKey<ml_dsa::MlDsa65>,
}

impl MlDsa65Verifier {
    pub fn from_pubkey_file(path: &str) -> Result<Self, crate::RiseError> {
        let raw = std::fs::read(path)?;
        let arr: [u8; 1952] = raw
            .as_slice()
            .try_into()
            .map_err(|_| crate::RiseError::Crypto("ML-DSA-65 pubkey must be 1952 bytes".into()))?;
        let enc = ml_dsa::EncodedVerifyingKey::<ml_dsa::MlDsa65>::from(arr);
        Ok(Self {
            key: ml_dsa::VerifyingKey::<ml_dsa::MlDsa65>::decode(&enc),
        })
    }

    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), crate::RiseError> {
        let sig = ml_dsa::Signature::<ml_dsa::MlDsa65>::try_from(signature)
            .map_err(|e| crate::RiseError::Crypto(format!("sig parse: {e}")))?;
        if self.key.verify_with_context(message, &[], &sig) {
            Ok(())
        } else {
            Err(crate::RiseError::InvalidSignature)
        }
    }
}
