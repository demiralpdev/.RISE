"""MS1 verify-web MVP (FastAPI). No file persistence, hash-id link only."""
import base64
import hashlib
import json

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

from trust import check_trust_list
from verify import verify_badge, verify_id

try:
    import report as _report  # lands in parallel (V-107); absent -> 501
except ImportError:
    _report = None

app = FastAPI(title="RISE verify-web MS1")

_results: dict = {}

# Badge ladder (legal weight): red < silver < gold-L2 < gold-L4. Demote only.
_LADDER = ["red", "silver", "gold-L2", "gold-L4"]


def _demote(badge: str) -> str:
    """Demote one step, floor at red. Unknown badges fail closed to red."""
    if badge not in _LADDER:
        return "red"
    return _LADDER[max(0, _LADDER.index(badge) - 1)]


def _signer_of(raw_manifest: str) -> tuple[str, str]:
    """Extract (subject, issuer) from the manifest. Unknown on any doubt."""
    try:
        m = json.loads(raw_manifest)
        if not isinstance(m, dict):
            return ("unknown-signer", "unknown-issuer")
        subject = m.get("device_id") or m.get("signer") or m.get("subject") or "unknown-signer"
        issuer = m.get("issuer") or m.get("signer_issuer") or "unknown-issuer"
        return (str(subject), str(issuer))
    except (json.JSONDecodeError, TypeError, AttributeError):
        return ("unknown-signer", "unknown-issuer")


def _apply_trust(badge: str, reason: str, raw_manifest: str) -> dict:
    """Consult the trust list; unknown/untrusted demotes one step, never upgrades."""
    subject, issuer = _signer_of(raw_manifest)
    verdict, detail = check_trust_list(subject, issuer)
    if verdict != "trusted":
        badge = _demote(badge)
    return {"badge": badge, "reason": reason, "trust": {"verdict": verdict, "detail": detail}}


def _build_report_bundle(vid: str, stored: dict):
    """Call into report.py via the first known builder; raise 501 if unusable."""
    builders = ("build_bundle", "build_report", "get_report", "make_report", "bundle", "report_bundle")
    for name in builders:
        fn = getattr(_report, name, None)
        if callable(fn):
            try:
                return fn(vid, stored)
            except TypeError:
                return fn(vid)
    raise HTTPException(status_code=501, detail="report module has no bundle builder.")


class VerifyIn(BaseModel):
    manifest: str
    frame_b64: str = ""


class ReportIn(BaseModel):
    verify_id: str


@app.get("/health")
def health():
    return {"ok": True}


@app.post("/api/v1/verify")
def verify(inp: VerifyIn):
    # Red-team R4: cap the input size (DoS surface).
    if len(inp.manifest) > 1_000_000:
        return {"badge": "red", "reason": "Manifest too large, rejected.", "verify_url": ""}
    try:
        frame = base64.b64decode(inp.frame_b64) if inp.frame_b64 else b""
    except (ValueError, TypeError):
        return {"badge": "red", "reason": "Frame undecodable, unverified.", "verify_url": ""}
    r = verify_badge(inp.manifest, frame)
    t = _apply_trust(r["badge"], r["reason"], inp.manifest)
    vid = verify_id(hashlib.sha256(inp.manifest.encode()).hexdigest())
    _results[vid] = {
        "badge": t["badge"],
        "reason": t["reason"],
        "trust": t["trust"],
        "manifest": inp.manifest,
    }
    return {
        "badge": t["badge"],
        "reason": t["reason"],
        "trust": t["trust"],
        "verify_url": f"/v/{vid}",
        "verify_id": vid,
        "note": "Not evidence, preliminary finding.",
    }


@app.post("/api/v1/report")
def make_report(inp: ReportIn):
    if _report is None:
        raise HTTPException(status_code=501, detail="report module unavailable.")
    stored = _results.get(inp.verify_id)
    if stored is None:
        raise HTTPException(status_code=404, detail="Unknown verify_id.")
    return _build_report_bundle(inp.verify_id, stored)


@app.get("/v/{vid}")
def verify_link(vid: str):
    result = _results.get(vid)
    if result is None:
        return {"verify_id": vid, "badge": "unknown", "note": "Not evidence, preliminary finding."}
    return {
        "verify_id": vid,
        "badge": result["badge"],
        "reason": result.get("reason", ""),
        "note": "Not evidence, preliminary finding.",
    }
