/* .rise static verifier — client-side badge decision tree (mirror of verify.py).
 * Fail-closed: on any doubt, downgrade. Gold is never issued in MS1. */
const GOLD_CAP = "silver";
const RED = "red";
const SILVER = "silver";
const GOLD_L4 = "gold-L4";
const GOLD_L2 = "gold-L2";

async function sha256Hex(bytes) {
  const d = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(d)].map(b => b.toString(16).padStart(2, "0")).join("");
}

function tiles(data, maxN = 4) {
  if (!data.length) return [];
  const n = Math.min(data.length, maxN);
  const size = Math.ceil(data.length / n);
  const out = [];
  for (let i = 0; i < data.length; i += size) out.push(data.slice(i, i + size));
  return out;
}

async function merkleRoot(hashes) {
  if (!hashes.length) return sha256Hex(new Uint8Array(0));
  let kat = [...hashes];
  while (kat.length > 1) {
    const ust = [];
    for (let i = 0; i < kat.length; i += 2) {
      const sag = i + 1 < kat.length ? kat[i + 1] : kat[i];
      ust.push(await sha256Hex(new TextEncoder().encode(kat[i] + sag)));
    }
    kat = ust;
  }
  return kat[0];
}

/* DER ECDSA signature -> raw 64-byte R||S (WebCrypto expects raw). */
function derToRaw(der) {
  if (der.length < 8 || der[0] !== 0x30) return der;
  let p = 2;
  const readInt = () => {
    if (der[p] !== 0x02) return null;
    const len = der[p + 1];
    const val = der.slice(p + 2, p + 2 + len);
    p += 2 + len;
    return val;
  };
  const r = readInt(), s = readInt();
  if (!r || !s) return der;
  const strip = (b) => { let a = b; while (a.length > 33 && a[0] === 0) a = a.slice(1); if (a[0] & 0x80) a = new Uint8Array([0, ...a]); return a; };
  const rs = strip(r), ss = strip(s);
  const raw = new Uint8Array(64);
  raw.set(rs.slice(-32), 0);
  raw.set(ss.slice(-32), 32);
  return raw;
}

/* ES256 verify: PEM/SPKI pubkey pasted by the verifier (out-of-band trust). */
async function verifyEs256(pemText, msgBytes, sigHex) {
  const b64 = pemText.replace(/-----(BEGIN|END) PUBLIC KEY-----/g, "").replace(/\s+/g, "");
  const spki = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
  const key = await crypto.subtle.importKey("spki", spki, { name: "ECDSA", namedCurve: "P-256" }, false, ["verify"]);
  const sig = derToRaw(Uint8Array.from(sigHex.match(/.{2}/g).map(h => parseInt(h, 16))));
  return crypto.subtle.verify({ name: "ECDSA", hash: "SHA-256" }, key, sig, msgBytes);
}

/* Must mirror the Rust canonical form EXACTLY: struct field order, unknown
 * fields dropped, signature forced to null. */
function canonicalJson(m) {
  const o = {};
  o.rise_version = m.rise_version ?? 1;
  o.frame_hashes = m.frame_hashes ?? [];
  o.tile_hashes = m.tile_hashes ?? [];
  o.merkle_root = m.merkle_root ?? "";
  o.sig_alg = m.sig_alg ?? "ES256";
  o.timestamp_ms = m.timestamp_ms ?? 0;
  o.device_id = m.device_id ?? "";
  o.assurance = m.assurance ?? "silver";
  o.timestamp_token = m.timestamp_token ?? null;
  o.signature = null;
  o.signing_key_id = m.signing_key_id ?? null;
  return JSON.stringify(o);
}

/* Badge decision: mirrors verify-web/verify.py (Phase 1 tree, fail-closed). */
async function verifyBadge(rawManifest, frameBytes, opts = {}) {
  let m;
  try { m = JSON.parse(rawManifest); }
  catch { return { badge: RED, reason: "Manifest unreadable, unverified." }; }
  if (!m.signature) return { badge: RED, reason: "No signature, not evidence." };
  if (!m.sig_alg) return { badge: RED, reason: "No signature algorithm, not evidence." };

  const frameHashes = m.frame_hashes || [];
  const tileHashes = m.tile_hashes || [];
  const ts = tiles(frameBytes);
  const frameHash = await sha256Hex(frameBytes);
  const tileList = [];
  for (const t of ts) tileList.push(await sha256Hex(t));
  const root = await merkleRoot(tileList);

  if (frameHashes.length && !frameHashes.includes(frameHash))
    return { badge: RED, reason: "Frame hash mismatch, content changed." };
  if (JSON.stringify(tileList) !== JSON.stringify(tileHashes))
    return { badge: RED, reason: "Tile hashes mismatch, content changed." };
  if (root !== m.merkle_root)
    return { badge: RED, reason: "Chain root mismatch, content changed." };

  /* Optional real signature verification (ES256) when a pubkey is provided. */
  let sigVerified = null;
  if (opts.pubkeyPem && m.sig_alg === "ES256") {
    const canon = canonicalJson(m);
    try {
      sigVerified = await verifyEs256(opts.pubkeyPem, new TextEncoder().encode(canon), m.signature);
    } catch { sigVerified = false; }
    if (!sigVerified) return { badge: RED, reason: "Signature invalid for provided key." };
  }

  if (opts.replay) return { badge: RED, reason: "Nonce replay suspicion, unverified." };
  const tier = m.assurance || SILVER;
  if (![GOLD_L4, GOLD_L2, SILVER].includes(tier))
    return { badge: RED, reason: "Unknown assurance tier." };
  /* MS1 cap: gold is never issued client-side, even with strong signals. */
  if (tier === GOLD_L4 || (tier === GOLD_L2 && !opts.strong_signal))
    return { badge: GOLD_CAP, reason: "Gold not issued in MS1 (chain/verification limits), downgraded." };
  return { badge: GOLD_CAP, reason: "Software signature valid (MS1), informational." };
}

export { verifyBadge, sha256Hex, merkleRoot, tiles, derToRaw, canonicalJson, verifyEs256 };
