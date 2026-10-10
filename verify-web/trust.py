"""V-103/V-104 trust layer: Trust List + OCSP hard-fail + multi-TSA check. Fail-closed."""
import json
from pathlib import Path

TRUSTED, UNTRUSTED, UNKNOWN = "trusted", "untrusted", "unknown"
MAX_TSA_SKEW_SECS = 300  # mirrors core/src/timestamp.rs MAX_TSA_SKEW_SECS
_DEFAULT_LIST = Path(__file__).with_name("trust-list.json")


def load_trust_list(path=None):
    """Loads pinned Trust List snapshot; raises on any error (caller maps to unknown)."""
    p = Path(path) if path else _DEFAULT_LIST
    return json.loads(p.read_text())


def check_trust_list(subject, issuer, path=None):
    """Trust List check: self-signed -> untrusted, unknown CA -> unknown, never trusted."""
    if subject == issuer:
        return (UNTRUSTED, "Self-signed certificate, untrusted.")
    try:
        anchors = load_trust_list(path).get("anchors", [])
    except (OSError, ValueError, AttributeError):
        return (UNKNOWN, "Trust list unreachable, unverified.")
    ids = {a.get("subject") for a in anchors} | {a.get("fingerprint") for a in anchors}
    if issuer in ids:
        return (TRUSTED, "Issuer pinned in Trust List snapshot.")
    return (UNKNOWN, "Unknown issuer, unverified.")


def check_ocsp(cert_pem=None, responder_url=None):
    """OCSP hard-fail stub: no responder configured -> unknown, never green."""
    # TODO: wire live OCSP (fetch responder, hard-fail on no/revoked response).
    _ = (cert_pem, responder_url)
    return (UNKNOWN, "No OCSP responder configured, unverified.")


def cross_check_tsa(token_a, token_b):
    """Multi-TSA cross-check: imprints must agree, times within 5-min skew, else reject."""
    try:
        ia, ta = token_a["imprint"], int(token_a["time"])
        ib, tb = token_b["imprint"], int(token_b["time"])
    except (KeyError, TypeError, ValueError, AttributeError):
        return (UNTRUSTED, "Malformed timestamp token, rejected.")
    if not ia or ia != ib:
        return (UNTRUSTED, "Timestamp imprints disagree, rejected.")
    if abs(ta - tb) > MAX_TSA_SKEW_SECS:
        return (UNTRUSTED, "Timestamp skew exceeds 5 minutes, rejected.")
    return (TRUSTED, "Timestamp pair agrees, accepted.")
