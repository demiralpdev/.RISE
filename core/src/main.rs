//! RISE CLI — MS1.
//! Commands: hash (streaming), sign (manifest v1), verify (real comparison).

use clap::{Parser, Subcommand};
use rise_core::{
    create_manifest_v1, manifest_from_json, manifest_to_json, sha256_stream, verify_manifest_v1,
    verify_signature, SigAlg,
};
use std::fs;
use std::fs::File;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Command-line entry point.
#[derive(Parser)]
#[command(name = "rise-core", about = "RISE core CLI (MS1)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

/// Subcommands.
#[derive(Subcommand)]
enum Cmd {
    /// Prints the SHA256 digest of a file (streaming, large-file safe).
    Hash {
        /// Path to the file.
        file: String,
    },
    /// Converts a frame into a manifest v1; signs it when --key is given.
    Sign {
        /// Frame file (raw or JPEG).
        file: String,
        /// Anonymous device id.
        #[arg(long, default_value = "cli-device")]
        device: String,
        /// Signing key file (PKCS#8 PEM for es256, raw 32 bytes for ed25519).
        #[arg(long)]
        key: Option<String>,
        /// Signature algorithm: es256 or ed25519.
        #[arg(long, default_value = "es256")]
        alg: String,
        /// Manifest JSON output path.
        #[arg(long)]
        out: Option<String>,
        /// Primary TSA URL for stamp-then-sign (e.g. https://freetsa.org/tsr).
        /// When given, the manifest is stamped BEFORE signing so the
        /// signature covers the timestamp token.
        #[arg(long)]
        tsa: Option<String>,
        /// Second TSA URL for cross-stamping (requires --tsa).
        #[arg(long)]
        tsa2: Option<String>,
        /// ML-DSA-65 seed file for post-quantum dual-signing.
        #[arg(long)]
        pqc_key: Option<String>,
    },
    /// Verifies a manifest v1 against the frame; checks the signature with --pubkey.
    Verify {
        /// Manifest JSON path (clean, no UNSIGNED prefix).
        manifest: String,
        /// Frame file.
        file: String,
        /// Public key file (SPKI PEM for es256, raw 32 bytes for ed25519).
        #[arg(long)]
        pubkey: Option<String>,
        /// ML-DSA-65 verification key file (1952 bytes).
        #[arg(long)]
        pqc_pub: Option<String>,
    },
    /// Generates a signing keypair: <out>.key + <out>.pub (0600 for .key).
    Keygen {
        /// Output file prefix.
        #[arg(long)]
        out: String,
        /// Signature algorithm: es256 or ed25519.
        #[arg(long, default_value = "es256")]
        alg: String,
    },
    /// Embeds the manifest into a JPEG via JUMBF APP11 (C2PA container).
    Pack {
        /// JPEG frame file.
        file: String,
        /// Manifest JSON file.
        manifest: String,
        /// Sealed output path.
        #[arg(long)]
        out: String,
    },
    /// Extracts the embedded manifest JSON from a packed JPEG.
    Unpack {
        /// Packed JPEG file.
        file: String,
    },
    /// Stamps a manifest with an RFC 3161 timestamp token (multi-TSA optional).
    Stamp {
        /// Manifest JSON path (updated in place).
        #[arg(long = "in")]
        manifest: String,
        /// Primary TSA URL (e.g. https://freetsa.org/tsr).
        #[arg(long)]
        tsa: String,
        /// Second TSA URL for cross-stamping.
        #[arg(long)]
        tsa2: Option<String>,
    },
    /// Seals a video: extracts frames with ffmpeg, hashes each frame,
    /// and builds a multi-frame manifest v1.
    Video {
        /// Input video file (e.g. input.mp4).
        input: String,
        /// Anonymous device id.
        #[arg(long, default_value = "cli-device")]
        device: String,
        /// Frames per second to extract.
        #[arg(long, default_value_t = 1)]
        fps: u32,
        /// Signing key file (PKCS#8 PEM for es256, raw 32 bytes for ed25519).
        #[arg(long)]
        key: Option<String>,
        /// Signature algorithm: es256 or ed25519.
        #[arg(long, default_value = "es256")]
        alg: String,
        /// Manifest JSON output path (prints to stdout when omitted).
        #[arg(long)]
        out: Option<String>,
        /// RFC 3161 TSA to stamp the manifest with before signing.
        #[arg(long)]
        tsa: Option<String>,
    },
    /// Seals a screenshot from the REAL OS framebuffer: captures via the
    /// OS screen-capture API, hashes immediately, signs with
    /// capture_source "screen", and packs into a sealed JPEG (.rise).
    Screen {
        /// Anonymous device id.
        #[arg(long, default_value = "cli-device")]
        device: String,
        /// Signing key file (PKCS#8 PEM for es256, raw 32 bytes for ed25519).
        #[arg(long)]
        key: Option<String>,
        /// Signature algorithm: es256 or ed25519.
        #[arg(long, default_value = "es256")]
        alg: String,
        /// Sealed output path (.rise JPEG with APP11 manifest).
        #[arg(long)]
        out: Option<String>,
    },
    /// Placeholder screen recording via ffmpeg avfoundation.
    /// --start begins a background capture, --stop ends it.
    ScreenRecord {
        /// Start a background screen recording.
        #[arg(long, default_value_t = false)]
        start: bool,
        /// Stop the background screen recording and seal it to .rvid.
        #[arg(long, default_value_t = false)]
        stop: bool,
        /// Recording output path (used with --start).
        #[arg(long, default_value = "/tmp/rise-screen-record.mov")]
        out: String,
        /// Signing key: seal the recording when --stop runs.
        #[arg(long)]
        key: Option<String>,
        /// Anonymous device id stamped into the manifest.
        #[arg(long, default_value = "auto")]
        device: String,
        /// Signature algorithm (es256|ed25519).
        #[arg(long, default_value = "es256")]
        alg: String,
        /// RFC 3161 TSA to stamp the recording manifest with.
        #[arg(long)]
        tsa: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    // exit codes: 0 valid, 2 read/parse error, 3 verification failed
    let kod = match cli.cmd {
        Cmd::Hash { file } => hash_cmd(&file),
        Cmd::Sign {
            file,
            device,
            key,
            alg,
            out,
            tsa,
            tsa2,
            pqc_key,
        } => sign_cmd(&SignArgs {
            file: &file,
            device: &device,
            key: key.as_deref(),
            alg: &alg,
            out: out.as_deref(),
            tsa: tsa.as_deref(),
            tsa2: tsa2.as_deref(),
            pqc_key: pqc_key.as_deref(),
        }),
        Cmd::Verify {
            manifest,
            file,
            pubkey,
            pqc_pub,
        } => verify_cmd(&manifest, &file, pubkey.as_deref(), pqc_pub.as_deref()),
        Cmd::Keygen { out, alg } => keygen_cmd(&out, &alg),
        Cmd::Pack {
            file,
            manifest,
            out,
        } => pack_cmd(&file, &manifest, &out),
        Cmd::Unpack { file } => unpack_cmd(&file),
        Cmd::Stamp {
            manifest,
            tsa,
            tsa2,
        } => stamp_cmd(&manifest, &tsa, tsa2.as_deref()),
        Cmd::Video {
            input,
            device,
            fps,
            key,
            alg,
            out,
            tsa,
        } => video_cmd(
            &input,
            &device,
            fps,
            key.as_deref(),
            &alg,
            out.as_deref(),
            tsa.as_deref(),
        ),
        Cmd::Screen {
            device,
            key,
            alg,
            out,
        } => screen_cmd(&device, key.as_deref(), &alg, out.as_deref()),
        Cmd::ScreenRecord {
            start,
            stop,
            out,
            key,
            device,
            alg,
            tsa,
        } => screen_record_cmd(
            start,
            stop,
            &out,
            key.as_deref(),
            &device,
            &alg,
            tsa.as_deref(),
        ),
    };
    std::process::exit(kod);
}

/// Returns the current time in milliseconds.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// hash command: streaming digest.
fn hash_cmd(file: &str) -> i32 {
    match File::open(file) {
        Ok(f) => match sha256_stream(f) {
            Ok(h) => {
                println!("{h}");
                0
            }
            Err(e) => {
                eprintln!("hash error: {e}");
                2
            }
        },
        Err(e) => {
            eprintln!("read error: {e}");
            2
        }
    }
}

/// Loads the signer for the requested algorithm from a key file.
fn load_signer(alg: &str, key: &str) -> Result<Box<dyn rise_core::signer::Signer>, String> {
    match alg {
        "es256" => rise_core::signer::P256Signer::from_pem_file(key)
            .map(|s| Box::new(s) as Box<dyn rise_core::signer::Signer>)
            .map_err(|e| e.to_string()),
        "ed25519" => rise_core::signer::Ed25519Signer::from_file(key)
            .map(|s| Box::new(s) as Box<dyn rise_core::signer::Signer>)
            .map_err(|e| e.to_string()),
        _ => Err(format!("unknown alg: {alg} (es256|ed25519)")),
    }
}

/// sign command: build manifest v1; stamp-then-sign when --tsa is given,
/// sign when --key is given. Without both flags the unsigned path is
/// byte-identical to before (capture.sh relies on it).
struct SignArgs<'a> {
    file: &'a str,
    device: &'a str,
    key: Option<&'a str>,
    alg: &'a str,
    out: Option<&'a str>,
    tsa: Option<&'a str>,
    tsa2: Option<&'a str>,
    pqc_key: Option<&'a str>,
}

fn sign_cmd(a: &SignArgs) -> i32 {
    let SignArgs {
        file,
        device,
        key,
        alg,
        out,
        tsa,
        tsa2,
        pqc_key,
    } = *a;
    // MS1 flow is single-frame: read the file
    let Ok(b) = fs::read(file) else {
        eprintln!("read error: {file}");
        return 2;
    };
    let sig_alg = match alg {
        "es256" => SigAlg::Es256,
        "ed25519" => SigAlg::Ed25519,
        _ => {
            eprintln!("unknown alg: {alg} (es256|ed25519)");
            return 2;
        }
    };
    // build manifest v1
    let mut m = create_manifest_v1(&[&b], device, now_ms(), sig_alg, "silver");
    // production order is stamp-then-sign: the token must be attached
    // BEFORE signing so the signature covers the token field.
    // Without --tsa this block is skipped and the manifest stays
    // byte-identical to the old unsigned flow.
    if tsa.is_some() || tsa2.is_some() {
        let Some(tsa_url) = tsa else {
            eprintln!("--tsa2 requires --tsa");
            return 2;
        };
        match attach_timestamp_token(&mut m, tsa_url, tsa2) {
            Ok(n) => eprintln!("stamped (token {n} bytes DER)"),
            Err(e) => {
                eprintln!("tsa error: {e}");
                return 2;
            }
        }
    }
    // sign when a key is given; unsigned is structurally complete without signature
    if let Some(key_path) = key {
        let signer = match load_signer(alg, key_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("key error: {e}");
                return 2;
            }
        };
        if let Err(e) = rise_core::sign_manifest_v1(&mut m, signer.as_ref()) {
            eprintln!("sign error: {e}");
            return 2;
        }
    }
    let json = match manifest_to_json(&m) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("manifest error: {e}");
            return 2;
        }
    };
    if let Some(path) = out {
        if let Err(e) = fs::write(path, &json) {
            eprintln!("write error: {e}");
            return 2;
        }
        println!("manifest written: {path}");
    }
    // PQC dual-sign if a seed file is provided
    if let Some(pqc_path) = pqc_key {
        let pqc = match rise_core::signer::MlDsa65Signer::from_seed_file(pqc_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("PQC key error: {e}");
                return 2;
            }
        };
        if m.signature.is_some() {
            // set alg BEFORE canonical so verify_pqc can reproduce it
            m.pqc_alg = Some("ML-DSA-65".to_string());
            m.pqc_signature = None;
            let canon = match rise_core::manifest_to_json(&m) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("PQC canonical error: {e}");
                    return 2;
                }
            };
            let pq_sig = match pqc.sign(canon.as_bytes()) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("PQC sign error: {e}");
                    return 2;
                }
            };
            m.pqc_signature = Some(
                pq_sig
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            );
            println!("PQC dual-signed (ML-DSA-65)");
            // re-serialize + re-write to include the PQC fields
            let final_json = match rise_core::manifest_to_json(&m) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("manifest error: {e}");
                    return 2;
                }
            };
            if let Some(path) = out {
                if let Err(e) = fs::write(path, &final_json) {
                    eprintln!("write error: {e}");
                    return 2;
                }
            }
        }
    }
    // state line: signed or unsigned
    if m.signature.is_some() {
        println!(
            "signed ({alg}, key: {})",
            m.signing_key_id.unwrap_or_default()
        );
    } else {
        println!("unsigned (no --key given; not evidence until signed)");
    }
    0
}

/// verify command: recompute + compare; signature check when --pubkey is given.
fn verify_cmd(manifest_path: &str, file: &str, pubkey: Option<&str>, pqc_pub: Option<&str>) -> i32 {
    // read the manifest
    let Ok(text) = fs::read_to_string(manifest_path) else {
        eprintln!("manifest read error: {manifest_path}");
        return 2;
    };
    let m = match manifest_from_json(&text) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("manifest parse error: {e}");
            return 2;
        }
    };
    // read the frame
    let Ok(b) = fs::read(file) else {
        eprintln!("frame read error: {file}");
        return 2;
    };
    // hash chain verification
    if let Err(e) = verify_manifest_v1(&m, &[&b]) {
        println!("INVALID: {e}");
        return 3;
    }
    // signature verification (fail-closed: unsigned is never VALID)
    match pubkey {
        Some(pk) => {
            let alg = match m.sig_alg {
                SigAlg::Es256 => "es256",
                SigAlg::Ed25519 => "ed25519",
            };
            let verifier = match alg {
                "es256" => rise_core::signer::P256Verifier::from_pem_file(pk)
                    .map(|v| Box::new(v) as Box<dyn rise_core::signer::Verifier>),
                _ => rise_core::signer::Ed25519Verifier::from_file(pk)
                    .map(|v| Box::new(v) as Box<dyn rise_core::signer::Verifier>),
            };
            let verifier = match verifier {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("pubkey error: {e}");
                    return 2;
                }
            };
            match verify_signature(&m, verifier.as_ref()) {
                Ok(()) => {
                    // PQC verification if a verification key is provided
                    if let Some(pqc_path) = pqc_pub {
                        let pv =
                            match rise_core::signer::MlDsa65Verifier::from_pubkey_file(pqc_path) {
                                Ok(v) => v,
                                Err(e) => {
                                    eprintln!("PQC pubkey error: {e}");
                                    return 2;
                                }
                            };
                        match rise_core::verify_pqc(&m, &pv) {
                            Ok(()) => {
                                println!("VALID (signed + PQC ML-DSA-65)");
                                0
                            }
                            Err(e) => {
                                println!("INVALID PQC: {e}");
                                3
                            }
                        }
                    } else {
                        println!("VALID (signed)");
                        0
                    }
                }
                Err(e) => {
                    println!("INVALID: {e}");
                    3
                }
            }
        }
        // no pubkey: chain holds but the signature is unverified
        None => {
            if m.signature.is_some() {
                println!("CHAIN OK, signature not checked (no --pubkey)");
                0
            } else {
                println!("CHAIN OK, UNSIGNED: not evidence");
                3
            }
        }
    }
}

/// keygen command: writes <out>.key (0600) + <out>.pub.
fn keygen_cmd(out: &str, alg: &str) -> i32 {
    let (write_result, key_path, pub_path) = match alg {
        "es256" => {
            let (priv_pem, pub_pem) = match rise_core::signer::generate_p256() {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("keygen error: {e}");
                    return 2;
                }
            };
            let key_path = format!("{out}.key");
            let pub_path = format!("{out}.pub");
            let r = fs::write(&key_path, priv_pem).and(fs::write(&pub_path, pub_pem));
            (r, key_path, pub_path)
        }
        "ed25519" => {
            let (seed, pubkey) = match rise_core::signer::generate_ed25519() {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("keygen error: {e}");
                    return 2;
                }
            };
            let key_path = format!("{out}.key");
            let pub_path = format!("{out}.pub");
            let r = fs::write(&key_path, seed).and(fs::write(&pub_path, pubkey));
            (r, key_path, pub_path)
        }
        "mldsa65" => {
            let (seed, vk) = match rise_core::signer::MlDsa65Signer::generate() {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("keygen error: {e}");
                    return 2;
                }
            };
            let key_path = format!("{out}.mldsa65.seed");
            let pub_path = format!("{out}.mldsa65.pub");
            let r = fs::write(&key_path, seed).and(fs::write(&pub_path, vk));
            (r, key_path, pub_path)
        }
        _ => {
            eprintln!("unknown alg: {alg} (es256|ed25519)");
            return 2;
        }
    };
    if let Err(e) = write_result {
        eprintln!("write error: {e}");
        return 2;
    }
    // private key: 0600 on unix; on Windows the profile ACL already scopes access
    #[cfg(unix)]
    if let Err(e) = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)) {
        eprintln!("permission error: {e}");
        return 2;
    }
    #[cfg(not(unix))]
    let _ = &key_path;
    println!("keypair written: {key_path} (0600) + {pub_path}");
    0
}

/// pack command: embed the manifest into the JPEG via JUMBF APP11.
fn pack_cmd(file: &str, manifest_path: &str, out: &str) -> i32 {
    let Ok(bytes) = fs::read(file) else {
        eprintln!("frame read error: {file}");
        return 2;
    };
    let Ok(text) = fs::read_to_string(manifest_path) else {
        eprintln!("manifest read error: {manifest_path}");
        return 2;
    };
    use rise_core::container::{self, Container};
    let sealed = match container::detect(&bytes) {
        Container::Jpeg => rise_core::jumbf::jpeg_embed(&bytes, text.as_bytes()),
        Container::Png => container::png_insert_chunk(&bytes, text.as_bytes())
            .map_err(|e| rise_core::RiseError::Crypto(format!("png pack: {e}"))),
        Container::Mp4 => container::mp4_append_box(&bytes, text.as_bytes())
            .map_err(|e| rise_core::RiseError::Crypto(format!("mp4 pack: {e}"))),
        Container::Unknown => {
            eprintln!("pack error: unsupported file type (JPEG, PNG or MP4): {file}");
            return 2;
        }
    };
    match sealed {
        Ok(sealed) => {
            if let Err(e) = fs::write(out, sealed) {
                eprintln!("write error: {e}");
                return 2;
            }
            let kind = match container::detect(&bytes) {
                Container::Jpeg => "JPEG APP11",
                Container::Png => "PNG risE chunk",
                Container::Mp4 => "MP4 rise box",
                Container::Unknown => unreachable!(),
            };
            println!("packed ({kind}): {out}");
            0
        }
        Err(e) => {
            eprintln!("pack error: {e}");
            2
        }
    }
}

/// Fetches RFC 3161 token(s) over the canonical manifest bytes (token
/// field empty at stamp time) and attaches the primary token, reusing the
/// timestamp.rs request/verify helpers. Returns the DER token length.
fn attach_timestamp_token(
    m: &mut rise_core::ManifestV1,
    tsa: &str,
    tsa2: Option<&str>,
) -> Result<usize, String> {
    // canonical bytes stamped: token field None at stamp time
    m.timestamp_token = None;
    let canon = manifest_to_json(m).map_err(|e| e.to_string())?;
    let digest = rise_core::timestamp::sha256_of(canon.as_bytes());
    let tok1 = rise_core::timestamp::request_timestamp(tsa, canon.as_bytes())
        .map_err(|e| e.to_string())?;
    rise_core::timestamp::verify_token_against_digest(&tok1, &digest)
        .map_err(|e| format!("token mismatch: {e}"))?;
    if let Some(url2) = tsa2 {
        let tok2 = rise_core::timestamp::request_timestamp(url2, canon.as_bytes())
            .map_err(|e| format!("tsa2: {e}"))?;
        rise_core::timestamp::cross_check_tokens(&tok1, &tok2, &digest)
            .map_err(|e| format!("cross-check failed: {e}"))?;
    }
    let n = tok1.len();
    use base64::Engine;
    m.timestamp_token = Some(base64::engine::general_purpose::STANDARD.encode(&tok1));
    Ok(n)
}

/// stamp command: fetch RFC 3161 token(s) over the canonical manifest
/// JSON (token field empty at stamp time) and write the base64 token back.
/// Note: stamping a SIGNED manifest invalidates its signature (the
/// signature covers the token field); prefer `sign --key --tsa` instead.
fn stamp_cmd(manifest_path: &str, tsa: &str, tsa2: Option<&str>) -> i32 {
    let Ok(text) = fs::read_to_string(manifest_path) else {
        eprintln!("manifest read error: {manifest_path}");
        return 2;
    };
    let mut m = match manifest_from_json(&text) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("manifest parse error: {e}");
            return 2;
        }
    };
    if m.signature.is_some() {
        eprintln!(
            "warn: stamping a signed manifest invalidates its signature; prefer sign --key --tsa"
        );
    }
    let n = match attach_timestamp_token(&mut m, tsa, tsa2) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("tsa error: {e}");
            return 2;
        }
    };
    let out = match manifest_to_json(&m) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("manifest error: {e}");
            return 2;
        }
    };
    if let Err(e) = fs::write(manifest_path, &out) {
        eprintln!("write error: {e}");
        return 2;
    }
    println!("stamped: {manifest_path} (token {n} bytes DER)");
    0
}

/// video command: extract frames with ffmpeg, hash each frame, and build
/// a multi-frame manifest v1. Tile hashes cover ALL frames combined, in
/// frame order (via create_manifest_v1). Signs when --key is given.
fn video_cmd(
    input: &str,
    device: &str,
    fps: u32,
    key: Option<&str>,
    alg: &str,
    out: Option<&str>,
    tsa: Option<&str>,
) -> i32 {
    // ffmpeg must be on PATH; shell out via std::process::Command
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("ffmpeg not found: install ffmpeg to seal videos (e.g. `brew install ffmpeg`)");
        return 2;
    }
    if !std::path::Path::new(input).exists() {
        eprintln!("input not found: {input}");
        return 2;
    }
    if fps == 0 {
        eprintln!("--fps must be >= 1");
        return 2;
    }
    let sig_alg = match alg {
        "es256" => SigAlg::Es256,
        "ed25519" => SigAlg::Ed25519,
        _ => {
            eprintln!("unknown alg: {alg} (es256|ed25519)");
            return 2;
        }
    };
    // unique temp dir for the extracted frames
    let tmp = std::env::temp_dir().join(format!("rise-video-{}-{}", std::process::id(), now_ms()));
    if let Err(e) = fs::create_dir_all(&tmp) {
        eprintln!("temp dir error: {e}");
        return 2;
    }
    // extract frames: ffmpeg -i input -vf fps=N frame_%04d.jpg
    let pattern = tmp.join("frame_%04d.jpg");
    let pattern_str = pattern.to_string_lossy().into_owned();
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-i",
            input,
            "-vf",
            &format!("fps={fps}"),
            &pattern_str,
        ])
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("ffmpeg failed (exit {s}); is this a valid video file?");
            let _ = fs::remove_dir_all(&tmp);
            return 2;
        }
        Err(e) => {
            eprintln!("ffmpeg error: {e}");
            let _ = fs::remove_dir_all(&tmp);
            return 2;
        }
    }
    // collect extracted frames in sorted order
    let mut paths: Vec<_> = match fs::read_dir(&tmp) {
        Ok(rd) => rd.filter_map(|e| e.ok().map(|x| x.path())).collect(),
        Err(e) => {
            eprintln!("temp read error: {e}");
            let _ = fs::remove_dir_all(&tmp);
            return 2;
        }
    };
    paths.sort();
    if paths.is_empty() {
        eprintln!("ffmpeg extracted no frames from: {input}");
        let _ = fs::remove_dir_all(&tmp);
        return 2;
    }
    // Long recordings: cap the manifest at ~256 evenly-sampled frames so
    // an hour-long capture does not bloat the sealed box (fps is a
    // sampling rate, not a claim about every frame).
    const MAX_FRAMES: usize = 256;
    if paths.len() > MAX_FRAMES {
        let step = (paths.len() / MAX_FRAMES).max(1);
        paths = paths.iter().step_by(step).cloned().collect();
    }
    // read every frame into memory for hashing
    let mut frames: Vec<Vec<u8>> = Vec::with_capacity(paths.len());
    for p in &paths {
        match fs::read(p) {
            Ok(b) => frames.push(b),
            Err(e) => {
                eprintln!("frame read error {}: {e}", p.display());
                let _ = fs::remove_dir_all(&tmp);
                return 2;
            }
        }
    }
    // clean up temp frames now that bytes are hashed in memory
    let _ = fs::remove_dir_all(&tmp);
    // build the multi-frame manifest (tiles span all frames)
    let refs: Vec<&[u8]> = frames.iter().map(|f| &f[..]).collect();
    let mut m = create_manifest_v1(&refs, device, now_ms(), sig_alg, "silver");
    // stamp-then-sign when a TSA is given; a TSA outage downgrades the
    // evidence (no RFC 3161 token) but must not lose the sealed recording
    if let Some(tsa_url) = tsa {
        match attach_timestamp_token(&mut m, tsa_url, None) {
            Ok(n) => eprintln!("stamped ({n} bytes DER, {tsa_url})"),
            Err(e) => {
                eprintln!("WARN tsa unreachable ({e}); sealing WITHOUT timestamp token");
            }
        }
    }
    // sign when a key is given
    if let Some(key_path) = key {
        let signer = match load_signer(alg, key_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("key error: {e}");
                return 2;
            }
        };
        if let Err(e) = rise_core::sign_manifest_v1(&mut m, signer.as_ref()) {
            eprintln!("sign error: {e}");
            return 2;
        }
    }
    let json = match manifest_to_json(&m) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("manifest error: {e}");
            return 2;
        }
    };
    if let Some(path) = out {
        let input_bytes = match fs::read(input) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("input re-read error: {e}");
                return 2;
            }
        };
        use rise_core::container::{self, Container};
        match container::detect(&input_bytes) {
            Container::Mp4 => {
                // Self-contained format: the .rise IS a playable MP4 with
                // the signed manifest embedded as a `rise` box.
                let sealed = match container::mp4_append_box(&input_bytes, json.as_bytes()) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("mp4 embed error: {e}");
                        return 2;
                    }
                };
                if let Err(e) = fs::write(path, &sealed) {
                    eprintln!("write error: {e}");
                    return 2;
                }
                eprintln!(
                    "sealed {} frames from {input} -> {path} (playable MP4 + embedded manifest)",
                    m.frame_hashes.len()
                );
            }
            _ => {
                // mkv/webm/flv cannot carry an ISO-BMFF box: JSON sidecar.
                if let Err(e) = fs::write(path, &json) {
                    eprintln!("write error: {e}");
                    return 2;
                }
                eprintln!(
                    "sealed {} frames from {input} -> {path} (manifest JSON; convert to MP4 for embedded packing)",
                    m.frame_hashes.len()
                );
            }
        }
    } else {
        println!("{json}");
        eprintln!("sealed {} frames from {input}", m.frame_hashes.len());
    }
    if m.signature.is_some() {
        eprintln!("signed ({alg})");
    } else {
        eprintln!("unsigned (no --key given; not evidence until signed)");
    }
    0
}

/// screen command: capture the REAL OS framebuffer via the platform
/// screen-capture API (screencapture on macOS), convert to JPEG, then run
/// the full chain: hash -> manifest (capture_source "screen") -> sign ->
/// pack (JPEG APP11). The capture tool talks to the display framebuffer,
/// which is the "real screen" proof: the pixels came from the OS
/// compositor, not from a file or an AI generator.
fn screen_cmd(device: &str, key: Option<&str>, alg: &str, out: Option<&str>) -> i32 {
    let sig_alg = match alg {
        "es256" => SigAlg::Es256,
        "ed25519" => SigAlg::Ed25519,
        _ => {
            eprintln!("unknown alg: {alg} (es256|ed25519)");
            return 2;
        }
    };
    // Unique temp dir for the raw capture plus the JPEG conversion.
    let tmp = std::env::temp_dir().join(format!("rise-screen-{}-{}", std::process::id(), now_ms()));
    if let Err(e) = fs::create_dir_all(&tmp) {
        eprintln!("temp dir error: {e}");
        return 2;
    }
    let png_path = tmp.join("capture.png");
    let jpg_path = tmp.join("capture.jpg");
    let png_str = png_path.to_string_lossy().into_owned();
    // Silent capture of the real framebuffer (no shutter sound).
    let status = Command::new("screencapture")
        .args(["-x", &png_str])
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("screencapture failed (exit {s})");
            let _ = fs::remove_dir_all(&tmp);
            return 2;
        }
        Err(e) => {
            eprintln!(
                "screencapture error: {e} (screen capture needs macOS screencapture on PATH)"
            );
            let _ = fs::remove_dir_all(&tmp);
            return 2;
        }
    }
    // APP11 packing needs JPEG: convert the PNG capture.
    let jpg_str = jpg_path.to_string_lossy().into_owned();
    let converted = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-i",
            &png_str,
            "-q:v",
            "2",
            &jpg_str,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !converted {
        // Fallback when ffmpeg is missing: macOS sips converter.
        let sips = Command::new("sips")
            .args(["-s", "format", "jpeg", &png_str, "--out", &jpg_str])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !sips {
            eprintln!("conversion failed: install ffmpeg (`brew install ffmpeg`)");
            let _ = fs::remove_dir_all(&tmp);
            return 2;
        }
    }
    let Ok(frame) = fs::read(&jpg_path) else {
        eprintln!("frame read error: {}", jpg_path.display());
        let _ = fs::remove_dir_all(&tmp);
        return 2;
    };
    let _ = fs::remove_dir_all(&tmp);
    // Build the manifest over the exact sealed bytes and tag the source.
    let mut m = create_manifest_v1(&[&frame[..]], device, now_ms(), sig_alg, "silver");
    m.capture_source = Some("screen".to_string());
    if let Some(key_path) = key {
        let signer = match load_signer(alg, key_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("key error: {e}");
                return 2;
            }
        };
        if let Err(e) = rise_core::sign_manifest_v1(&mut m, signer.as_ref()) {
            eprintln!("sign error: {e}");
            return 2;
        }
    }
    let json = match manifest_to_json(&m) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("manifest error: {e}");
            return 2;
        }
    };
    let out_path = out
        .map(str::to_string)
        .unwrap_or_else(|| format!("screen-{}.rise", now_ms()));
    match rise_core::jumbf::jpeg_embed(&frame, json.as_bytes()) {
        Ok(sealed) => {
            if let Err(e) = fs::write(&out_path, sealed) {
                eprintln!("write error: {e}");
                return 2;
            }
            eprintln!("sealed screen capture (capture_source=screen) -> {out_path}");
        }
        Err(e) => {
            eprintln!("pack error: {e}");
            return 2;
        }
    }
    if m.signature.is_some() {
        eprintln!("signed ({alg})");
    } else {
        eprintln!("unsigned (no --key given; not evidence until signed)");
    }
    0
}

/// Pid file tracking the background ffmpeg screen recording.
fn screen_record_pid_file() -> std::path::PathBuf {
    std::env::temp_dir().join("rise-screen-record.pid")
}

/// Finds the first avfoundation "Capture screen" video device index, so
/// screen recording targets the display instead of a webcam. Indices are
/// per-machine (webcams occupy the low slots), so this must be dynamic.
fn detect_avfoundation_screen() -> Option<u32> {
    let out = Command::new("ffmpeg")
        .args(["-f", "avfoundation", "-list_devices", "true", "-i", ""])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    for line in text.lines() {
        let Some(pos) = line.find("Capture screen") else {
            continue;
        };
        let Some(open) = line[..pos].rfind('[') else {
            continue;
        };
        let Some(idx) = line[open + 1..pos]
            .trim_end_matches(']')
            .trim()
            .parse()
            .ok()
        else {
            continue;
        };
        return Some(idx);
    }
    None
}

/// screen-record command: --start spawns a background ffmpeg avfoundation
/// capture into `<out>.part` (a crash never exposes a partial sealed file),
/// stashing the seal parameters in the pid file. --stop kills ffmpeg,
/// waits for the container to settle, then seals IN PROCESS into a
/// playable `.rvid` next to the recording. One command, no sidecars.
fn screen_record_cmd(
    start: bool,
    stop: bool,
    out: &str,
    key: Option<&str>,
    device: &str,
    alg: &str,
    tsa: Option<&str>,
) -> i32 {
    if start == stop {
        eprintln!("specify exactly one of --start or --stop");
        return 2;
    }
    let pid_file = screen_record_pid_file();
    if start {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            eprintln!("ffmpeg not found: install ffmpeg (`brew install ffmpeg`)");
            return 2;
        }
        if pid_file.exists() {
            eprintln!(
                "already recording (pid file {} exists); --stop first",
                pid_file.display()
            );
            return 2;
        }
        // Record to a .part name: the .rvid only appears once sealed.
        let part_path = format!("{out}.part");
        let screen_idx = detect_avfoundation_screen().unwrap_or(3);
        let input = format!("{screen_idx}:none");
        let child = Command::new("ffmpeg")
            .args([
                "-y",
                "-loglevel",
                "error",
                "-f",
                "avfoundation",
                "-pixel_format",
                "uyvy422",
                "-i",
                &input,
                "-movflags",
                "+faststart",
                "-f",
                "mov",
                &part_path,
            ])
            .spawn();
        match child {
            Ok(c) => {
                let stash = format!(
                    "{}\n{out}\n{}\n{device}\n{alg}\n{}\n",
                    c.id(),
                    key.unwrap_or(""),
                    tsa.unwrap_or("")
                );
                if let Err(e) = fs::write(&pid_file, stash) {
                    eprintln!("pid write error: {e}");
                    return 2;
                }
                eprintln!("screen recording started (pid {}) -> {part_path}", c.id());
                eprintln!("stop and seal with: rise-core screen-record --stop");
                std::mem::forget(c);
                0
            }
            Err(e) => {
                eprintln!("ffmpeg spawn error: {e}");
                2
            }
        }
    } else {
        let Ok(text) = fs::read_to_string(&pid_file) else {
            eprintln!("not recording (no pid file {})", pid_file.display());
            return 2;
        };
        let mut lines = text.lines();
        let pid: i32 = match lines.next().and_then(|l| l.trim().parse().ok()) {
            Some(p) => p,
            None => {
                eprintln!("corrupt pid file {}", pid_file.display());
                return 2;
            }
        };
        let out_path = lines.next().unwrap_or(out).to_string();
        let stashed_key = lines.next().map(str::to_string).filter(|s| !s.is_empty());
        let stashed_device = lines.next().unwrap_or("auto").to_string();
        let stashed_alg = lines.next().unwrap_or("es256").to_string();
        let stashed_tsa = lines.next().map(str::to_string).filter(|s| !s.is_empty());
        let _ = fs::remove_file(&pid_file);
        let killed = Command::new("kill")
            .arg(pid.to_string())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !killed {
            eprintln!("could not signal pid {pid}; recording may have already exited");
        }
        let part_path = format!("{out_path}.part");
        let rec_path = if std::path::Path::new(&part_path).exists() {
            part_path
        } else {
            out_path.clone()
        };
        // Let ffmpeg flush and the file settle before hashing frames.
        let mut prev: i64 = -1;
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let sz = fs::metadata(&rec_path)
                .map(|m| m.len() as i64)
                .unwrap_or(prev);
            if sz == prev {
                break;
            }
            prev = sz;
        }
        let rvid_out = std::path::Path::new(&out_path)
            .with_extension("rvid")
            .to_string_lossy()
            .into_owned();
        eprintln!("sealing {rec_path} -> {rvid_out}");
        let code = video_cmd(
            &rec_path,
            &stashed_device,
            1,
            stashed_key.as_deref(),
            &stashed_alg,
            Some(&rvid_out),
            stashed_tsa.as_deref(),
        );
        if code == 0 {
            let _ = fs::remove_file(&rec_path);
            eprintln!("done: {rvid_out}");
        } else {
            eprintln!(
                "seal failed; recording kept at {rec_path} (seal manually with rise-core video)"
            );
        }
        code
    }
}

/// Pass-through for raw JSON payloads: None when the bytes are not valid JSON.
fn json_or_none(raw: &[u8]) -> Option<Vec<u8>> {
    serde_json::from_slice::<serde_json::Value>(raw).ok()?;
    Some(raw.to_vec())
}

/// unpack command: extract the embedded manifest JSON (JPEG APP11,
/// PNG risE chunk, or MP4 rise box — the FIRST embedded manifest wins).
fn unpack_cmd(file: &str) -> i32 {
    let Ok(bytes) = fs::read(file) else {
        eprintln!("frame read error: {file}");
        return 2;
    };
    use rise_core::container::{self, Container};
    let extracted: Option<Vec<u8>> = match container::detect(&bytes) {
        // JPEG carries a JUMBF-wrapped payload (C2PA container).
        Container::Jpeg => rise_core::jumbf::jpeg_extract(&bytes)
            .and_then(|j| rise_core::jumbf::parse_json_payload(&j).ok()),
        // PNG risE chunk / MP4 rise box carry the manifest JSON raw.
        Container::Png => container::png_extract_chunk(&bytes).and_then(|raw| json_or_none(&raw)),
        Container::Mp4 => container::mp4_extract_box(&bytes).and_then(|raw| json_or_none(&raw)),
        Container::Unknown => None,
    };
    match extracted {
        Some(payload) => {
            println!("{}", String::from_utf8_lossy(&payload));
            0
        }
        None => {
            eprintln!("no embedded manifest found in: {file}");
            3
        }
    }
}

#[cfg(test)]
mod unpack_mp4_tests {
    use super::*;

    /// Minimal synthetic MP4: one `ftyp` box (detect() keys on `ftyp` at offset 4).
    fn synth_mp4() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&24u32.to_be_bytes());
        out.extend_from_slice(b"ftypisom\x00\x00\x02\x00isomiso2");
        out
    }

    fn write_tmp(name: &str, bytes: &[u8]) -> String {
        let path = std::env::temp_dir().join(format!(
            "rise-unpack-test-{}-{}-{name}",
            std::process::id(),
            name.len()
        ));
        fs::write(&path, bytes).expect("tmp write");
        path.to_string_lossy().into_owned()
    }

    fn uuid_box(uuid: &[u8; 16], payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&((payload.len() + 16 + 8) as u32).to_be_bytes());
        v.extend_from_slice(b"uuid");
        v.extend_from_slice(uuid);
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn unpack_mp4_uuid_happy_path_exit_0() {
        let manifest = br#"{"rise_version":1,"frame_hashes":[]}"#;
        let sealed =
            rise_core::container::mp4_append_box(&synth_mp4(), manifest).expect("seal");
        let p = write_tmp("happy", &sealed);
        assert_eq!(unpack_cmd(&p), 0);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn unpack_mp4_missing_uuid_exit_3() {
        let p = write_tmp("missing", &synth_mp4());
        assert_eq!(unpack_cmd(&p), 3);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn unpack_mp4_truncated_box_exit_3_no_panic() {
        let manifest = br#"{"rise_version":1}"#;
        let mut sealed =
            rise_core::container::mp4_append_box(&synth_mp4(), manifest).expect("seal");
        sealed.truncate(sealed.len() - 2); // declared box runs past EOF
        let p = write_tmp("trunc", &sealed);
        assert_eq!(unpack_cmd(&p), 3);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn unpack_mp4_wrong_uuid_ignored_exit_3() {
        let mut data = synth_mp4();
        data.extend_from_slice(&uuid_box(&[0u8; 16], b"not-rise"));
        let p = write_tmp("wrong", &data);
        assert_eq!(unpack_cmd(&p), 3);
        let _ = fs::remove_file(&p);
    }
}
