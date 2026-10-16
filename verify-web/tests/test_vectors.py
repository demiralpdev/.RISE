"""KAT: locks the Python canonicalizer to a real rise-core signed manifest.

The fixture (tests/vectors) was signed by `rise-core sign` — its signature
verifies ONLY over the exact Rust serde byte layout, so any drift in
_canonical_bytes breaks these tests by design.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from verify import _canonical_bytes, verify_signature

VECTORS = Path(__file__).resolve().parent / "vectors"


def _load() -> tuple[dict, str, str]:
    manifest = json.loads((VECTORS / "manifest.json").read_text())
    canonical = (VECTORS / "canonical.txt").read_text()
    pubkey = (VECTORS / "pubkey.pem").read_text()
    return manifest, canonical, pubkey


def test_canonical_bytes_match_rise_core_fixture():
    manifest, canonical, _ = _load()
    assert _canonical_bytes(manifest).decode("utf-8") == canonical


def test_verify_signature_accepts_rise_core_fixture():
    manifest, _, pubkey = _load()
    assert verify_signature(manifest, pubkey) is True


def test_verify_signature_rejects_tampered_fixture():
    manifest, _, pubkey = _load()
    tampered = dict(manifest)
    tampered["device_id"] = "attacker"
    assert verify_signature(tampered, pubkey) is False
