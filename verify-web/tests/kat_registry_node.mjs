/* KAT: registry resolution — verifyBadge with NO opts.pubkeyPem resolves
 * manifest.device_id via the pinned ./keys.json registry and crypto-verifies
 * with the existing verifyEs256 (vector: tests/vectors, device "audit-device").
 * Run: node verify-web/tests/kat_registry_node.mjs
 * Fail-closed modes run in fresh processes (module caches the registry once):
 *   REGISTRY_TEST_MODE=fetch-throw | http-error | bad-json */
import { spawnSync } from "node:child_process";
import { webcrypto } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { verifyBadge } from "../static/verify.js";

if (!globalThis.crypto?.subtle) globalThis.crypto = webcrypto;

const here = path.dirname(fileURLToPath(import.meta.url));
const manifestRaw = fs.readFileSync(
  path.join(here, "vectors", "manifest.json"),
  "utf8",
);
const manifest = JSON.parse(manifestRaw);
const registry = JSON.parse(
  fs.readFileSync(path.join(here, "..", "static", "keys.json"), "utf8"),
);
const FALLBACK =
  "Structure valid, signature NOT crypto-verified (MS1), informational.";

const mode = process.env.REGISTRY_TEST_MODE ?? "default";
let fetchCalls = 0;
if (mode === "fetch-throw") {
  globalThis.fetch = async () => {
    fetchCalls++;
    throw new Error("network down");
  };
} else if (mode === "http-error") {
  globalThis.fetch = async () => {
    fetchCalls++;
    return { ok: false, status: 404 };
  };
} else if (mode === "bad-json") {
  globalThis.fetch = async () => {
    fetchCalls++;
    return {
      ok: true,
      json: async () => {
        throw new Error("not json");
      },
    };
  };
} else {
  globalThis.fetch = async (url) => {
    fetchCalls++;
    if (String(url) !== "./keys.json")
      throw new Error("unexpected registry URL: " + url);
    return { ok: true, json: async () => registry };
  };
}

function fail(msg, res) {
  console.error(msg, res === undefined ? "" : JSON.stringify(res));
  process.exit(1);
}

if (mode !== "default") {
  // Every registry failure must fall through silently: no throw, no
  // validity claim, no keySource — the pre-existing MS1 informational path.
  const res = await verifyBadge(manifestRaw, new Uint8Array(), {});
  if (res.badge !== "silver" || res.reason !== FALLBACK || "keySource" in res)
    fail(mode + " must fall through to the MS1 informational path:", res);
  console.log("Fail-closed ok (" + mode + "): no throw, no validity claim");
  process.exit(0);
}

// 1. Registry hit: real crypto verify of the existing vector, no pasted PEM.
const res = await verifyBadge(manifestRaw, new Uint8Array(), {});
if (
  res.badge !== "silver" ||
  res.reason !== "Signature verified against pinned device key." ||
  res.keySource !== "registry"
)
  fail("registry resolution must verify the vector:", res);
console.log(
  "Registry ok: GOLD_CAP(silver) via pinned device key, keySource=registry",
);

// 2. Cached: the second call must not refetch keys.json.
await verifyBadge(manifestRaw, new Uint8Array(), {});
if (fetchCalls !== 1)
  fail("registry fetched " + fetchCalls + "x, expected exactly 1:");
console.log("Cache ok: ./keys.json fetched exactly once");

// 3. Unknown device: fail closed on the MS1 informational path.
const unknown = JSON.stringify({ ...manifest, device_id: "no-such-device" });
const resUnknown = await verifyBadge(unknown, new Uint8Array(), {});
if (
  resUnknown.badge !== "silver" ||
  resUnknown.reason !== FALLBACK ||
  "keySource" in resUnknown
)
  fail("unknown device must fall through:", resUnknown);
console.log("Fail-closed ok: unknown device claims nothing");

// 4. Fetch/parse failures in fresh processes (registry cache is per-load).
for (const m of ["fetch-throw", "http-error", "bad-json"]) {
  const r = spawnSync(process.execPath, [fileURLToPath(import.meta.url)], {
    env: { ...process.env, REGISTRY_TEST_MODE: m },
    encoding: "utf8",
  });
  if (r.status !== 0)
    fail("mode " + m + " exited " + r.status + ": " + r.stderr + r.stdout);
  process.stdout.write(r.stdout);
}
