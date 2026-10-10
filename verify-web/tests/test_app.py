"""app.py endpoint tests — TestClient only, no network."""
import base64
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from fastapi.testclient import TestClient  # noqa: E402

from app import app  # noqa: E402
from verify import _merkle  # noqa: E402

client = TestClient(app)

FRAME = b"frame-data"
ANCHOR = "test-anchor-ca"


def _manifest(frame: bytes = FRAME, **extra) -> str:
    n = min(len(frame), 4)
    size = -(-len(frame) // n)
    tiles = [hashlib.sha256(frame[i:i + size]).hexdigest() for i in range(0, len(frame), size)]
    m = {
        "rise_version": 1,
        "frame_hashes": [hashlib.sha256(frame).hexdigest()],
        "tile_hashes": tiles,
        "merkle_root": _merkle(tiles),
        "sig_alg": "ES256",
        "signature": "test-signature",
        "timestamp_ms": 123,
        "device_id": "test-device",
    }
    m.update(extra)
    return json.dumps(m)


def _b64(frame: bytes = FRAME) -> str:
    return base64.b64encode(frame).decode()


def test_health():
    assert client.get("/health").json() == {"ok": True}


def test_verify_trusted_keeps_silver(monkeypatch):
    # The prod trust list ships EMPTY anchors (red-team finding F1): a trusted
    # verdict is only reachable via a fixture-patched check_trust_list.
    import app as app_module

    monkeypatch.setattr(app_module, "check_trust_list",
                        lambda *a, **k: ("trusted", "Issuer pinned (fixture)."))
    m = _manifest(issuer=ANCHOR)
    r = client.post("/api/v1/verify", json={"manifest": m, "frame_b64": _b64()}).json()
    assert r["badge"] == "silver"
    assert r["trust"]["verdict"] == "trusted"
    assert r["verify_url"].startswith("/v/")


def test_prod_trust_list_has_no_test_anchor():
    # Locks the red-team F1 fix: the shipped trust list must not trust any
    # issuer by default (empty anchors until a verified upstream snapshot).
    import app as app_module

    assert not app_module.check_trust_list("leaf", ANCHOR)[0] == "trusted"


def test_verify_unknown_demotes_to_red():
    m = _manifest(issuer="unknown-ca-xyz")
    r = client.post("/api/v1/verify", json={"manifest": m, "frame_b64": _b64()}).json()
    assert r["badge"] == "red"
    assert r["trust"]["verdict"] == "unknown"


def test_verify_untrusted_self_signed_demotes():
    m = _manifest(device_id="self-signed-dev", issuer="self-signed-dev")
    r = client.post("/api/v1/verify", json={"manifest": m, "frame_b64": _b64()}).json()
    assert r["badge"] == "red"
    assert r["trust"]["verdict"] == "untrusted"


def test_verify_red_stays_red():
    m = _manifest(issuer=ANCHOR)
    r = client.post("/api/v1/verify", json={"manifest": m, "frame_b64": _b64(b"tampered")}).json()
    assert r["badge"] == "red"


def test_verify_link_roundtrip():
    m = _manifest(issuer=ANCHOR)
    r = client.post("/api/v1/verify", json={"manifest": m, "frame_b64": _b64()}).json()
    vid = r["verify_id"]
    link = client.get(f"/v/{vid}").json()
    assert link["verify_id"] == vid
    assert link["badge"] == r["badge"]


def test_verify_link_unknown():
    link = client.get("/v/no-such-id").json()
    assert link["badge"] == "unknown"


def test_report_unknown_or_unavailable():
    # report.py absent -> 501; present -> 404 for unknown id. Both acceptable.
    res = client.post("/api/v1/report", json={"verify_id": "no-such-id"})
    assert res.status_code in (404, 501)


def test_report_verified_or_unavailable():
    m = _manifest(issuer=ANCHOR)
    r = client.post("/api/v1/verify", json={"manifest": m, "frame_b64": _b64()}).json()
    res = client.post("/api/v1/report", json={"verify_id": r["verify_id"]})
    assert res.status_code in (200, 501)
    if res.status_code == 200:
        assert isinstance(res.json(), dict)
