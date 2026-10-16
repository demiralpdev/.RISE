# Publishing @univerisr-ai/rise-cli — checklist + steps

> Do NOT run `npm publish` from automation. A human with an npm account runs
> the steps below. (Anyone can create a free account at https://www.npmjs.com/signup —
> no age/payment barrier beyond a valid email.)

## Status

- `@univerisr-ai/rise-cli@0.2.1` is LIVE on npm (published 2026-10-16;
  scoped releases use `npm publish --access public`). It shipped the fixed
  installer (`rel-` asset names — 0.2.0's downloads 404ed into the silent
  source-build fallback), the corrected README, `bin.rise` normalized to
  `rise.js`, and a `files` allowlist (no `PUBLISH.md` in the tarball).
  `latest` points at `0.2.1`.
- `0.2.2` is prepared in this repo and NOT yet published. Changes vs 0.2.1:
  - update gate in `rise.js`: npm-installed copies check the registry at
    most once per day and refuse to run when outdated — only the latest
    version is supported; offline runs proceed. Enforcement starts with the
    version that ships the gate (0.2.2+; older installs cannot be
    retrofitted).
  - README "Updates" section documents the policy.
- `package.json`: `name` = `@univerisr-ai/rise-cli`, `version` = `0.2.2`,
  `bin.rise` = `rise.js`. `npm publish --dry-run` is warning-free and ships
  `install.js`, `package.json`, `rise.js`, `README.md` (`bin/` is excluded —
  binaries download at install time).
- Download URL in `install.js`: `.../releases/latest/download/{asset}` where
  the asset names carry the release `rel-` prefix (e.g.
  `rel-rise-core-aarch64-apple-darwin`); `latest` currently resolves to
  **v0.2.5**.
- v0.2.5 asset coverage (macOS x64 has no asset; the installer falls back to a
  source build — graceful, no hard failure):
  - ✅ `rel-rise-core-aarch64-apple-darwin` (macOS arm64)
  - ✅ `rel-rise-core-x86_64-unknown-linux-musl` (Linux x64)
  - ✅ `rel-rise-core-aarch64-unknown-linux-musl` (Linux arm64)
  - ✅ `rel-rise-core-x86_64-pc-windows-gnu.exe` (Windows x64)
  - ❌ macOS x64 (`rel-rise-core-x86_64-apple-darwin` — source build)

## Exact publish steps (human, in order)

```bash
cd package/

# 1. Sanity check what will be published (must succeed, no errors)
npm pack --dry-run

# 2. Log in once (account: https://www.npmjs.com/signup)
npm login

# 3. Confirm you're logged in as the right user
npm whoami

# 4. Publish (first release of a scoped package needs --access public)
npm publish --access public

# 5. Verify it went live (may take a few minutes to appear)
npm view @univerisr-ai/rise-cli version   # expect: the package.json version

# 6. Smoke-test from a clean shell
npm install -g @univerisr-ai/rise-cli && rise --help
```

Later versions: bump `version` in `package.json`, re-run step 1, then
`npm publish` again (`--access public` is only required for the first
publish of a scoped package; keeping it is harmless). Keep `bin.rise` as
`rise.js` (no `./` prefix).
