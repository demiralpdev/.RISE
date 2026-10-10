#!/usr/bin/env node
/* @rise/cli bin wrapper — runs the downloaded/source-built rise-core binary. */
const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const PLATFORMS = {
  "darwin-arm64": "rise-core-aarch64-apple-darwin",
  "darwin-x64": "rise-core-x86_64-apple-darwin",
  "linux-arm64": "rise-core-aarch64-unknown-linux-gnu",
  "linux-x64": "rise-core-x86_64-unknown-linux-gnu",
  "win32-x64": "rise-core-x86_64-pc-windows-msvc.exe",
};

const key = `${process.platform}-${process.arch}`;
const binary = PLATFORMS[key];

// Search order: ./bin/ → core/target/release/ → core/target/debug/
const candidates = [
  path.join(__dirname, "bin", binary || ""),
  path.join(__dirname, "bin", "rise-core"),
  path.join(__dirname, "..", "core", "target", "release",
    process.platform === "win32" ? "rise-core.exe" : "rise-core"),
  path.join(__dirname, "..", "core", "target", "debug",
    process.platform === "win32" ? "rise-core.exe" : "rise-core"),
];

const exe = candidates.find((p) => fs.existsSync(p) && fs.statSync(p).size > 0);

if (!exe) {
  console.error("rise-core binary not found. Run: npm install -g @rise/cli (rebuilds)");
  console.error("Or: git clone https://github.com/demiralpdev/.RISE.git && cd .RISE/core && cargo build --release");
  process.exit(1);
}

const child = spawn(exe, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: true,
});
child.on("exit", (code) => process.exit(code || 0));
