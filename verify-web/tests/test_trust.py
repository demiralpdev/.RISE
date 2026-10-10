"""V-103/V-104 trust layer tests — recorded vectors only, no network."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from trust import check_ocsp, check_trust_list, cross_check_tsa  # noqa: E402

ANCHOR = "test-anchor-ca"


def _tok(imprint="ab" * 32, time=1_760_000_000):
    return {"imprint": imprint, "time": time}


def test_self_signed_untrusted():
    v, _ = check_trust_list("self-signed-test", "self-signed-test")
    assert v == "untrusted"


def test_unknown_ca_unknown_never_trusted():
    v, _ = check_trust_list("leaf", "unknown-ca-xyz")
    assert v == "unknown" and v != "trusted"


def test_unreachable_list_unknown():
    v, _ = check_trust_list("leaf", ANCHOR, path="/nonexistent/list.json")
    assert v == "unknown" and v != "trusted"


def test_known_anchor_trusted():
    v, _ = check_trust_list("leaf", ANCHOR)
    assert v == "trusted"


def test_ocsp_stub_never_green():
    assert check_ocsp()[0] == "unknown"
    assert check_ocsp("pem", "http://x")[0] != "trusted"
    src = Path(__file__).resolve().parent.parent.joinpath("trust.py").read_text()
    assert "TODO" in src  # locks the stub until live OCSP lands


def test_tsa_match_accepted():
    v, _ = cross_check_tsa(_tok(), _tok(time=1_760_000_100))
    assert v == "trusted"


def test_tsa_mismatch_rejected():
    v, _ = cross_check_tsa(_tok("aa" * 32), _tok("bb" * 32))
    assert v == "untrusted"


def test_tsa_skew_rejected():
    v, _ = cross_check_tsa(_tok(), _tok(time=1_760_000_000 + 301))
    assert v == "untrusted"


def test_tsa_malformed_rejected():
    assert cross_check_tsa({}, _tok())[0] == "untrusted"
    assert cross_check_tsa(None, None)[0] == "untrusted"
