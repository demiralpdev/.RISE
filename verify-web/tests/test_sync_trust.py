"""Sync tests — inline recorded fixtures only, no network."""
import base64
import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from sync_trust import SyncError, sync  # noqa: E402

_PEM = ("-----BEGIN CERTIFICATE-----\n"
        + base64.b64encode(b"recorded-test-anchor").decode()
        + "\n-----END CERTIFICATE-----\n")
CURRENT = {"version": 3, "snapshot_date": "2026-10-10", "anchors": []}


def _payload(**over):
    doc = {"version": 4, "snapshot_date": "2026-10-11",
           "anchors": [{"subject": "new-anchor-ca", "pem": _PEM}]}
    doc.update(over)
    return json.dumps(doc).encode()


def _snap(tmp_path):
    dest = tmp_path / "trust-list.json"
    dest.write_text(json.dumps(CURRENT))
    return dest


def test_newer_version_updates_snapshot(tmp_path):
    dest = _snap(tmp_path)
    assert sync(_payload(), dest) == "updated"
    assert json.loads(dest.read_text())["version"] == 4
    assert not dest.with_suffix(".json.tmp").exists()


def test_same_version_is_no_op(tmp_path):
    dest = _snap(tmp_path)
    before = dest.read_bytes()
    assert sync(_payload(version=3, snapshot_date="2026-10-10"), dest) == "no-op"
    assert dest.read_bytes() == before


def test_malformed_upstream_rejected(tmp_path):
    dest = _snap(tmp_path)
    before = dest.read_bytes()
    bad_pem = _payload(version=5, anchors=[{"subject": "x", "pem": "not-a-pem"}])
    for raw in (bad_pem, b"{not json", json.dumps({"version": 6}).encode()):
        with pytest.raises(SyncError):
            sync(raw, dest)
    assert dest.read_bytes() == before  # fail-closed: snapshot untouched
    assert not dest.with_suffix(".json.tmp").exists()
