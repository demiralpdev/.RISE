# .rise

.rise is a thin trust layer that seals every camera frame with hardware-backed proof; bindings capture the frame and hand it to `core/`, verify-web shows the result as a badge.

- Roadmap: ROADMAP.md
- Shells: `bindings/` (android, ios, macos, windows, linux)
- Verifier: `verify-web/`
- **Public verifier: https://demiralpdev.github.io/.RISE/** (client-side, no server — files never leave the browser)

## Quickstart

```bash
# core CLI
cd core
cargo build
./target/debug/rise-core --help

# verifier tests
cd ../verify-web
.venv/bin/python -m pytest tests/ -q
```
