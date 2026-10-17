/* KAT: locks the JS canonicalizer + ES256 verify to a real rise-core artifact.
 * Fixture: tests/vectors (signed by `rise-core sign`, throwaway audit key).
 * Run: node verify-web/tests/kat_node.mjs */
import fs from "node:fs";
import path from "node:path";
import { webcrypto } from "node:crypto";
import { fileURLToPath } from "node:url";
import { canonicalJson, verifyBadge, verifyEs256, NON_CAMERA_SOURCES } from "../static/verify.js";

if (!globalThis.crypto?.subtle) globalThis.crypto = webcrypto;

const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), "vectors");
const manifest = JSON.parse(fs.readFileSync(path.join(dir, "manifest.json"), "utf8"));
const canonical = fs.readFileSync(path.join(dir, "canonical.txt"), "utf8");
const pubkey = fs.readFileSync(path.join(dir, "pubkey.pem"), "utf8");

const got = canonicalJson(manifest);
if (got !== canonical) {
  console.error("canonical mismatch");
  console.error("  js : " + got);
  console.error("  kat: " + canonical);
  process.exit(1);
}
const ok = await verifyEs256(
  pubkey,
  new TextEncoder().encode(got),
  manifest.signature,
);
if (!ok) {
  console.error("ES256 verification failed against the rise-core fixture");
  process.exit(1);
}
console.log("KAT ok: canonical JSON + ES256 match the rise-core fixture");

const edManifest = {
  rise_version: 1,
  frame_hashes: [],
  tile_hashes: [],
  merkle_root: "n/a",
  sig_alg: "Ed25519",
  timestamp_ms: 0,
  device_id: "kat",
  assurance: "silver",
  signature: "00".repeat(64),
};
const edRes = await verifyBadge(JSON.stringify(edManifest), new Uint8Array(), {
  pubkeyPem: pubkey,
});
if (edRes.badge !== "red") {
  console.error(
    "Ed25519 + pubkey must fail closed (got badge: " + edRes.badge + ")",
  );
  process.exit(1);
}
console.log("Fail-closed ok: Ed25519 + pubkey rejected (matches Python badge tree)");

const expectedSources = ["screen", "obs", "virtual", "external_usb"];
if (
  JSON.stringify(Object.keys(NON_CAMERA_SOURCES).sort()) !==
  JSON.stringify(expectedSources.slice().sort())
) {
  console.error(
    "non-camera class set drifted from core CAPTURE_SOURCES: " +
      Object.keys(NON_CAMERA_SOURCES),
  );
  process.exit(1);
}
const plainManifest = {
  rise_version: 1,
  frame_hashes: [],
  tile_hashes: [],
  merkle_root: "n/a",
  sig_alg: "ES256",
  timestamp_ms: 0,
  device_id: "kat-noncamera",
  assurance: "silver",
  signature: "00".repeat(64),
};
for (const src of expectedSources) {
  const res = await verifyBadge(
    JSON.stringify({ ...plainManifest, capture_source: src }),
    new Uint8Array(),
  );
  if (res.captureSource !== src || !res.captureNote) {
    console.error(`missing capture disclaimer for ${src}: ${JSON.stringify(res)}`);
    process.exit(1);
  }
}
const clean = await verifyBadge(JSON.stringify(plainManifest), new Uint8Array());
if (clean.captureSource || clean.captureNote) {
  console.error("plain camera seal must not carry capture fields");
  process.exit(1);
}
console.log("Non-camera labels ok: 4 classes disclaimed, plain seal clean");
