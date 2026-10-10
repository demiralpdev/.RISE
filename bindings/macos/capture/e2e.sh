#!/bin/bash
# T-301 macOS end-to-end loop test (PLAN.md).
# Full loop on this machine:
#   capture.sh -> keygen (/tmp) -> sign --key --tsa -> verify --pubkey
#   -> pack -> unpack (manifest roundtrip) -> tampered copy (INVALID, exit 3).
# Exits 0 only if every stage passes; prints one line per stage.
# Temp keys live in /tmp and are deleted at end via trap. No secrets in repo.
set -uo pipefail
cd "$(dirname "$0")"

CORE="../../../core/target/debug/rise-core"
TSA="https://freetsa.org/tsr"
TMP="$(mktemp -d /tmp/rise-e2e.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

FRAME="$TMP/frame.jpg"
MANIFEST="$TMP/signed.manifest.json"
SEALED="$TMP/sealed.jpg"
UNPACKED="$TMP/unpacked.manifest.json"
TAMPERED="$TMP/tampered.jpg"

pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1"; exit 1; }

# Stage 1: core CLI present (relative path from capture dir).
[ -x "$CORE" ] || fail "core CLI missing at $CORE (build core first: cargo build -p rise-core)"
ls "$CORE" >/dev/null 2>&1 || fail "core CLI not listable at $CORE"
pass "stage 1/8 core CLI present ($CORE)"

# Stage 2: capture one real frame (no fakes; permission errors abort).
if ./capture.sh "$FRAME" >/dev/null 2>&1; then
  [ -s "$FRAME" ] || fail "stage 2/8 capture produced empty frame"
  pass "stage 2/8 capture real frame ($FRAME)"
else
  echo "FAIL: stage 2/8 capture failed (no fake frame used)."
  echo "Permission hint: System Settings > Privacy & Security > Camera > allow Terminal, then run again."
  exit 1
fi

# Stage 3: keygen into /tmp (never the repo).
"$CORE" keygen --out "$TMP/key" >/dev/null 2>&1 \
  || fail "stage 3/8 keygen failed"
[ -f "$TMP/key.key" ] && [ -f "$TMP/key.pub" ] \
  || fail "stage 3/8 keygen output missing"
pass "stage 3/8 keygen temp keypair (/tmp, cleaned by trap)"

# Stage 4: sign with key + TSA stamp (stamp-then-sign).
"$CORE" sign "$FRAME" --device e2e-test --key "$TMP/key.key" \
  --out "$MANIFEST" --tsa "$TSA" >/dev/null 2>&1 \
  || fail "stage 4/8 sign --key --tsa failed"
[ -s "$MANIFEST" ] || fail "stage 4/8 sign produced empty manifest"
pass "stage 4/8 sign --key --tsa $TSA"

# Stage 5: verify signed manifest (expect VALID + exit 0).
OUT="$("$CORE" verify "$MANIFEST" "$FRAME" --pubkey "$TMP/key.pub" 2>&1)"
CODE=$?
[ "$CODE" -eq 0 ] || fail "stage 5/8 verify exit=$CODE (want 0): $OUT"
echo "$OUT" | grep -q "VALID" || fail "stage 5/8 verify output lacks VALID: $OUT"
pass "stage 5/8 verify VALID signed (exit 0)"

# Stage 6: pack manifest into JPEG (JUMBF APP11).
"$CORE" pack "$FRAME" "$MANIFEST" --out "$SEALED" >/dev/null 2>&1 \
  || fail "stage 6/8 pack failed"
[ -s "$SEALED" ] || fail "stage 6/8 pack produced empty file"
pass "stage 6/8 pack sealed JPEG"

# Stage 7: unpack + manifest roundtrip (unpack adds one trailing newline
# via println; command substitution strips trailing newlines, so the
# comparison stays strict on every other byte).
"$CORE" unpack "$SEALED" >"$UNPACKED" 2>/dev/null \
  || fail "stage 7/8 unpack failed"
[ "$(cat "$MANIFEST")" = "$(cat "$UNPACKED")" ] \
  || fail "stage 7/8 unpack roundtrip differs from signed manifest"
pass "stage 7/8 unpack manifest roundtrip identical"

# Stage 8: tampered copy must be INVALID with exit 3.
cp "$FRAME" "$TAMPERED"
printf 'rise-e2e-tamper' >>"$TAMPERED"
TOUT="$("$CORE" verify "$MANIFEST" "$TAMPERED" --pubkey "$TMP/key.pub" 2>&1)"
TCODE=$?
[ "$TCODE" -eq 3 ] || fail "stage 8/8 tampered verify exit=$TCODE (want 3): $TOUT"
echo "$TOUT" | grep -q "INVALID" || fail "stage 8/8 tampered output lacks INVALID: $TOUT"
pass "stage 8/8 tampered copy INVALID (exit 3)"

echo "E2E GREEN: all 8 stages passed"
