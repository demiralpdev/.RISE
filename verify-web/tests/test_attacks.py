"""T-501 attack tests (threat-model sections 1/2/4): fake-file, re-capture, edit, replay.

All vectors are recorded/offline (no network). Every test asserts the badge
stays red/unknown and never reaches silver or higher (fail-closed).
"""
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from trust import check_trust_list  # noqa: E402
from verify import _merkle, _sha256, _tiles, verify_badge  # noqa: E402

ALLOWED = ("red", "unknown")


def _manifest(frame: bytes, **extra) -> str:
    """Builds a well-formed signed manifest for frame using verify.py helpers."""
    tile_hashes = [_sha256(t) for t in _tiles(frame)]
    m = {
        "rise_version": 1,
        "frame_hashes": [hashlib.sha256(frame).hexdigest()],
        "tile_hashes": tile_hashes,
        "merkle_root": _merkle(tile_hashes),
        "sig_alg": "ES256",
        "signature": "test-signature",
        "timestamp_ms": 123,
        "device_id": "test-device",
    }
    m.update(extra)
    return json.dumps(m)


def test_fake_file_self_signed_unknown_ca():
    """Threat-model S1: self-signed manifest from an unknown CA is not evidence."""
    status, _ = check_trust_list("attacker-self", "attacker-self")
    assert status in ("untrusted", "unknown")
    assert status != "trusted"
    status2, _ = check_trust_list("fake-leaf", "unknown-ca-xyz")
    assert status2 in ("untrusted", "unknown")
    # Fake file carries no verifiable signature -> badge stays red.
    unsigned = json.dumps({"rise_version": 1, "frame_hashes": [], "sig_alg": "ES256"})
    badge = verify_badge(unsigned, b"fake-frame-bytes")["badge"]
    assert badge in ALLOWED
    assert badge not in ("silver", "gold-L4", "gold-L2")


def test_screen_recapture_hash_mismatch():
    """Threat-model S2: screen re-capture yields different bytes -> hash mismatch."""
    original = b"original-camera-frame-bytes-001"
    recaptured = b"screen-recapture-of-same-scene-002"
    assert hashlib.sha256(original).hexdigest() != hashlib.sha256(recaptured).hexdigest()
    manifest = _manifest(original)
    result = verify_badge(manifest, recaptured)
    assert result["badge"] in ALLOWED
    assert result["badge"] not in ("silver", "gold-L4", "gold-L2")


def test_edit_single_tile_byte_flipped():
    """Threat-model S4: flipping one tile byte breaks the tile/merkle chain."""
    frame = b"editable-frame-bytes-0123456789"
    manifest = _manifest(frame)
    tampered = bytearray(frame)
    tampered[0] ^= 0xFF
    result = verify_badge(manifest, bytes(tampered))
    assert result["badge"] in ALLOWED
    assert result["badge"] not in ("silver", "gold-L4", "gold-L2")


def test_replay_same_manifest_twice():
    """Threat-model S1: resubmitting the same manifest with replay=True is rejected."""
    frame = b"replayable-frame-bytes-000"
    manifest = _manifest(frame)
    first = verify_badge(manifest, frame)
    assert first["badge"] in ALLOWED or first["badge"] == "silver"
    second = verify_badge(manifest, frame, replay=True)
    assert second["badge"] in ALLOWED
    assert second["badge"] not in ("silver", "gold-L4", "gold-L2")


def test_stripped_manifest_unsigned_recapture():
    """Defense in depth: stripped/screenshot path (UNSIGNED prefix) never verifies."""
    frame = b"screenshotted-frame-no-metadata"
    manifest = _manifest(frame)
    result = verify_badge("UNSIGNED:" + manifest, frame)
    assert result["badge"] in ALLOWED
    assert result["badge"] not in ("silver", "gold-L4", "gold-L2")
