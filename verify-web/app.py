"""MS1 verify-web MVP (FastAPI). No file persistence, hash-id link only."""
import base64

from fastapi import FastAPI
from pydantic import BaseModel

from verify import verify_badge, verify_id

app = FastAPI(title="RISE verify-web MS1")

_results: dict = {}


class VerifyIn(BaseModel):
    manifest: str
    frame_b64: str = ""


@app.get("/health")
def health():
    return {"ok": True}


@app.post("/api/v1/verify")
def verify(inp: VerifyIn):
    try:
        frame = base64.b64decode(inp.frame_b64) if inp.frame_b64 else b""
    except (ValueError, TypeError):
        return {"badge": "red", "reason": "Frame undecodable, unverified.", "verify_url": ""}
    r = verify_badge(inp.manifest, frame)
    vid = verify_id(r["reason"] + inp.manifest[:32])
    _results[vid] = {"badge": r["badge"], "reason": r["reason"]}
    r["verify_url"] = f"/v/{vid}"
    r["note"] = "Not evidence, preliminary finding."
    return r


@app.get("/v/{vid}")
def verify_link(vid: str):
    result = _results.get(vid)
    if result is None:
        return {"verify_id": vid, "badge": "unknown", "note": "Not evidence, preliminary finding."}
    return {"verify_id": vid, **result, "note": "Not evidence, preliminary finding."}
