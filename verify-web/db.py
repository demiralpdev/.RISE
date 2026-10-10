"""MongoDB evidence ledger — append-only verification results.

The connection string lives in .env (gitignored, never committed).
If MongoDB is unreachable, the in-memory dict in app.py still serves.
"""
import hashlib
import json
import os
from datetime import datetime, timezone

_client = None


def _get_client():
    global _client
    if _client is not None:
        return _client
    uri = os.environ.get("RISE_MONGO_URI", "")
    if not uri:
        _env = os.path.join(os.path.dirname(__file__), ".env")
        if os.path.exists(_env):
            for line in open(_env):
                line = line.strip()
                if line.startswith("RISE_MONGO_URI="):
                    uri = line.split("=", 1)[1].strip().strip('"').strip("'")
                    break
    if not uri:
        return None
    try:
        from pymongo import MongoClient
        _client = MongoClient(uri, serverSelectionTimeoutMS=5000)
        _client.admin.command("ping")
        return _client
    except Exception:
        _client = None
        return None


def _db():
    c = _get_client()
    if c is None:
        return None
    return c["rise_evidence"]["verifications"]


def store(vid: str, badge: str, reason: str, manifest: str, trust: dict) -> bool:
    col = _db()
    if col is None:
        return False
    try:
        manifest_hash = hashlib.sha256(manifest.encode()).hexdigest()
        col.insert_one({
            "_id": vid,
            "badge": badge,
            "reason": reason,
            "manifest_hash": manifest_hash,
            "manifest": manifest,
            "trust": trust,
            "stored_at": datetime.now(timezone.utc),
        })
        return True
    except Exception:
        return False


def fetch(vid: str) -> dict | None:
    col = _db()
    if col is None:
        return None
    try:
        doc = col.find_one({"_id": vid})
        if doc is None:
            return None
        return {
            "badge": doc.get("badge", "unknown"),
            "reason": doc.get("reason", ""),
            "trust": doc.get("trust", {}),
            "manifest_hash": doc.get("manifest_hash", ""),
            "stored_at": str(doc.get("stored_at", "")),
        }
    except Exception:
        return None


def health() -> bool:
    return _get_client() is not None
