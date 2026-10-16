#!/usr/bin/env node
/* @univerisr-ai/rise-cli bin wrapper — runs the downloaded/source-built rise-core binary. */
const { spawn } = require("child_process");
const https = require("https");
const path = require("path");
const fs = require("fs");
const os = require("os");

const PLATFORMS = {
  "darwin-arm64": "rel-rise-core-aarch64-apple-darwin",
  "darwin-x64": "rel-rise-core-x86_64-apple-darwin",
  "linux-arm64": "rel-rise-core-aarch64-unknown-linux-musl",
  "linux-x64": "rel-rise-core-x86_64-unknown-linux-musl",
  "win32-x64": "rel-rise-core-x86_64-pc-windows-gnu.exe",
};

const key = `${process.platform}-${process.arch}`;
const binary = PLATFORMS[key];

// Search order: ./bin/ → core/target/release/ → core/target/debug/
const candidates = [
  path.join(__dirname, "bin", binary || ""),
  path.join(__dirname, "bin", "rise-core"),
  path.join(
    __dirname,
    "..",
    "core",
    "target",
    "release",
    process.platform === "win32" ? "rise-core.exe" : "rise-core",
  ),
  path.join(
    __dirname,
    "..",
    "core",
    "target",
    "debug",
    process.platform === "win32" ? "rise-core.exe" : "rise-core",
  ),
];

const exe = candidates.find((p) => fs.existsSync(p) && fs.statSync(p).size > 0);

if (!exe) {
  console.error(
    "rise-core binary not found. Run: npm install -g @univerisr-ai/rise-cli (rebuilds)",
  );
  console.error(
    "Or: git clone https://github.com/demiralpdev/.RISE.git && cd .RISE/core && cargo build --release",
  );
  process.exit(1);
}

function proceed() {
  const child = spawn(exe, process.argv.slice(2), {
    stdio: "inherit",
    windowsHide: true,
  });
  child.on("exit", (code) => process.exit(code || 0));
}

/* Update gate: only the latest published version is supported. npm-installed
 * copies check the registry at most once per day and refuse to run when they
 * are outdated. Offline / registry-unreachable runs are NOT blocked. */
const CACHE_FILE = path.join(os.homedir(), ".rise-update-check.json");
const CHECK_TTL_MS = 24 * 60 * 60 * 1000;
const LATEST_URL = "https://registry.npmjs.org/@univerisr-ai/rise-cli/latest";
const installed = require("./package.json").version;

function refuse(latest) {
  console.error(
    `rise: version ${installed} is unsupported — latest is ${latest}.`,
  );
  console.error(
    "Update before continuing: npm install -g @univerisr-ai/rise-cli@latest",
  );
  process.exit(1);
}

function cmp(a, b) {
  const pa = String(a).split(".").map(Number);
  const pb = String(b).split(".").map(Number);
  if (pa.some(Number.isNaN) || pb.some(Number.isNaN)) return null;
  for (let i = 0; i < 3; i++) {
    const x = pa[i] || 0;
    const y = pb[i] || 0;
    if (x !== y) return x < y ? -1 : 1;
  }
  return 0;
}

function evaluate(latest) {
  const rel = cmp(installed, latest);
  if (rel === -1) refuse(latest);
  proceed();
}

function readCachedLatest() {
  try {
    const c = JSON.parse(fs.readFileSync(CACHE_FILE, "utf8"));
    if (
      typeof c.checkedAt === "number" &&
      typeof c.latest === "string" &&
      Date.now() - c.checkedAt < CHECK_TTL_MS
    ) {
      return c.latest;
    }
  } catch {
    /* no cache yet / unreadable — fall through to a network check */
  }
  return null;
}

function writeCachedLatest(latest) {
  try {
    fs.writeFileSync(
      CACHE_FILE,
      JSON.stringify({ checkedAt: Date.now(), latest }),
    );
  } catch {
    /* home not writable — fine, we just re-check next time */
  }
}

function fetchLatest() {
  let done = false;
  const finish = (latest) => {
    if (done) return;
    done = true;
    if (latest) {
      writeCachedLatest(latest);
      evaluate(latest);
    } else {
      proceed(); // registry unreachable — offline mode, do not block
    }
  };
  const req = https.get(LATEST_URL, { timeout: 3000 }, (res) => {
    if (res.statusCode !== 200) {
      res.resume();
      return finish(null);
    }
    let body = "";
    res.on("data", (d) => (body += d));
    res.on("end", () => {
      try {
        finish(JSON.parse(body).version || null);
      } catch {
        finish(null);
      }
    });
  });
  req.on("timeout", () => {
    req.destroy();
    finish(null);
  });
  req.on("error", () => finish(null));
}

// Gate npm-installed copies only (bin/ layout); repo/source runs stay ungated.
if (exe.startsWith(path.join(__dirname, "bin"))) {
  const cached = readCachedLatest();
  if (cached) evaluate(cached);
  else fetchLatest();
} else {
  proceed();
}
