"""V-107 report tests — recorded vectors only, no network."""
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from report import DISCLAIMER, build_report, render_text  # noqa: E402
from verify import _merkle  # noqa: E402


def _manifest(kare: bytes, **extra) -> str:
    n = min(len(kare), 4)
    boy = -(-len(kare) // n)
    tiles = [hashlib.sha256(kare[i:i + boy]).hexdigest() for i in range(0, len(kare), boy)]
    m = {
        "frame_hashes": [hashlib.sha256(kare).hexdigest()],
        "tile_hashes": tiles,
        "merkle_root": _merkle(tiles),
        "sig_alg": "ES256",
        "signature": "test-signature",
    }
    m.update(extra)
    return json.dumps(m)


def _trust():
    return {
        "trust_list": ("trusted", "Issuer pinned in Trust List snapshot."),
        "ocsp": ("unknown", "No OCSP responder configured, unverified."),
        "tsa": ("trusted", "Timestamp pair agrees, accepted."),
        "rekor": ("unknown", "No inclusion proof provided, unverified."),
    }


def test_report_has_all_sections_and_disclaimer():
    r = build_report(_manifest(b"frame"), b"frame", _trust())
    for key in ("file_hashes", "badge", "reason", "trust", "tsa_note",
                "rekor_note", "witness", "disclaimer"):
        assert key in r
    assert r["disclaimer"] == DISCLAIMER
    text = render_text(r)
    assert DISCLAIMER in text
    assert "PAdES" in text or "Text/JSON" in text


def test_tampered_input_yields_red_report():
    r = build_report(_manifest(b"frame"), b"tampered", _trust())
    assert r["badge"] == "red"
    assert DISCLAIMER in render_text(r)


def test_witness_collusion_flagged():
    wit = [{"device_id": "same", "ip": "1.1.1.1"}, {"device_id": "same", "ip": "2.2.2.2"}]
    r = build_report(_manifest(b"f"), b"f", _trust(), witnesses=wit)
    assert "collusion" in r["witness"].lower()
    assert "collusion" in render_text(r).lower()


def test_no_witnesses_neutral():
    r = build_report(_manifest(b"f"), b"f", _trust())
    assert "neutral" in r["witness"].lower()
