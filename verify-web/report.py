"""V-107 report output: text/JSON bundle only. Fail-closed, English only."""
import hashlib

from verify import verify_badge

DISCLAIMER = "Not evidence, preliminary finding."
FORMAT_NOTE = "Text/JSON bundle only; no PAdES signing claimed."
TSA_POLICY = "Multi-TSA required; a single TSA is insufficient."
REKOR_POLICY = "Rekor inclusion is transparency only, not a time proof."


def _sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _verdict(entry) -> tuple[str, str]:
    try:
        if isinstance(entry, (list, tuple)) and len(entry) == 2:
            v, r = str(entry[0]), str(entry[1])
        elif isinstance(entry, dict):
            v, r = str(entry.get("verdict", "unknown")), str(entry.get("reason", ""))
        else:
            return ("unknown", "Verdict missing, unverified.")
        if v not in ("trusted", "untrusted", "unknown"):
            return ("unknown", "Verdict missing, unverified.")
        return (v, r or "No reason given.")
    except Exception:
        return ("unknown", "Verdict missing, unverified.")


def _witness_note(witnesses) -> str:
    if not witnesses:
        return "No witness list provided, independence neutral (stub)."
    try:
        devs = [str(w.get("device_id", "")) for w in witnesses if isinstance(w, dict)]
        ips = [str(w.get("ip", "")) for w in witnesses if isinstance(w, dict)]
        if len(set(d for d in devs if d)) < len([d for d in devs if d]):
            return "Witness collusion suspected: shared device (stub)."
        if len(set(i for i in ips if i)) < len([i for i in ips if i]):
            return "Witness collusion suspected: shared IP (stub)."
        return "Witnesses appear independent (stub check)."
    except Exception:
        return "Witness list unreadable, independence neutral (stub)."


def build_report(manifest: str, frame: bytes, trust_result=None, witnesses=None) -> dict:
    """Builds a fail-closed report dict. Never raises; tampering yields red."""
    try:
        data = bytes(frame or b"")
    except Exception:
        data = b""
    try:
        badge = verify_badge(manifest, data)
    except Exception:
        badge = {"badge": "red", "reason": "Report failed closed, unverified."}
    trust = trust_result if isinstance(trust_result, dict) else {}
    wit = witnesses
    if wit is None and isinstance(trust.get("witnesses"), list):
        wit = trust["witnesses"]
    tsa_v, tsa_r = _verdict(trust.get("tsa"))
    rekor_v, rekor_r = _verdict(trust.get("rekor"))
    return {
        "file_hashes": {"sha256": _sha256(data)},
        "badge": badge.get("badge", "red"),
        "reason": badge.get("reason", "Unverified."),
        "trust": {k: list(_verdict(v)) for k, v in trust.items() if k != "witnesses"},
        "tsa_note": f"{TSA_POLICY} Status: {tsa_v} — {tsa_r}",
        "rekor_note": f"{REKOR_POLICY} Status: {rekor_v} — {rekor_r}",
        "witness": _witness_note(wit),
        "disclaimer": DISCLAIMER,
        "format_note": FORMAT_NOTE,
    }


def render_text(report: dict) -> str:
    """Renders a printable one-page text report. Fail-closed on bad input."""
    try:
        r = dict(report or {})
    except Exception:
        r = {}
    badge = r.get("badge", "red")
    lines = [
        "RISE verification report",
        f"Badge: {badge}",
        f"Reason: {r.get('reason', 'Unverified.')}",
        f"SHA-256: {(r.get('file_hashes') or {}).get('sha256', 'n/a')}",
        "Trust verdicts:",
    ]
    trust = r.get("trust") or {}
    if not trust:
        lines.append("- none provided: unknown — unverified.")
    else:
        for k in sorted(trust):
            v, reason = _verdict(trust[k])
            lines.append(f"- {k}: {v} — {reason}")
    lines += [
        f"TSA: {r.get('tsa_note', TSA_POLICY)}",
        f"Rekor: {r.get('rekor_note', REKOR_POLICY)}",
        f"Witness: {r.get('witness', 'neutral (stub).')}",
        f"Format: {r.get('format_note', FORMAT_NOTE)}",
        f"Disclaimer: {r.get('disclaimer', DISCLAIMER)}",
    ]
    return "\n".join(lines) + "\n"
