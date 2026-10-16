"""MS1 verify-web MVP (FastAPI). No file persistence, hash-id link only."""
import base64
import hashlib
import json
from types import ModuleType

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from trust import check_trust_list, time_revocation_verdict
from verify import verify_badge, verify_id

_db: ModuleType | None
try:
    import db as _db
except ImportError:
    _db = None

_report: ModuleType | None
try:
    import report as _report  # lands in parallel (V-107); absent -> 501
except ImportError:
    _report = None

app = FastAPI(title="RISE verify-web MS1")

_results: dict = {}

# Bounded in-memory store: evict oldest past the cap (unbounded growth is a DoS surface).
_RESULTS_MAX = 4096

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


def _time_ceiling(badge: str, verdicts: dict) -> str:
    """Ceiling: badge may exceed silver only when timestamp+tsa+ocsp are all trusted.

    Unknown never demotes (gold-* is clamped to silver, anything else untouched);
    a single untrusted verdict demotes one step (then the ceiling still applies).
    """
    try:
        vs = []
        for _k in ("timestamp", "tsa", "ocsp"):
            _e = verdicts.get(_k) if isinstance(verdicts, dict) else None
            if isinstance(_e, (list, tuple)) and len(_e) >= 1:
                vs.append(str(_e[0]))
            elif isinstance(_e, dict):
                vs.append(str(_e.get("verdict", "unknown")))
            else:
                vs.append("unknown")
    except Exception:
        vs = ["unknown", "unknown", "unknown"]
    if all(_v == "trusted" for _v in vs):
        return badge
    out = _demote(badge) if any(_v == "untrusted" for _v in vs) else badge
    if out in ("gold-L2", "gold-L4"):
        return "silver"
    return out


def _apply_trust(badge: str, reason: str, raw_manifest: str, key_verified: bool = False,
                 key_source: str | None = None) -> dict:
    """Consult the trust list; unknown/untrusted demotes one step, never upgrades.

    A manifest whose signature verified against a pinned device-key registry
    entry carries its own trust anchor (the pinned key itself), so it is NOT
    demoted by the C2PA TSA anchor list. Every other path is unchanged.

    ADDITIONAL (time-proof ceiling): the final badge can never exceed silver
    unless timestamp+tsa+ocsp are ALL trusted. Unknown time/revocation
    verdicts cap but never demote; untrusted ones demote one step.
    """
    try:
        _m = json.loads(raw_manifest) if isinstance(raw_manifest, str) else {}
        if not isinstance(_m, dict):
            _m = {}
    except (json.JSONDecodeError, TypeError, AttributeError):
        _m = {}
    _trv = time_revocation_verdict(_m)
    _extra = {k: list(v) for k, v in _trv.items()}
    if key_verified is True and key_source == "registry":
        subject, issuer = _signer_of(raw_manifest)
        verdict, detail = check_trust_list(subject, issuer)
        badge = _time_ceiling(badge, _trv)
        return {"badge": badge, "reason": reason,
                "trust": {"verdict": verdict, "detail": detail,
                          "anchor": "device-key-registry", **_extra}}
    subject, issuer = _signer_of(raw_manifest)
    verdict, detail = check_trust_list(subject, issuer)
    if verdict != "trusted":
        badge = _demote(badge)
    badge = _time_ceiling(badge, _trv)
    return {"badge": badge, "reason": reason,
            "trust": {"verdict": verdict, "detail": detail, **_extra}}


def _build_report_bundle(vid: str, stored: dict):
    """Build the printable report from the stored verdict (report.py contract).

    Frames are not persisted, so report.build_report cannot re-run the frame
    checks; the stored verdict WINS and the badge is never upgraded.
    """
    fn = getattr(_report, "build_report", None)
    if not callable(fn):
        raise HTTPException(status_code=501, detail="report module has no builder.")
    rep = fn(stored.get("manifest", ""), b"", trust_result=stored.get("trust"))
    rep["verify_id"] = vid
    rep["badge"] = stored.get("badge", "red")
    rep["reason"] = stored.get("reason", rep.get("reason", ""))
    return rep


class VerifyIn(BaseModel):
    manifest: str
    frame_b64: str = ""
    signer_pub: str = ""


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
    r = verify_badge(inp.manifest, frame, signer_pub=inp.signer_pub or None)
    t = _apply_trust(r["badge"], r["reason"], inp.manifest,
                     key_verified=r.get("key_verified", False),
                     key_source=r.get("key_source"))
    vid = verify_id(hashlib.sha256(inp.manifest.encode()).hexdigest())
    _results[vid] = {
        "badge": t["badge"],
        "reason": t["reason"],
        "trust": t["trust"],
        "manifest": inp.manifest,
    }
    while len(_results) > _RESULTS_MAX:
        _results.pop(next(iter(_results)))
    if _db is not None:
        _db.store(vid, t["badge"], t["reason"], inp.manifest, t["trust"])
    resp = {
        "badge": t["badge"],
        "reason": t["reason"],
        "trust": t["trust"],
        "verify_url": f"/v/{vid}",
        "verify_id": vid,
        "note": "Not evidence, preliminary finding.",
    }
    if "capture_source" in r:
        resp["capture_source"] = r["capture_source"]
    if "capture_note" in r:
        resp["capture_note"] = r["capture_note"]
    if r.get("key_verified") is True:
        resp["key_verified"] = True
        resp["key_source"] = r.get("key_source")
    return resp


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
    if result is None and _db is not None:
        mongo = _db.fetch(vid)
        if mongo is not None:
            result = {"badge": mongo["badge"], "reason": mongo["reason"], "trust": mongo.get("trust", {})}
    if result is None:
        return {"verify_id": vid, "badge": "unknown", "note": "Not evidence, preliminary finding."}
    return {
        "verify_id": vid,
        "badge": result["badge"],
        "reason": result.get("reason", ""),
        "note": "Not evidence, preliminary finding.",
    }
