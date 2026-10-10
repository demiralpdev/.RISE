#!/usr/bin/env node
/* Downloads a prebuilt rise-core binary from GitHub Releases, or builds
 * from source if no release is available for the platform. */
const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const PLATFORMS = {
  "darwin-arm64": "rise-core-aarch64-apple-darwin",
  "darwin-x64": "rise-core-x86_64-apple-darwin",
  "linux-arm64": "rise-core-aarch64-unknown-linux-gnu",
  "linux-x64": "rise-core-x86_64-unknown-linux-gnu",
  "win32-x64": "rise-core-x86_64-pc-windows-msvc.exe",
};

const RELEASES_URL = "https://github.com/demiralpdev/.RISE/releases/latest/download";
const BIN_DIR = path.join(__dirname, "bin");
const key = `${process.platform}-${process.arch}`;
const binary = PLATFORMS[key];

if (!binary) {
  console.error(`No prebuilt binary for ${key}. Build from source:`);
  console.error("  git clone https://github.com/demiralpdev/.RISE.git && cd .RISE/core && cargo build --release");
  process.exit(1);
}

const localPath = path.join(BIN_DIR, binary);

async function main() {
  fs.mkdirSync(BIN_DIR, { recursive: true });

  // Try GitHub Releases first
  const url = `${RELEASES_URL}/${binary}`;
  try {
    console.log(`Downloading ${binary} from GitHub Releases...`);
    execSync(`curl -fsSL -o "${localPath}" "${url}"`, { stdio: "pipe", timeout: 30000 });
    if (fs.existsSync(localPath) && fs.statSync(localPath).size > 1000) {
      fs.chmodSync(localPath, 0o755);
      console.log(`Installed: ${localPath}`);
      return;
    }
  } catch (e) {
    console.log("No prebuilt binary available (Releases not published yet).");
  }

  // Build from source
  console.log("Building from source (requires Rust + cargo)...");
  try {
    const tmp = path.join(__dirname, "..", "..", "core");
    if (fs.existsSync(path.join(tmp, "Cargo.toml"))) {
      execSync("cargo build --release", { cwd: tmp, stdio: "inherit" });
      console.log("Built from source. Binary at core/target/release/rise-core");
      console.log("Note: `rise` CLI wrapper will fall back to the source-built binary.");
    } else {
      console.log("To build: git clone https://github.com/demiralpdev/.RISE.git && cd .RISE/core && cargo build --release");
    }
  } catch (e) {
    console.log("Build failed:", e.message?.slice(0, 100));
    console.log("Install Rust: https://rustup.rs then retry.");
  }
}

main();
