# Publishing @rise/cli — checklist + steps

> Do NOT run `npm publish` from automation. A human with an npm account runs
> the steps below. (Anyone can create a free account at https://www.npmjs.com/signup —
> no age/payment barrier beyond a valid email.)

## Pre-publish verification (done 2026-10-11, all green unless noted)

- [x] `package.json`: `name` = `@rise/cli`, `version` = `0.2.0`,
      `bin.rise` = `./rise.js`. Valid JSON, `npm pack --dry-run` succeeds.
- [x] `npm pack --dry-run` (in `package/`): includes `install.js`,
      `package.json`, `rise.js` (+ `README.md` after it was added).
      `bin/` is excluded via `.gitignore` — correct, binaries download at
      install time.
- [x] `npm view @rise/cli` → **404 Not Found**: the name is available,
      nothing published yet under this scope/name.
- [x] Download URL in `install.js`:
      `https://github.com/demiralpdev/.RISE/releases/latest/download/rise-core-{target}`
      — repo + path correct, and `latest` currently resolves to **v0.2.0**
      (`gh release view --json tagName` → `v0.2.0`).
- [⚠️] Release asset coverage (v0.2.0 has 3 of the 5 binaries `install.js` asks for):
  - ✅ `rise-core-aarch64-apple-darwin` (macOS arm64)
  - ✅ `rise-core-aarch64-unknown-linux-musl` (Linux arm64)
  - ✅ `rise-core-x86_64-unknown-linux-musl` (Linux x64)
  - ❌ `rise-core-x86_64-apple-darwin` (macOS x64 — falls back to source build)
  - ❌ `rise-core-x86_64-pc-windows-msvc.exe` (Windows x64 — falls back to source build)
  - Fallback is graceful (no hard failure), but consider uploading the two
    missing binaries to the v0.2.0 release before publishing for full coverage.
- [ℹ️] `npm pack` warns "No .npmignore file found, using .gitignore" — harmless;
      `bin/` exclusion already works via `.gitignore`.

## Exact publish steps (human, in order)

```bash
cd package/

# 1. Sanity check what will be published (must succeed, no errors)
npm pack --dry-run

# 2. Create a free account once (browser): https://www.npmjs.com/signup
#    then log in from the CLI:
npm login

# 3. Confirm you're logged in as the right user
npm whoami

# 4. (First release of a scoped package only — required, otherwise 402/403)
npm publish --access public

# 5. Verify it went live (may take a few minutes to appear)
npm view @rise/cli version   # expect: 0.2.0

# 6. Smoke-test from a clean shell
npm install -g @rise/cli && rise --help
```

Later versions: bump `version` in `package.json`, re-run step 1, then
`npm publish --access public` again (no `--access` needed after the first
publish, but harmless to keep).
