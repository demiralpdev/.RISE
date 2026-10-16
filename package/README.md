# @univerisr-ai/rise-cli

Photo/video authenticity CLI: hash, sign, stamp, verify, pack, unpack.

This package wraps the `rise-core` Rust binary. On install it downloads a
prebuilt binary from
[GitHub Releases](https://github.com/demiralpdev/.RISE/releases) for your
platform. If no prebuilt binary exists for your platform, it falls back to
building from source (requires the Rust toolchain — see below).

Main repo: **https://github.com/demiralpdev/.RISE**

## Install

```bash
npm install -g @univerisr-ai/rise-cli
rise --help
```

Requirements: Node.js (for the installer/wrapper only). No Rust needed unless
your platform has no prebuilt binary (see
[Supported platforms](#supported-platforms)).

## Updates

Only the latest version is supported. npm-installed copies check the registry
for a newer release at most once per day and refuse to run when outdated:

```bash
npm install -g @univerisr-ai/rise-cli@latest
```

No internet connection? The check is skipped and the CLI keeps working —
offline verification is never blocked.

## Usage

```bash
# Hash a frame (SHA256 + tile Merkle root)
rise hash path/to/frame.jpg

# Create a device keypair (private key written with mode 0600)
rise keygen my-device

# Sign a frame into a manifest (stamp-then-sign with a live TSA)
rise sign path/to/frame.jpg \
  --key my-device.key \
  --alg es256 \
  --tsa https://freetsa.org/tsr \
  --out frame.manifest.json

# Verify a manifest against its frame
rise verify frame.manifest.json path/to/frame.jpg \
  --pubkey my-device.pub

# Seal the manifest into the JPEG, then read it back
rise pack path/to/frame.jpg frame.manifest.json --out sealed.jpg
rise unpack sealed.jpg
```

Post-quantum dual signing (ML-DSA-65 alongside the classical signature):

```bash
rise sign path/to/frame.jpg \
  --key my-device.key \
  --pqc-key my-device.pqc.seed \
  --out frame.manifest.json

rise verify frame.manifest.json path/to/frame.jpg \
  --pubkey my-device.pub \
  --pqc-pub my-device.pqc.pub
```

Verification is fail-closed: a missing manifest means untrusted, a revoked or
unreachable trust anchor means unknown — never green.

## Supported platforms

Prebuilt binaries are fetched from
`https://github.com/demiralpdev/.RISE/releases/latest/download/rise-core-{target}`:

| Platform | Binary | Status (v0.2.5) |
|---|---|---|
| macOS arm64 | `rel-rise-core-aarch64-apple-darwin` | ✅ prebuilt |
| macOS x64 | `rel-rise-core-x86_64-apple-darwin` | ⚠️ source build |
| Linux x64 | `rel-rise-core-x86_64-unknown-linux-musl` | ✅ prebuilt |
| Linux arm64 | `rel-rise-core-aarch64-unknown-linux-musl` | ✅ prebuilt |
| Windows x64 | `rel-rise-core-x86_64-pc-windows-gnu.exe` | ✅ prebuilt |

⚠️ = installer falls back to `cargo build --release` from a cloned repo.
To build from source manually:

```bash
git clone https://github.com/demiralpdev/.RISE.git
cd .RISE/core && cargo build --release
```

## Links

- Repo + docs: https://github.com/demiralpdev/.RISE
- Releases: https://github.com/demiralpdev/.RISE/releases
- Public verifier (runs fully client-side): https://riseproof.me
- Security policy: https://github.com/demiralpdev/.RISE/blob/main/SECURITY.md

## License

Apache-2.0.
