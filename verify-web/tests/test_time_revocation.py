"""Live-path time-proof ceiling: single-TSA / no-OCSP can never exceed silver.

Locks the app._time_ceiling + trust.time_revocation_verdict wiring: unknown
verdicts CAP (gold-* clamps to silver, never demote), an untrusted verdict
demotes one step first (inert today - nothing returns untrusted yet), and the
ceiling is independent of GOLD_CAP so a future cap lift cannot silently go
green on a single TSA. Fail-closed on every malformed input.
"""
import base64
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from app import _time_ceiling, app
from fastapi.testclient import TestClient
from trust import UNKNOWN, check_ocsp, time_revocation_verdict
from verify import _merkle

client = TestClient(app)
VECTORS = Path(__file__).resolve().parent / "vectors"

_OK = {"timestamp": ("trusted", "x"), "tsa": ("trusted", "y"), "ocsp": ("trusted", "z")}


def _manifest(token=None, **extra) -> str:
    frame = b"frame-data"
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
        "timestamp_token": token,
    }
    m.update(extra)
    return json.dumps(m)


# --- time_revocation_verdict (pure) -----------------------------------------


def test_no_token_time_proof_missing():
    v = time_revocation_verdict(json.loads(_manifest(token=None)))
    assert v["timestamp"][0] == UNKNOWN
    assert v["tsa"][0] == UNKNOWN
    assert "missing" in v["timestamp"][1]


def test_token_present_single_tsa_insufficient():
    v = time_revocation_verdict(json.loads(_manifest(token="MIIb-token")))
    assert v["timestamp"][0] == UNKNOWN
    assert v["tsa"][0] == UNKNOWN
    assert "Single" in v["tsa"][1]


def test_verdicts_never_trusted_client_side():
    for token in (None, "", "tok"):
        v = time_revocation_verdict(json.loads(_manifest(token=token)))
        assert all(t[0] != "trusted" for t in v.values())


def test_malformed_manifest_never_raises():
    assert time_revocation_verdict({})["ocsp"][0] == UNKNOWN
    assert time_revocation_verdict(None)["tsa"][0] == UNKNOWN
    assert time_revocation_verdict("junk")["timestamp"][0] == UNKNOWN


def test_ocsp_wired_into_live_path():
    # check_ocsp is no longer dead code: its exact verdict rides on every result.
    v = time_revocation_verdict(json.loads(_manifest()))
    assert v["ocsp"] == tuple(check_ocsp())


# --- _time_ceiling (unit) ---------------------------------------------------


def test_ceiling_clamps_gold_to_silver():
    v = time_revocation_verdict(json.loads(_manifest(token="tok")))
    assert _time_ceiling("gold-L4", v) == "silver"
    assert _time_ceiling("gold-L2", v) == "silver"


def test_ceiling_never_touches_silver_or_red():
    v = time_revocation_verdict(json.loads(_manifest(token="tok")))
    assert _time_ceiling("silver", v) == "silver"
    assert _time_ceiling("red", v) == "red"


def test_ceiling_independent_of_gold_cap():
    # Even if GOLD_CAP were lifted someday, the gate must hold on its own.
    assert _time_ceiling("gold-L4", _OK) == "gold-L4"
    assert _time_ceiling("gold-L4", dict(_OK, ocsp=("unknown", "z"))) == "silver"


def test_ceiling_untrusted_demotes_then_caps():
    bad = {"timestamp": ("untrusted", "x"), "tsa": ("unknown", "y"), "ocsp": ("unknown", "z")}
    assert _time_ceiling("gold-L2", bad) == "silver"
    assert _time_ceiling("silver", bad) == "red"


def test_ceiling_bad_input_fails_closed():
    assert _time_ceiling("gold-L4", None) == "silver"
    assert _time_ceiling("gold-L4", {}) == "silver"


# --- live endpoint ----------------------------------------------------------


def test_api_response_carries_time_verdicts():
    # Registry-verified device key (item A1) keeps silver; the ceiling must not
    # demote it, and the honest unknown verdicts must ride on the response.
    r = client.post(
        "/api/v1/verify",
        json={
            "manifest": (VECTORS / "manifest.json").read_text(),
            "frame_b64": base64.b64encode((VECTORS / "frame.bin").read_bytes()).decode(),
        },
    ).json()
    assert r["badge"] == "silver"
    t = r["trust"]
    assert {"timestamp", "tsa", "ocsp"} <= set(t)
    assert t["timestamp"][0] == UNKNOWN  # the KAT vector has timestamp_token null
    assert t["tsa"][0] == UNKNOWN
    assert t["ocsp"][0] == UNKNOWN
