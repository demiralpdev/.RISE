"""MS1 verify-web MVP: badge decision tree (Phase 1). Fail-closed, no file persistence."""
import hashlib
import json
from pathlib import Path

GOLD_CAP = "silver"  # MS1 cap: no gold-L4
REASON_MAX = 120

KEY_REGISTRY_PATH = Path(__file__).resolve().parent / "static" / "keys.json"


def _registry_entries(path=None) -> list:
    """Raw pinned registry entries; [] on ANY problem (fail-closed)."""
    try:
        p = Path(path) if path else KEY_REGISTRY_PATH
        doc = json.loads(p.read_text())
        if not isinstance(doc, dict):
            return []
        entries = doc.get("keys")
        return entries if isinstance(entries, list) else []
    except Exception:
        return []


def load_key_registry(path=None) -> dict[str, str]:
    """Load the pinned device-key registry (device_id -> SPKI PEM).

    Fail-closed: returns {} on ANY problem (missing file, bad JSON, wrong
    type, missing "keys"). Never raises, never logs secrets.
    """
    out: dict[str, str] = {}
    for e in _registry_entries(path):
        if not isinstance(e, dict):
            continue
        did = e.get("device_id")
        pem = e.get("pem")
        if isinstance(did, str) and did and isinstance(pem, str) and pem:
            out[did] = pem
    return out


# Backing classes that count as hardware. Anything else — including a missing
# registry entry — is software, so a self-declared claim inside the
# attacker-supplied manifest can never raise a tier.
_HARDWARE_BACKINGS = frozenset({"tpm", "secure_enclave", "tee", "strongbox"})


def _entry_is_hardware(e: dict) -> bool:
    if e.get("hardware_backed") is True:
        return True
    b = e.get("backing")
    return isinstance(b, str) and b.strip().lower() in _HARDWARE_BACKINGS


def hardware_backing(device_id, path=None) -> str:
    """'hardware' only when the PINNED registry entry declares it, else 'software'."""
    if not isinstance(device_id, str) or not device_id:
        return "software"
    for e in _registry_entries(path):
        if isinstance(e, dict) and e.get("device_id") == device_id:
            return "hardware" if _entry_is_hardware(e) else "software"
    return "software"


def resolve_pubkey(manifest, path=None) -> str | None:
    """Resolve a pinned SPKI PEM for manifest["device_id"]. None when absent/unmatched."""
    try:
        m = json.loads(manifest) if isinstance(manifest, str) else manifest
        if not isinstance(m, dict):
            return None
        did = m.get("device_id")
        if not isinstance(did, str) or not did:
            return None
        return load_key_registry(path).get(did)
    except Exception:
        return None

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


# Non-camera capture classes: the seal proves the bytes were stamped, NOT that a
# real camera sensor produced the scene. Each maps to an explicit disclaimer so
# no surface can present a virtual/screen/external seal as camera evidence.
# Values mirror core's --capture-source (camera|screen|obs|virtual|external_usb).
_NON_CAMERA_SOURCES = {
    "screen": (
        "Screen capture: the capture is attested, but the screen "
        "content's origin (camera, edited, or AI-generated) is NOT verified."
    ),
    "obs": (
        "Software/virtual-camera capture (OBS): the recording is attested, "
        "but it does NOT prove the pixels came from a real camera sensor."
    ),
    "virtual": (
        "Virtual camera: the frames come from a software device, not a real "
        "camera sensor; this is not camera evidence."
    ),
    "external_usb": (
        "External USB camera: the capture is attested, but an external source "
        "can be fed arbitrary video; camera evidence is capped."
    ),
}


def _ok(reason: str, m: dict) -> dict:
    """Success result; non-camera captures carry an explicit origin disclaimer.

    A non-camera capture (screen/obs/virtual/external_usb) attests that the
    bytes are a faithful capture at seal time — it does NOT attest that a real
    camera sensor produced the scene (it may be edited or AI-generated). The
    label and note ride along on every success result so no UI can overclaim.
    """
    out = {"badge": GOLD_CAP, "reason": reason[:REASON_MAX]}
    src = m.get("capture_source")
    if isinstance(src, str) and src:
        out["capture_source"] = src
        note = _NON_CAMERA_SOURCES.get(src.strip().lower())
        if note:
            out["capture_note"] = note
    return out


def _canonical_bytes(manifest: dict) -> bytes:
    ordered: dict = {}
    for key in _CANONICAL_ORDER:
        if key == "signature":
            continue
        if key in ("pqc_alg", "pqc_signature"):
            continue
        if key == "assurance":
            ordered[key] = manifest.get(key, "silver")
        elif key == "timestamp_token":
            # Rust serializes this field even when None (no skip_serializing_if):
            # must always emit "timestamp_token":null to match the Rust bytes.
            ordered[key] = manifest.get(key)
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


def _malformed_error(m: dict) -> dict | None:
    if not isinstance(m.get("frame_hashes"), list):
        return _red("Malformed frame hashes, rejected.")
    if not isinstance(m.get("tile_hashes"), list):
        return _red("Malformed tile hashes, rejected.")
    if not isinstance(m.get("merkle_root"), str):
        return _red("Malformed merkle root, rejected.")
    return None


def _content_error(m: dict, frame: bytes) -> dict | None:
    tiles = [_sha256(t) for t in _tiles(frame)]
    if frame and _sha256(frame) not in m.get("frame_hashes"):
        return _red("Frame hash mismatch, content changed.")
    if frame and tiles != m.get("tile_hashes"):
        return _red("Tile hashes mismatch, content changed.")
    if _merkle(tiles) != m.get("merkle_root"):
        return _red("Chain root mismatch, content changed.")
    return None


def _gold_key_is_hardware(m: dict) -> bool:
    return hardware_backing(m.get("device_id")) == "hardware"


def _gold_needs_hardware(m: dict) -> dict:
    """Gold requires a hardware-backed key from the pinned registry, never the manifest's own claim."""
    r = _ok("Gold needs a hardware-backed key; this key is software-backed.", m)
    r["key_verified"] = False
    r["key_source"] = None
    r["hardware_backing"] = hardware_backing(m.get("device_id"))
    return r


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
        malformed = _malformed_error(m)
        if malformed is not None:
            return malformed
        content = _content_error(m, frame)
        if content is not None:
            return content
        if replay:
            return _red("Nonce replay suspicion, unverified.")
        if signer_pub:
            if not verify_signature(m, signer_pub):
                r = _red("Signature cryptographically invalid.")
                r["key_verified"] = False
                r["key_source"] = "caller"
                return r
            r = _ok("Signature verified (ES256), chain valid.", m)
            r["key_verified"] = True
            r["key_source"] = "caller"
            return r
        reg_pub = resolve_pubkey(m)
        if reg_pub:
            if not verify_signature(m, reg_pub):
                r = _red("Signature cryptographically invalid.")
                r["key_verified"] = False
                r["key_source"] = "registry"
                return r
            r = _ok("Signature verified against pinned device key.", m)
            r["key_verified"] = True
            r["key_source"] = "registry"
            r["hardware_backing"] = hardware_backing(m.get("device_id"))
            return r
        tier = m.get("assurance", "silver")
        if tier not in ("gold-L4", "gold-L2", "silver"):
            r = _red("Unknown assurance tier.")
            r["key_verified"] = False
            r["key_source"] = None
            return r
        if tier == "gold-L4" and not strong_signal:
            r = _ok("MVP: gold-L4 is not issued, chain unverified.", m)
            r["key_verified"] = False
            r["key_source"] = None
            return r
        if tier == "gold-L2" and not strong_signal:
            r = _ok("No strong signal, downgraded one tier.", m)
            r["key_verified"] = False
            r["key_source"] = None
            return r
        if tier in ("gold-L4", "gold-L2") and not _gold_key_is_hardware(m):
            return _gold_needs_hardware(m)
        r = _ok("Structure valid, signature NOT crypto-verified (MS1), informational.", m)
        r["key_verified"] = False
        r["key_source"] = None
        return r
    except (json.JSONDecodeError, TypeError):
        return _red("Manifest unreadable, unverified.")


def verify_id(frame_hash: str) -> str:
    """Hash-id link identity without file persistence."""
    return _sha256(frame_hash.encode())[:16]
