/* RISE client-side verifier — ES module. 100% browser, no server, no cookies. */
const GOLD_CAP = "silver";
const RED = "red";
const SILVER = "silver";
const GOLD_L4 = "gold-L4";
const GOLD_L2 = "gold-L2";

async function sha256Hex(data) {
  const d = await crypto.subtle.digest("SHA-256", data);
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
  const strip = b => {
    let a = b;
    while (a.length > 33 && a[0] === 0) a = a.slice(1);
    if (a[0] & 0x80) a = new Uint8Array([0, ...a]);
    return a;
  };
  const rs = strip(r), ss = strip(s);
  const raw = new Uint8Array(64);
  raw.set(rs.slice(-32), 0);
  raw.set(ss.slice(-32), 32);
  return raw;
}

async function verifyEs256(pemText, msgBytes, sigHex) {
  const b64 = pemText.replace(/-----(BEGIN|END) PUBLIC KEY-----/g, "").replace(/\s+/g, "");
  const spki = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
  const key = await crypto.subtle.importKey("spki", spki, { name: "ECDSA", namedCurve: "P-256" }, false, ["verify"]);
  const sig = derToRaw(Uint8Array.from(sigHex.match(/.{2}/g).map(h => parseInt(h, 16))));
  return crypto.subtle.verify({ name: "ECDSA", hash: "SHA-256" }, key, sig, msgBytes);
}

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

async function verifyBadge(rawManifest, frameBytes, opts = {}) {
  let m;
  try { m = JSON.parse(rawManifest); }
  catch { return { badge: RED, reason: "Manifest unreadable, unverified." }; }
  if (!m.signature) return { badge: RED, reason: "No signature, not evidence." };
  if (!m.sig_alg) return { badge: RED, reason: "No signature algorithm, not evidence." };

  const frameHashes = m.frame_hashes;
  if (!Array.isArray(frameHashes)) return { badge: RED, reason: "Malformed frame hashes." };
  const tileHashesIn = m.tile_hashes;
  if (!Array.isArray(tileHashesIn)) return { badge: RED, reason: "Malformed tile hashes." };
  if (typeof m.merkle_root !== "string") return { badge: RED, reason: "Malformed merkle root." };

  const ts = tiles(frameBytes);
  const tileList = [];
  for (const t of ts) tileList.push(await sha256Hex(t));
  const root = await merkleRoot(tileList);

  if (frameBytes.length && !frameHashes.includes(await sha256Hex(frameBytes)))
    return { badge: RED, reason: "Frame hash mismatch, content changed." };
  if (frameBytes.length && JSON.stringify(tileList) !== JSON.stringify(tileHashesIn))
    return { badge: RED, reason: "Tile hashes mismatch, content changed." };
  if (frameBytes.length && root !== m.merkle_root)
    return { badge: RED, reason: "Chain root mismatch, content changed." };

  if (opts.pubkeyPem && m.sig_alg === "ES256") {
    const canon = canonicalJson(m);
    try {
      const ok = await verifyEs256(opts.pubkeyPem, new TextEncoder().encode(canon), m.signature);
      if (!ok) return { badge: RED, reason: "Signature invalid for provided key." };
    } catch { return { badge: RED, reason: "Signature verification error." }; }
  }

  const tier = m.assurance ?? SILVER;
  if (![GOLD_L4, GOLD_L2, SILVER].includes(tier)) return { badge: RED, reason: "Unknown assurance tier." };
  if (tier === GOLD_L4 || (tier === GOLD_L2 && !opts.strong_signal))
    return { badge: GOLD_CAP, reason: "Gold not issued in MS1 (verification limits), downgraded." };
  return { badge: GOLD_CAP, reason: "Software signature valid (MS1), informational." };
}

/* Container extraction: pulls the embedded manifest out of a packed file.
 * Mirrors core/src/container.rs + jumbf.rs (RISE-01 §2.2).
 * JPEG: APP11 JUMBF segment. PNG: ancillary `risE` chunk (raw JSON).
 * MP4: top-level `rise` box (raw JSON). Returns {kind, manifest, clean} or null. */
function extractRise(data) {
  const u8 = data instanceof Uint8Array ? data : new Uint8Array(data);
  const at = (i, ...bs) => bs.every((b, k) => u8[i + k] === b);
  const asciiEq = (i, s) => [...s].every((c, k) => u8[i + k] === c.charCodeAt(0));
  const be32 = i => (u8[i] << 24 | u8[i + 1] << 16 | u8[i + 2] << 8 | u8[i + 3]) >>> 0;
  const hasKey = (from, to) => {
    const needle = "rise_version";
    outer: for (let i = from; i + needle.length <= to; i++) {
      for (let k = 0; k < needle.length; k++)
        if (u8[i + k] !== needle.charCodeAt(k)) continue outer;
      return true;
    }
    return false;
  };
  const jsonSlice = (from, to) => {
    let s = -1, e = -1;
    for (let i = from; i < to; i++) if (u8[i] === 0x7b) { s = i; break; }
    for (let i = to - 1; i >= from; i--) if (u8[i] === 0x7d) { e = i; break; }
    return s >= 0 && e > s ? new TextDecoder().decode(u8.slice(s, e + 1)) : null;
  };

  // JPEG
  if (u8.length > 3 && at(0, 0xff, 0xd8, 0xff)) {
    let out = [u8[0], u8[1]], i = 2, manifest = null;
    while (i + 4 <= u8.length) {
      if (u8[i] !== 0xff) return null;
      const marker = u8[i + 1];
      if (marker === 0xd9) { out.push(0xff, 0xd9); break; }
      if (marker === 0xda) {
        const seglen = be32(i + 2) >> 16;
        out.push(...u8.slice(i, i + 2 + seglen));
        i += 2 + seglen;
        if (i < u8.length) out.push(...u8.slice(i));
        break;
      }
      const seglen = be32(i + 2) >> 16;
      const end = i + 2 + seglen;
      if (end > u8.length || seglen < 2) return null;
      if (marker === 0xeb && hasKey(i, end)) {
        if (manifest === null) manifest = jsonSlice(i, end);
      } else {
        out.push(...u8.slice(i, end));
      }
      i = end;
    }
    if (manifest === null) return null;
    return { kind: "jpeg", manifest, clean: Uint8Array.from(out) };
  }

  // PNG
  if (u8.length > 8 && at(0, 0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a)) {
    let out = [...u8.slice(0, 8)], i = 8, manifest = null;
    while (i + 8 <= u8.length) {
      const len = be32(i);
      const end = i + 8 + len + 4;
      if (end > u8.length) return null;
      if (asciiEq(i + 4, "risE")) {
        if (manifest === null) manifest = new TextDecoder().decode(u8.slice(i + 8, i + 8 + len));
      } else {
        out.push(...u8.slice(i, end));
      }
      if (asciiEq(i + 4, "IEND")) break;
      i = end;
    }
    if (manifest === null) return null;
    return { kind: "png", manifest, clean: Uint8Array.from(out) };
  }

  // MP4: top-level `uuid` box (RISE UUID, C2PA convention) or legacy `rise` box
  if (u8.length > 8 && asciiEq(4, "ftyp")) {
    const RISE_UUID = [0x72,0x69,0x73,0x65,0x00,0x00,0x40,0x80,
                       0x00,0x00,0x00,0x00,0x52,0x49,0x53,0x45];
    let i = 0;
    while (i + 8 <= u8.length) {
      const size = be32(i);
      if (size === 0 || size < 8 || i + size > u8.length) break;
      if (asciiEq(i + 4, "uuid") && i + 24 <= u8.length &&
          RISE_UUID.every((b, k) => u8[i + 8 + k] === b))
        return { kind: "mp4", manifest: new TextDecoder().decode(u8.slice(i + 24, i + size)), clean: u8 };
      if (asciiEq(i + 4, "rise"))
        return { kind: "mp4", manifest: new TextDecoder().decode(u8.slice(i + 8, i + size)), clean: u8 };
      i += size;
    }
    return null;
  }

  return null;
}

export { verifyBadge, sha256Hex, merkleRoot, tiles, derToRaw, canonicalJson, verifyEs256, extractRise };
