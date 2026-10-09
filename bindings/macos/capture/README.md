# macOS Capture (minimal)

Captures one frame -> `../../target/frame-001.jpg` (i.e. `bindings/target/`).
Priority: **Rapoo USB** if present, otherwise FaceTime HD.

## Run

```bash
cd bindings/macos/capture
./capture.sh
```

Alternative (pure Swift, no ffmpeg):

```bash
swiftc capture.swift -o capture-swift
./capture-swift
```

## Permission

macOS asks for camera permission on first run. If denied:

**System Settings -> Privacy & Security -> Camera -> allow Terminal**, then run again.

## Files

- `capture.sh` — 1 frame via ffmpeg (recommended)
- `capture.swift` — 1 frame via AVCapture (if ffmpeg is missing)
- `core.h` — FFI header stub for `core/`
- `capture.log` — ffmpeg output of the last run (debugging)
