"""Device-key registry tests — pinned device keys verify without a caller key.

The KAT fixture (tests/vectors) was signed by `rise-core sign` for
device_id "audit-device"; static/keys.json pins exactly that SPKI PEM.
"""
import base64
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from app import _apply_trust, app
from fastapi.testclient import TestClient
from verify import load_key_registry, resolve_pubkey, verify_badge

VECTORS = Path(__file__).resolve().parent / "vectors"
client = TestClient(app)


def _vector() -> tuple[str, bytes]:
    raw = (VECTORS / "manifest.json").read_text()
    frame = (VECTORS / "frame.bin").read_bytes()
    return raw, frame


def test_registry_positive_silver_and_not_demoted():
    raw, frame = _vector()
    r = verify_badge(raw, frame)
    assert r["badge"] == "silver"
    assert r["key_verified"] is True
    assert r["key_source"] == "registry"
    t = _apply_trust(r["badge"], r["reason"], raw,
                     key_verified=r["key_verified"], key_source=r["key_source"])
    assert t["badge"] == "silver"


def test_registry_positive_endpoint_not_demoted():
    raw, frame = _vector()
    r = client.post("/api/v1/verify", json={
        "manifest": raw,
        "frame_b64": base64.b64encode(frame).decode(),
    }).json()
    assert r["badge"] == "silver"
    assert r.get("key_verified") is True


def test_resolve_pubkey_unknown_device_none():
    assert resolve_pubkey({"device_id": "no-such-device"}) is None
    assert resolve_pubkey({}) is None
    assert resolve_pubkey({"device_id": 123}) is None


def test_tampered_signature_red():
    raw, frame = _vector()
    m = json.loads(raw)
    last = "0" if m["signature"][-1] != "0" else "1"
    m["signature"] = m["signature"][:-1] + last
    r = verify_badge(json.dumps(m), frame)
    assert r["badge"] == "red"
    assert r["reason"] == "Signature cryptographically invalid."


def test_tampered_frame_red():
    raw, frame = _vector()
    bad = bytes(b ^ 0xFF for b in frame[:1]) + frame[1:]
    assert verify_badge(raw, bad)["badge"] == "red"


def test_missing_corrupt_registry_fail_closed(tmp_path):
    assert load_key_registry(path="/nonexistent/keys.json") == {}
    bad = tmp_path / "bad.json"
    bad.write_text("{not json")
    assert load_key_registry(path=str(bad)) == {}
    wrong = tmp_path / "wrong.json"
    wrong.write_text(json.dumps({"note": "no keys here"}))
    assert load_key_registry(path=str(wrong)) == {}
    lst = tmp_path / "list.json"
    lst.write_text(json.dumps([1, 2, 3]))
    assert load_key_registry(path=str(lst)) == {}
    assert resolve_pubkey({"device_id": "audit-device"},
                          path="/nonexistent/keys.json") is None


def test_caller_key_still_marks_caller():
    raw, frame = _vector()
    pubkey = (VECTORS / "pubkey.pem").read_text()
    r = verify_badge(raw, frame, signer_pub=pubkey)
    assert r["badge"] == "silver"
    assert r["key_verified"] is True
    assert r["key_source"] == "caller"
