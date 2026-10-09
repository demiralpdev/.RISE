"""MS1 badge decision tree tests — 8 cases, PLAN.md Phase 1 acceptance list."""
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from verify import GOLD_CAP, _merkle, verify_badge, verify_id  # noqa: E402


def _valid_manifest(kare: bytes, **extra) -> str:
    """Builds a signed sample manifest matching the MS1 schema (real hash chain)."""
    tiles = []
    n = min(len(kare), 4)
    boy = -(-len(kare) // n)
    for i in range(0, len(kare), boy):
        tiles.append(hashlib.sha256(kare[i:i + boy]).hexdigest())
    m = {
        "rise_version": 1,
        "frame_hashes": [hashlib.sha256(kare).hexdigest()],
        "tile_hashes": tiles,
        "merkle_root": _merkle(tiles),
        "sig_alg": "ES256",
        "signature": "test-signature",
        "timestamp_ms": 123,
        "device_id": "test-device",
    }
    m.update(extra)
    return json.dumps(m)


def test_unsigned_red():
    # missing signature field -> red
    m = json.dumps({"rise_version": 1, "frame_hashes": [], "sig_alg": "ES256"})
    assert verify_badge(m, b"")["badge"] == "red"


def test_hash_mismatch_red():
    # tampered frame -> red
    m = _valid_manifest(b"frame-data")
    assert verify_badge(m, b"tampered-data")["badge"] == "red"


def test_unsigned_prefix_red():
    # stub prefix -> red
    assert verify_badge("UNSIGNED:" + _valid_manifest(b"x"), b"x")["badge"] == "red"


def test_replay_red():
    # nonce replay -> red
    m = _valid_manifest(b"frame-data")
    assert verify_badge(m, b"frame-data", replay=True)["badge"] == "red"


def test_gold_cap_silver():
    # gold-L4 claim without a strong signal -> gold never, silver cap
    m = _valid_manifest(b"frame-data", assurance="gold-L4")
    r = verify_badge(m, b"frame-data")
    assert r["badge"] == GOLD_CAP
    assert r["badge"] != "gold-L4"


def test_silver_pass():
    # clean chain + signature present -> silver
    m = _valid_manifest(b"frame-data")
    assert verify_badge(m, b"frame-data")["badge"] == GOLD_CAP


def test_broken_json_red():
    # corrupt JSON -> red
    assert verify_badge("broken-json{{", b"x")["badge"] == "red"


def test_verify_id_short():
    # hash-id link identity is 16 characters
    assert len(verify_id("abc")) == 16
