"""
RISE — Auto-seal OBS recordings with cryptographic proof.

Install: OBS -> Tools -> Scripts -> + -> select this file.
After every recording stops, the script runs:
    rise-core video <recording> --device obs --key <key> --out <recording>.rvid
and writes a self-contained, playable .rvid next to the video (MP4-family
recordings) or a manifest-JSON .rise sidecar (other containers).

Requires: Python 3.6+ in OBS (macOS: set Python path in Scripts settings),
          rise-core binary (auto-detected), ffmpeg on PATH.
Docs: https://obsproject.com/docs/scripting.html
"""

import os
import subprocess
import threading

try:
    import obspython as obs
except ImportError:
    # Allows `python3 -c "import ast; ..."` syntax checks outside OBS.
    obs = None


# ---------------------------------------------------------------------------
# Config / key management
# ---------------------------------------------------------------------------

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
RISE_HOME = os.path.expanduser("~/.rise")
KEY_PATH = os.path.join(RISE_HOME, "keys", "obs.key")
# Legacy key created by seal-recording.sh — reuse if the new one is missing.
LEGACY_KEY_PATH = os.path.join(RISE_HOME, "keys", "obs-seal.key")

CANDIDATE_BINARIES = [
    os.path.normpath(os.path.join(SCRIPT_DIR, "..", "..", "core",
                                  "target", "release", "rise-core")),
    os.path.join(RISE_HOME, "bin", "rise-core"),
    "rise-core",  # PATH fallback
]


def find_rise_core():
    """Return a working rise-core binary path, or None if not found."""
    for candidate in CANDIDATE_BINARIES:
        if os.sep in candidate:
            if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
                return candidate
        else:
            # Bare command name — check PATH.
            for d in os.environ.get("PATH", "").split(os.pathsep):
                full = os.path.join(d, candidate)
                if os.path.isfile(full) and os.access(full, os.X_OK):
                    return candidate
    return None


def ensure_key():
    """Ensure a signing key exists. Returns key path or None.

    Reuses ~/.rise/keys/obs-seal.key (legacy) or ~/.rise/keys/obs.key.
    Otherwise auto-generates a fresh ES256 key via `rise-core keygen`.
    """
    os.makedirs(os.path.join(RISE_HOME, "keys"), exist_ok=True)
    if os.path.isfile(KEY_PATH):
        return KEY_PATH
    if os.path.isfile(LEGACY_KEY_PATH):
        return LEGACY_KEY_PATH

    binary = find_rise_core()
    if binary is None:
        log("RISE: no signing key and rise-core not found — "
            "cannot auto-generate key at %s" % KEY_PATH)
        return None
    try:
        subprocess.run([binary, "keygen", "--out", KEY_PATH],
                       capture_output=True, timeout=60, check=True)
        log("RISE: generated new signing key at %s" % KEY_PATH)
        return KEY_PATH
    except (subprocess.SubprocessError, OSError) as e:
        log("RISE: key generation failed: %s" % e)
        return None


def log(message):
    """Log to OBS script log when inside OBS, else stdout."""
    if obs is not None:
        obs.script_log(obs.LOG_INFO, message)
    else:
        print(message)


# ---------------------------------------------------------------------------
# Sealing
# ---------------------------------------------------------------------------

def seal_recording(recording_path):
    """Run `rise-core video` on a finished recording. Safe to call anywhere."""
    if not recording_path or not os.path.isfile(recording_path):
        log("RISE: recording file not found, skipping: %s" % recording_path)
        return

    binary = find_rise_core()
    if binary is None:
        log("RISE: rise-core binary not found (tried %s). "
            "Install it or place it at ~/.rise/bin/rise-core."
            % ", ".join(CANDIDATE_BINARIES))
        return

    key = ensure_key()
    if key is None:
        log("RISE: no signing key available — cannot seal %s"
            % recording_path)
        return

    # MP4-family recordings seal into a self-contained playable .rvid
    # (manifest embedded as a uuid box); other containers keep the
    # manifest-JSON .rise sidecar until converted to MP4.
    ext = os.path.splitext(recording_path)[1].lower()
    sealed = recording_path + (".rvid" if ext in (".mp4", ".m4v", ".mov")
                               else ".rise")
    cmd = [binary, "video", recording_path,
           "--device", "obs", "--key", key, "--out", sealed]
    log("RISE: sealing %s ..." % recording_path)
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True,
                              timeout=600)
    except subprocess.TimeoutExpired:
        log("RISE: sealing timed out for %s" % recording_path)
        return
    except OSError as e:
        log("RISE: failed to launch rise-core: %s" % e)
        return

    if proc.returncode == 0 and os.path.isfile(sealed):
        log("RISE: sealed ✓  %s  (verify at riseproof.me)" % sealed)
    else:
        log("RISE: sealing FAILED (exit %d): %s"
            % (proc.returncode, (proc.stderr or proc.stdout or "").strip()))


def seal_async(recording_path):
    """Seal in a background thread so the OBS UI never blocks."""
    t = threading.Thread(target=seal_recording, args=(recording_path,),
                         daemon=True, name="rise-auto-seal")
    t.start()


# ---------------------------------------------------------------------------
# OBS event plumbing
# ---------------------------------------------------------------------------

def get_last_recording_path():
    """Best-effort lookup of the most recent recording output path.

    Prefers the frontend API (obs_frontend_get_last_recording /
    obs_frontend_get_global_output_config); falls back to None so the
    caller logs a clear message instead of crashing.
    """
    if obs is None:
        return None
    try:
        if hasattr(obs, "obs_frontend_get_last_recording"):
            path = obs.obs_frontend_get_last_recording()
            if path:
                return path
    except Exception as e:  # noqa: BLE001 — never crash OBS
        log("RISE: obs_frontend_get_last_recording failed: %s" % e)
    try:
        if hasattr(obs, "obs_frontend_get_global_output_config"):
            config = obs.obs_frontend_get_global_output_config()
            if config is not None and hasattr(obs, "obs_data_get_string"):
                for key in ("path", "url"):
                    val = obs.obs_data_get_string(config, key)
                    if val:
                        return val
    except Exception as e:  # noqa: BLE001 — never crash OBS
        log("RISE: global output config lookup failed: %s" % e)
    return None


def on_frontend_event(event):
    """Callback registered via obs_frontend_add_event_callback."""
    try:
        stopped = obs.OBS_FRONTEND_EVENT_RECORDING_STOPPED
    except AttributeError:
        return
    if event != stopped:
        return
    path = get_last_recording_path()
    if not path:
        log("RISE: recording stopped but output path is unknown — "
            "cannot seal. Check OBS Settings → Output → Recording Path.")
        return
    log("RISE: recording stopped: %s" % path)
    seal_async(path)


# ---------------------------------------------------------------------------
# OBS script lifecycle (required function names per obspython API)
# ---------------------------------------------------------------------------

def script_description():
    return ("RISE — Auto-seal recordings with cryptographic proof\n\n"
            "Automatically runs `rise-core video` on each finished "
            "recording and writes a .rise manifest next to it.\n"
            "Verify proofs at riseproof.me.")


def script_load(settings):
    if obs is None:
        return
    try:
        obs.obs_frontend_add_event_callback(on_frontend_event)
        log("RISE: auto-seal loaded — will seal each recording on stop. "
            "Binary: %s" % (find_rise_core() or "NOT FOUND"))
    except Exception as e:  # noqa: BLE001 — never crash OBS
        try:
            obs.script_log(obs.LOG_WARNING,
                           "RISE: failed to register event callback: %s" % e)
        except Exception:
            pass


def script_unload():
    if obs is None:
        return
    try:
        obs.obs_frontend_remove_event_callback(on_frontend_event)
    except Exception:
        pass
    log("RISE: auto-seal unloaded.")
