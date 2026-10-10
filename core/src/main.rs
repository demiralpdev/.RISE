//! RISE CLI — MS1.
//! Commands: hash (streaming), sign (manifest v1), verify (real comparison).

use clap::{Parser, Subcommand};
use rise_core::{
    create_manifest_v1, manifest_from_json, manifest_to_json, sha256_stream, verify_manifest_v1,
    verify_signature, SigAlg,
};
use std::fs;
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
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
        } => sign_cmd(&file, &device, key.as_deref(), &alg, out.as_deref()),
        Cmd::Verify {
            manifest,
            file,
            pubkey,
        } => verify_cmd(&manifest, &file, pubkey.as_deref()),
        Cmd::Keygen { out, alg } => keygen_cmd(&out, &alg),
        Cmd::Pack {
            file,
            manifest,
            out,
        } => pack_cmd(&file, &manifest, &out),
        Cmd::Unpack { file } => unpack_cmd(&file),
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

/// sign command: build manifest v1; sign when --key is given.
fn sign_cmd(file: &str, device: &str, key: Option<&str>, alg: &str, out: Option<&str>) -> i32 {
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
fn verify_cmd(manifest_path: &str, file: &str, pubkey: Option<&str>) -> i32 {
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
                    println!("VALID (signed)");
                    0
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
        _ => {
            eprintln!("unknown alg: {alg} (es256|ed25519)");
            return 2;
        }
    };
    if let Err(e) = write_result {
        eprintln!("write error: {e}");
        return 2;
    }
    // private key gets 0600
    if let Err(e) = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)) {
        eprintln!("permission error: {e}");
        return 2;
    }
    println!("keypair written: {key_path} (0600) + {pub_path}");
    0
}

/// pack command: embed the manifest into the JPEG via JUMBF APP11.
fn pack_cmd(file: &str, manifest_path: &str, out: &str) -> i32 {
    let Ok(jpg) = fs::read(file) else {
        eprintln!("frame read error: {file}");
        return 2;
    };
    let Ok(text) = fs::read_to_string(manifest_path) else {
        eprintln!("manifest read error: {manifest_path}");
        return 2;
    };
    match rise_core::jumbf::jpeg_embed(&jpg, text.as_bytes()) {
        Ok(sealed) => {
            if let Err(e) = fs::write(out, sealed) {
                eprintln!("write error: {e}");
                return 2;
            }
            println!("packed: {out}");
            0
        }
        Err(e) => {
            eprintln!("pack error: {e}");
            2
        }
    }
}

/// unpack command: extract the embedded manifest JSON.
fn unpack_cmd(file: &str) -> i32 {
    let Ok(jpg) = fs::read(file) else {
        eprintln!("frame read error: {file}");
        return 2;
    };
    match rise_core::jumbf::jpeg_extract(&jpg) {
        Some(jumbf) => match rise_core::jumbf::parse_json_payload(&jumbf) {
            Ok(payload) => {
                println!("{}", String::from_utf8_lossy(&payload));
                0
            }
            Err(e) => {
                eprintln!("jumbf parse error: {e}");
                2
            }
        },
        None => {
            eprintln!("no embedded manifest found in: {file}");
            3
        }
    }
}
