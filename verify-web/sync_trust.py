"""MS5 trust-list sync: refresh the pinned C2PA snapshot, fail-closed. Stdlib only."""
import json
import os
import ssl
import sys
import urllib.request
from pathlib import Path

# Live probe 2026-10-10: trust-list.c2pa.org did not resolve (DNS NXDOMAIN);
# URL below is pinned from the "source" field of the trust-list.json snapshot.
UPSTREAM_URL = "https://trust-list.c2pa.org"
_DEFAULT_PATH = Path(__file__).with_name("trust-list.json")
_TIMEOUT_SECS = 20


class SyncError(Exception):
    """Upstream payload failed validation; sync must abort without writing."""


def fetch(url=UPSTREAM_URL):
    """Download the upstream Trust List; any network error propagates."""
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=_TIMEOUT_SECS) as resp:
        return resp.read()


def _is_newer(upstream, current):
    """Newer version wins; ties fall back to snapshot_date (ISO strings)."""
    uv, cv = int(upstream.get("version", 0)), int(current.get("version", 0))
    if uv != cv:
        return uv > cv
    return str(upstream.get("snapshot_date", "")) > str(current.get("snapshot_date", ""))


def validate(upstream, current):
    """True if upstream is well-formed and newer; False if same/older. Raises SyncError."""
    if not isinstance(upstream, dict):
        raise SyncError("Upstream payload is not a JSON object.")
    anchors = upstream.get("anchors")
    if not isinstance(anchors, list) or not anchors:
        raise SyncError("Upstream has no anchors.")
    for anchor in anchors:
        pem = anchor.get("pem") if isinstance(anchor, dict) else None
        if not isinstance(anchor, dict) or not anchor.get("subject") or not pem:
            raise SyncError("Anchor missing subject/pem.")
        try:
            ssl.PEM_cert_to_DER_cert(pem)
        except Exception:
            raise SyncError("Anchor PEM does not parse.")
    return _is_newer(upstream, current)


def sync(raw, path=None):
    """Validate raw upstream bytes; atomically replace snapshot only on update."""
    dest = Path(path) if path else _DEFAULT_PATH
    try:
        upstream = json.loads(raw)
    except ValueError:
        raise SyncError("Upstream is not valid JSON.")
    current = json.loads(dest.read_text()) if dest.exists() else {}
    if not validate(upstream, current):
        return "no-op"
    tmp = dest.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(upstream, indent=2) + "\n")
    os.replace(tmp, dest)  # atomic: snapshot is never left partial
    return "updated"


def main(argv=None):
    args = (argv or sys.argv)[1:]
    try:
        print(f"sync {sync(fetch(args[0] if args else UPSTREAM_URL))}")
    except (SyncError, OSError, ValueError) as exc:
        print(f"sync failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
