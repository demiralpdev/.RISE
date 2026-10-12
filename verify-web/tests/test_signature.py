"""ES256 signature verification tests — real P-256 keypair via `cryptography`."""
import base64
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from app import app
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import (
    decode_dss_signature,
)
from cryptography.hazmat.primitives.serialization import (
    Encoding,
    PublicFormat,
)
from fastapi.testclient import TestClient
from verify import _canonical_bytes, verify_badge, verify_signature

client = TestClient(app)
FRAME = b"frame-data-es256"


def _keypair():
    priv = ec.generate_private_key(ec.SECP256R1())
    pub_pem = (
        priv.public_key()
        .public_bytes(Encoding.PEM, PublicFormat.SubjectPublicKeyInfo)
        .decode()
    )
    return priv, pub_pem


def _chain(frame: bytes = FRAME) -> dict:
    n = min(len(frame), 4)
    size = -(-len(frame) // n)
    tiles = [hashlib.sha256(frame[i:i + size]).hexdigest() for i in range(0, len(frame), size)]
    from verify import _merkle
    return {
        "rise_version": 1,
        "frame_hashes": [hashlib.sha256(frame).hexdigest()],
        "tile_hashes": tiles,
        "merkle_root": _merkle(tiles),
        "sig_alg": "ES256",
        "timestamp_ms": 123,
        "device_id": "test-device",
        "assurance": "silver",
        "signing_key_id": "p256-soft",
    }


def _sign_manifest(manifest: dict, priv) -> str:
    canon = _canonical_bytes(manifest)
    der = priv.sign(canon, ec.ECDSA(hashes.SHA256()))
    r, s = decode_dss_signature(der)
    raw = r.to_bytes(32, "big") + s.to_bytes(32, "big")
    signed = dict(manifest)
    signed["signature"] = raw.hex()
    return json.dumps(signed)


def test_canonical_matches_rust_serde_layout():
    m = {
        "rise_version": 1,
        "frame_hashes": ["aa"],
        "tile_hashes": ["bb"],
        "merkle_root": "cc",
        "sig_alg": "ES256",
        "timestamp_ms": 123,
        "device_id": "test-device",
        "assurance": "silver",
        "signature": "deadbeef",
        "signing_key_id": "p256-soft",
    }
    assert _canonical_bytes(m) == (
        b'{"rise_version":1,"frame_hashes":["aa"],"tile_hashes":["bb"],'
        b'"merkle_root":"cc","sig_alg":"ES256","timestamp_ms":123,'
        b'"device_id":"test-device","assurance":"silver",'
        b'"signing_key_id":"p256-soft"}'
    )


def test_verify_signature_valid():
    priv, pub_pem = _keypair()
    raw = _sign_manifest(_chain(), priv)
    assert verify_signature(json.loads(raw), pub_pem) is True


def test_verify_signature_accepts_json_string():
    priv, pub_pem = _keypair()
    raw = _sign_manifest(_chain(), priv)
    assert verify_signature(raw, pub_pem) is True


def test_verify_signature_tampered_fails():
    priv, pub_pem = _keypair()
    raw = _sign_manifest(_chain(), priv)
    m = json.loads(raw)
    m["device_id"] = "attacker"
    assert verify_signature(m, pub_pem) is False


def test_verify_signature_wrong_key_fails():
    priv, _ = _keypair()
    _, other_pub = _keypair()
    raw = _sign_manifest(_chain(), priv)
    assert verify_signature(json.loads(raw), other_pub) is False


def test_verify_signature_malformed_sig_fails():
    _, pub_pem = _keypair()
    m = _chain()
    m["signature"] = "zzzz-not-hex"
    assert verify_signature(m, pub_pem) is False
    m["signature"] = "abcd"
    assert verify_signature(m, pub_pem) is False


def test_badge_silver_on_valid_signature():
    priv, pub_pem = _keypair()
    raw = _sign_manifest(_chain(), priv)
    r = verify_badge(raw, FRAME, signer_pub=pub_pem)
    assert r["badge"] == "silver"


def test_badge_red_on_invalid_signature():
    _, pub_pem = _keypair()
    other_priv, _ = _keypair()
    raw = _sign_manifest(_chain(), other_priv)
    r = verify_badge(raw, FRAME, signer_pub=pub_pem)
    assert r["badge"] == "red"
    assert r["reason"] == "Signature cryptographically invalid."


def test_badge_no_signer_pub_unchanged():
    priv, _ = _keypair()
    raw = _sign_manifest(_chain(), priv)
    r = verify_badge(raw, FRAME)
    assert r["badge"] == "silver"


def test_endpoint_silver_with_valid_signature(monkeypatch):
    import app as app_module

    monkeypatch.setattr(
        app_module, "check_trust_list", lambda *a, **k: ("trusted", "Issuer pinned (fixture).")
    )
    priv, pub_pem = _keypair()
    raw = _sign_manifest(_chain(), priv)
    r = client.post(
        "/api/v1/verify",
        json={
            "manifest": raw,
            "frame_b64": base64.b64encode(FRAME).decode(),
            "signer_pub": pub_pem,
        },
    ).json()
    assert r["badge"] == "silver"


def test_endpoint_red_with_invalid_signature():
    priv, _ = _keypair()
    _, other_pub = _keypair()
    raw = _sign_manifest(_chain(), priv)
    r = client.post(
        "/api/v1/verify",
        json={
            "manifest": raw,
            "frame_b64": base64.b64encode(FRAME).decode(),
            "signer_pub": other_pub,
        },
    ).json()
    assert r["badge"] == "red"
    assert r["reason"] == "Signature cryptographically invalid."
