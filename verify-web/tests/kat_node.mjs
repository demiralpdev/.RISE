/* KAT: locks the JS canonicalizer + ES256 verify to a real rise-core artifact.
 * Fixture: tests/vectors (signed by `rise-core sign`, throwaway audit key).
 * Run: node verify-web/tests/kat_node.mjs */
import fs from "node:fs";
import path from "node:path";
import { webcrypto } from "node:crypto";
import { fileURLToPath } from "node:url";
import { canonicalJson, verifyEs256 } from "../static/verify.js";

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
