"""MS1 verify-web MVP: badge decision tree (Phase 1). Fail-closed, no file persistence."""
import hashlib
import json

GOLD_CAP = "silver"  # MS1 cap: no gold-L4
REASON_MAX = 120

# Canonical ManifestV1 field order — must match Rust serde output in core/src/lib.rs:
# rise_version, frame_hashes, tile_hashes, merkle_root, sig_alg, timestamp_ms,
# device_id, assurance, timestamp_token, signature, signing_key_id (+ pqc/capture
# fields when present). serde_json::to_string is compact (no spaces).
_CANONICAL_ORDER = (
    "rise_version",
    "frame_hashes",
    "tile_hashes",
    "merkle_root",
    "sig_alg",
    "timestamp_ms",
    "device_id",
    "assurance",
    "timestamp_token",
    "signature",
    "signing_key_id",
    "pqc_alg",
    "pqc_signature",
    "capture_source",
)


def _sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _tiles(data: bytes) -> list[bytes]:
    if not data:
        return []
    n = min(len(data), 4)
    boy = -(-len(data) // n)
    return [data[i:i + boy] for i in range(0, len(data), boy)]


def _merkle(hashes: list[str]) -> str:
    if not hashes:
        return _sha256(b"")
    kat = list(hashes)
    while len(kat) > 1:
        ust = []
        for i in range(0, len(kat), 2):
            sag = kat[i + 1] if i + 1 < len(kat) else kat[i]
            ust.append(_sha256((kat[i] + sag).encode()))
        kat = ust
    return kat[0]


def _red(reason: str) -> dict:
    return {"badge": "red", "reason": reason[:REASON_MAX]}


def _canonical_bytes(manifest: dict) -> bytes:
    ordered: dict = {}
    for key in _CANONICAL_ORDER:
        if key == "signature":
            continue
        if key in ("pqc_alg", "pqc_signature"):
            continue
        if key == "assurance":
            ordered[key] = manifest.get(key, "silver")
        elif key in manifest and manifest[key] is not None:
            ordered[key] = manifest[key]
    return json.dumps(ordered, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def verify_signature(manifest, pubkey_pem: str) -> bool:
    try:
        from cryptography.hazmat.primitives import hashes
        from cryptography.hazmat.primitives.asymmetric import ec
        from cryptography.hazmat.primitives.asymmetric.utils import encode_dss_signature
        from cryptography.hazmat.primitives.serialization import load_pem_public_key
    except ImportError:
        return False
    try:
        m = json.loads(manifest) if isinstance(manifest, str) else manifest
        if not isinstance(m, dict):
            return False
        sig_hex = m.get("signature")
        if not isinstance(sig_hex, str) or not sig_hex:
            return False
        try:
            raw = bytes.fromhex(sig_hex)
        except ValueError:
            return False
        if len(raw) != 64:
            return False
        r = int.from_bytes(raw[:32], "big")
        s = int.from_bytes(raw[32:], "big")
        der_sig = encode_dss_signature(r, s)
        public_key = load_pem_public_key(
            pubkey_pem.encode("utf-8") if isinstance(pubkey_pem, str) else pubkey_pem
        )
        if not isinstance(public_key, ec.EllipticCurvePublicKey):
            return False
        if not isinstance(public_key.curve, ec.SECP256R1):
            return False
        public_key.verify(der_sig, _canonical_bytes(m), ec.ECDSA(hashes.SHA256()))
        return True
    # fail-closed: any verification error means the signature is invalid
    except Exception:
        return False


def verify_badge(
    raw_manifest: str, frame: bytes, replay: bool = False, strong_signal: bool = False,
    signer_pub: str | None = None,
) -> dict:
    """Returns {badge, reason}. Tiers: red/silver. Gold is closed in MS1."""
    try:
        text = raw_manifest
        # stub signature prefix: signature unverified -> red (fail-closed)
        if text.startswith("UNSIGNED:"):
            return _red("No signature, not evidence.")
        m = json.loads(text)
        if not m.get("signature"):
            return _red("No signature, not evidence.")
        if not m.get("sig_alg"):
            return _red("No signature algorithm, not evidence.")
        # MS1 manifest v1: frame_hashes list (type-strict, fail-closed)
        frame_hashes = m.get("frame_hashes")
        if not isinstance(frame_hashes, list):
            return _red("Malformed frame hashes, rejected.")
        tile_hashes_in = m.get("tile_hashes")
        if not isinstance(tile_hashes_in, list):
            return _red("Malformed tile hashes, rejected.")
        if not isinstance(m.get("merkle_root"), str):
            return _red("Malformed merkle root, rejected.")
        tiles = [_sha256(t) for t in _tiles(frame)]
        if frame and _sha256(frame) not in frame_hashes:
            return _red("Frame hash mismatch, content changed.")
        if frame and tiles != tile_hashes_in:
            return _red("Tile hashes mismatch, content changed.")
        if _merkle(tiles) != m.get("merkle_root"):
            return _red("Chain root mismatch, content changed.")
        if replay:
            return _red("Nonce replay suspicion, unverified.")
        if signer_pub:
            if not verify_signature(m, signer_pub):
                return _red("Signature cryptographically invalid.")
            return {"badge": GOLD_CAP, "reason": "Signature verified (ES256), chain valid."}
        tier = m.get("assurance", "silver")
        if tier not in ("gold-L4", "gold-L2", "silver"):
            return _red("Unknown assurance tier.")
        if tier == "gold-L4" and not strong_signal:
            return {"badge": GOLD_CAP, "reason": "MVP: gold-L4 is not issued, chain unverified."}
        if tier == "gold-L2" and not strong_signal:
            return {"badge": GOLD_CAP, "reason": "No strong signal, downgraded one tier."}
        return {"badge": GOLD_CAP, "reason": "Structure valid, signature NOT crypto-verified (MS1), informational."}
    except (json.JSONDecodeError, TypeError):
        return _red("Manifest unreadable, unverified.")


def verify_id(frame_hash: str) -> str:
    """Hash-id link identity without file persistence."""
    return _sha256(frame_hash.encode())[:16]
