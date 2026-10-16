# RISE — OBS Auto-Seal

`rise-auto-seal.py` is an OBS Studio Python script. After every recording
stops, it automatically seals the video with cryptographic proof by running:

```sh
rise-core video <recording> --device obs --key <key> --out <recording>.rvid
```

MP4-family recordings become self-contained, playable `.rvid` files
(manifest embedded); other containers keep a `.rise` JSON sidecar.
Drop the file at **riseproof.me** to verify.

## Install

1. Build or download `rise-core` so the script can find it:
   - `<repo>/core/target/release/rise-core` (auto-detected), **or**
   - `~/.rise/bin/rise-core` (fallback), **or**
   - anywhere on `PATH` as `rise-core`.
2. Make sure `ffmpeg` is on `PATH` (`ffmpeg -version`).
3. Open OBS → **Tools → Scripts**.
4. Click **+**, select `bindings/obs/rise-auto-seal.py`.
5. Confirm the script description reads
   *"RISE — Auto-seal recordings with cryptographic proof"*.
6. Hit **Start Recording**, then **Stop Recording**.
7. Check the OBS log (**Help → Log Files → View Current Log**) for:
   `RISE: sealed ✓  manifest: ... (verify at riseproof.me)`.

## Python path setup (macOS)

OBS on macOS ships without Python — you must point it at yours:

1. Install Python 3.6+: `brew install python3` (note the install path,
   e.g. `/opt/homebrew/Frameworks/Python.framework/Versions/3.x`).
2. OBS → **Tools → Scripts → Python Settings**.
3. Set **Python Install Path** to that framework directory
   (the folder containing `Python`/`libpython3.x.dylib`).
4. Restart OBS. If the Scripts panel shows the RISE script without a
   "Python not found" banner, you're good.

Linux/Windows: install matching 64-bit Python 3.6+ and set the same
Python Settings path if OBS complains.

## What it does

- Hooks `OBS_FRONTEND_EVENT_RECORDING_STOPPED` via
  `obs_frontend_add_event_callback`.
- Looks up the finished file via `obs_frontend_get_last_recording()`
  (fallback: global output config `path`/`url`).
- Seals in a background thread so the OBS UI never freezes.
- Reuses `~/.rise/keys/obs-seal.key` (legacy) or `~/.rise/keys/obs.key`;
  auto-generates a fresh ES256 key with `rise-core keygen` on first run.
- Logs every step with `script_log(LOG_INFO, ...)` — success, failure,
  or missing binary. It **never crashes OBS**: all errors are caught
  and reported in the log.

## Verify at riseproof.me

1. Find `<your-recording>.rvid` (MP4-family) or `<name>.mkv.rise` next to the video.
2. Open **riseproof.me**, drop the file — verification runs in your browser.
3. A badge = frames + signature verified against the sealed hash chain.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `rise-core binary not found` | Place it at `~/.rise/bin/rise-core` or build `core/` |
| `output path is unknown` | Set OBS Settings → Output → Recording Path |
| `sealing FAILED` | Check the log lines after it for `rise-core` stderr; ensure `ffmpeg` is installed |
| `Python not found` banner | Fix Python Install Path (see macOS section), restart OBS |
