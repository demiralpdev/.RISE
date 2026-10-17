"""MS1 badge decision tree tests — 8 cases, PLAN.md Phase 1 acceptance list."""
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from verify import GOLD_CAP, _merkle, verify_badge, verify_id


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


def test_screen_capture_labeled():
    # screen grab: capture attested, screen content origin explicitly NOT verified
    m = _valid_manifest(b"screen-frame", capture_source="screen")
    r = verify_badge(m, b"screen-frame")
    assert r["badge"] == GOLD_CAP
    assert r["capture_source"] == "screen"
    assert "NOT verified" in r.get("capture_note", "")


def test_plain_manifest_has_no_capture_fields():
    # camera/file seals (no capture_source) never grow capture fields
    r = verify_badge(_valid_manifest(b"plain-frame"), b"plain-frame")
    assert "capture_source" not in r
    assert "capture_note" not in r


def test_obs_capture_labeled_not_camera():
    # OBS/virtual-camera seal: attested, but never presented as camera evidence
    m = _valid_manifest(b"obs-frame", capture_source="obs")
    r = verify_badge(m, b"obs-frame")
    assert r["badge"] == GOLD_CAP
    assert r["capture_source"] == "obs"
    assert "NOT prove" in r.get("capture_note", "")


def test_virtual_and_external_usb_labeled():
    # virtual + external_usb seals carry an origin disclaimer too
    for src in ("virtual", "external_usb"):
        r = verify_badge(_valid_manifest(b"x", capture_source=src), b"x")
        assert r["capture_source"] == src
        assert r.get("capture_note")


def test_non_camera_source_set_matches_core():
    # drift guard: Python/JS/Rust must agree on which classes are not a camera
    from verify import _NON_CAMERA_SOURCES

    assert set(_NON_CAMERA_SOURCES) == {"screen", "obs", "virtual", "external_usb"}
    assert "camera" not in _NON_CAMERA_SOURCES


def _pin_registry(monkeypatch, tmp_path, entries):
    import verify

    p = tmp_path / "keys.json"
    p.write_text(json.dumps({"keys": entries}))
    monkeypatch.setattr(verify, "KEY_REGISTRY_PATH", p)


def test_hardware_backing_requires_pinned_declaration(monkeypatch, tmp_path):
    from verify import hardware_backing

    _pin_registry(monkeypatch, tmp_path, [{"device_id": "hw-device", "backing": "tpm"}])
    assert hardware_backing("hw-device") == "hardware"


def test_hardware_backing_accepts_full_vocabulary(monkeypatch, tmp_path):
    from verify import hardware_backing

    for i, backing in enumerate(("tpm", "secure_enclave", "tee", "strongbox", "TPM ")):
        _pin_registry(monkeypatch, tmp_path, [{"device_id": f"d{i}", "backing": backing}])
        assert hardware_backing(f"d{i}") == "hardware"


def test_missing_or_undeclared_entry_is_software(monkeypatch, tmp_path):
    from verify import hardware_backing

    _pin_registry(monkeypatch, tmp_path, [{"device_id": "plain-device", "pem": "x"}])
    assert hardware_backing("plain-device") == "software"
    assert hardware_backing("absent-device") == "software"
    assert hardware_backing(None) == "software"


def test_manifest_self_claim_cannot_raise_backing(monkeypatch, tmp_path):
    # an attacker-supplied manifest claiming hardware must not raise the tier
    _pin_registry(monkeypatch, tmp_path, [])
    m = _valid_manifest(
        b"gold-frame",
        assurance="gold-L2",
        hardware_backed=True,
        backing="secure_enclave",
    )
    r = verify_badge(m, b"gold-frame", strong_signal=True)
    assert r["badge"] == "silver"
    assert r["hardware_backing"] == "software"
    assert "hardware-backed key" in r["reason"]
