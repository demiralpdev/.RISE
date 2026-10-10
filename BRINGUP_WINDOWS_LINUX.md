# Bring-up: Windows PC + Linux laptop (.rise 0.2.0)

Two separate machines, two separate sheets. Do them in any order.
Paste back the outputs listed at the end of each sheet.

Repo: `.rise` public repo, version 0.2.0.
Layout this doc assumes (verified in-repo):

- `core/` — Rust crate `rise-core`, binary `rise-core` (`core/Cargo.toml`)
- `bindings/linux/capture/capture.sh` — V4L2 frame capture via `ffmpeg -f v4l2`
- `bindings/linux/README.md` — Linux is a thin shell, silver ceiling
- `bindings/windows/capture/capture.cpp` — STUB, not compiled/tested upstream
- `bindings/windows/README.md` — Windows thin shell, MediaFoundation path

---

## Sheet A — Windows PC

### A1. Install Rust (rustup)

In PowerShell:

```powershell
winget install --id Rustlang.Rustup -e
# close + reopen PowerShell, then:
rustc --version
cargo --version
```

Expected (VALID/green): both commands print a version, e.g. `rustc 1.x.y`, `cargo 1.x.y`.

### A2. Clone the repo

```powershell
cd $HOME
git clone <PASTE-YOUR-REPO-URL-HERE> .RISE
cd .RISE
dir
```

You should see `core/`, `bindings/`, `verify-web/`, `README.md`.

### A3. Build + test core

```powershell
cd $HOME\.RISE\core
cargo build
cargo test
.\target\debug\rise-core.exe --help
```

### A4. Camera check (no repo camera script on Windows)

`bindings/windows/capture/capture.cpp` is a **stub** — there is nothing to
compile or run on this leg yet. Just prove a camera exists:

```powershell
# 1) OS-level check: open the Camera app, confirm you see video.
# 2) List imaging devices:
Get-PnpDevice -Class Camera, Image | Select-Object Status, Class, FriendlyName
```

Optional (only if you have ffmpeg installed):

```powershell
ffmpeg -list_devices true -f dshow -i dummy 2>&1 | Select-String "Camera|video"
```

### A5. What VALID/green looks like

- `cargo build` ends with `Finished` and no `error`.
- `cargo test` ends with `test result: ok. N passed; 0 failed` (N may vary by version).
- `rise-core.exe --help` prints usage (subcommands include `sign`-family; paste whatever it prints).
- `Get-PnpDevice` lists at least one camera with `Status = OK`.

### A6. Paste back (Windows)

Paste all of this into one message:

1. `rustc --version` + `cargo --version` output
2. `cargo test` tail (last ~15 lines)
3. `rise-core.exe --help` output
4. `Get-PnpDevice -Class Camera, Image ...` output (or "Camera app shows video: yes/no")
5. Windows version: output of `winver` or `[System.Environment]::OSVersion.Version`

---

## Sheet B — Linux laptop

### B1. Install Rust + ffmpeg + v4l2-utils

Debian/Ubuntu:

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version && cargo --version

# camera tools
sudo apt-get update
sudo apt-get install -y ffmpeg v4l-utils
ffmpeg -version | head -2
v4l2-ctl --version
```

Fedora:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
sudo dnf install -y ffmpeg v4l-utils
```

### B2. Clone the repo

```bash
cd ~
git clone <PASTE-YOUR-REPO-URL-HERE> .RISE
cd .RISE
ls
```

You should see `core/`, `bindings/`, `verify-web/`, `README.md`.

### B3. Build + test core

```bash
cd ~/​.RISE/core
cargo build
cargo test
./target/debug/rise-core --help
```

(This matches `README.md` Quickstart exactly: `cd core`, `cargo build`,
`./target/debug/rise-core --help`.)

### B4. Camera check + capture script

```bash
# device listing (this is what capture.sh parses; USB cameras win)
v4l2-ctl --list-devices
ls -l /dev/video*

# run the capture (path verified: bindings/linux/capture/capture.sh)
cd ~/.RISE
bash bindings/linux/capture/capture.sh
ls -lh bindings/target/frame-001.jpg
```

Notes on what the script does (from reading it):

- Output is `bindings/target/frame-001.jpg` (script default `../../target/frame-001.jpg`
  relative to `bindings/linux/capture/`).
- Tries `ffmpeg -f v4l2 -framerate 30 -video_size 1280x720`, retries with
  default settings on failure; failures land in `bindings/linux/capture/capture.log`.
- If `core/target/debug/rise-core` is built, it also writes
  `bindings/target/frame-001.manifest.json` via `rise-core sign <frame> --device <name>`.

### B5. What VALID/green looks like

- `cargo test` in `core/`: `test result: ok. N passed; 0 failed`.
- `v4l2-ctl --list-devices` shows at least one block ending in `/dev/videoN`.
- `capture.sh` prints `Camera device: /dev/videoN (...)` and an `ls -lh` line
  for `frame-001.jpg` with a non-zero size (e.g. `... 45K ... frame-001.jpg`).
- No `ERROR: no suitable camera found` and no `ERROR: could not capture a frame`.

### B6. Paste back (Linux)

Paste all of this into one message:

1. `rustc --version` + `cargo --version` output
2. `ffmpeg -version | head -2` + `v4l2-ctl --version`
3. `cargo test` tail (last ~15 lines, run in `core/`)
4. Full `v4l2-ctl --list-devices` output
5. Full `bash bindings/linux/capture/capture.sh` output + `ls -lh bindings/target/frame-001.jpg`
6. If a `.manifest.json` was written: `cat bindings/target/frame-001.manifest.json`
7. On failure: `cat bindings/linux/capture/capture.log` (last ~30 lines) + `ls -l /dev/video*`

---

## Troubleshooting (both machines)

| # | Symptom | Cause | Fix |
|---|---------|-------|-----|
| 1 | `cargo: command not found` | shell predates rustup | close + reopen terminal, or `source "$HOME/.cargo/env"` (Linux) / reopen PowerShell (Windows) |
| 2 | `ERROR: no suitable camera found` (Linux) | no `/dev/video*` node | plug in USB camera, then `ls -l /dev/video*` and `v4l2-ctl --list-devices` |
| 3 | `ERROR: could not capture a frame` / empty jpg (Linux) | device busy or permission denied | close apps using the camera (browser/Zoom), check `capture.log`, then permission row below |
| 4 | `ls -l /dev/video0` shows no read access (Linux) | user not in `video` group | `sudo usermod -aG video $USER`, log out/in, retry (this hint is also printed by the script) |
| 5 | No camera in `Get-PnpDevice` / Camera app black (Windows) | privacy switch or driver | Settings > Privacy & security > Camera: allow desktop apps; check physical shutter switch; update driver in Device Manager |

Scope note: Linux leg targets **silver at most** (per `bindings/linux/README.md`
gold-gate checklist) and Windows capture is currently a stub — this bring-up
only proves toolchain + camera + `core/` tests green on each machine.
