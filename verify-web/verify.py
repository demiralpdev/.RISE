"""MS1 verify-web MVP: badge decision tree (Phase 1). Fail-closed, no file persistence."""
import hashlib
import json

GOLD_CAP = "silver"  # MS1 cap: no gold-L4
REASON_MAX = 120


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


def verify_badge(
    raw_manifest: str, frame: bytes, replay: bool = False, strong_signal: bool = False
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
