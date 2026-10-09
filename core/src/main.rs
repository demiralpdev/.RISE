//! RISE CLI — MS1.
//! Commands: hash (streaming), sign (manifest v1), verify (real comparison).

use clap::{Parser, Subcommand};
use rise_core::{
    create_manifest_v1, manifest_from_json, manifest_to_json, sha256_stream, sign_manifest,
    verify_manifest_v1, SigAlg,
};
use std::fs;
use std::fs::File;
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
    /// Converts a frame into a manifest v1; adds the signing stub.
    Sign {
        /// Frame file (raw or JPEG).
        file: String,
        /// Anonymous device id.
        #[arg(long, default_value = "cli-device")]
        device: String,
        /// Clean manifest JSON output path.
        #[arg(long)]
        out: Option<String>,
    },
    /// Verifies a manifest v1 against the frame by recomputation.
    Verify {
        /// Manifest JSON path (clean, no UNSIGNED prefix).
        manifest: String,
        /// Frame file.
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();
    // exit codes: 0 valid, 2 read/parse error, 3 verification failed
    let kod = match cli.cmd {
        Cmd::Hash { file } => hash_cmd(&file),
        Cmd::Sign { file, device, out } => sign_cmd(&file, &device, out.as_deref()),
        Cmd::Verify { manifest, file } => verify_cmd(&manifest, &file),
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

/// sign command: build manifest v1 + signing stub.
fn sign_cmd(file: &str, device: &str, out: Option<&str>) -> i32 {
    // MS1 is single-frame: read the file
    let Ok(b) = fs::read(file) else {
        eprintln!("read error: {file}");
        return 2;
    };
    // build manifest v1
    let m = create_manifest_v1(&[&b], device, now_ms(), SigAlg::default());
    let json = match manifest_to_json(&m) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("manifest error: {e}");
            return 2;
        }
    };
    // write the clean JSON if requested
    if let Some(path) = out {
        if let Err(e) = fs::write(path, &json) {
            eprintln!("write error: {e}");
            return 2;
        }
        println!("manifest written: {path}");
    }
    // stub signature output (replaced by COSE in MS2)
    println!("{}", sign_manifest(&json));
    0
}

/// verify command: recompute + compare.
fn verify_cmd(manifest_path: &str, file: &str) -> i32 {
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
    // full chain verification
    match verify_manifest_v1(&m, &[&b]) {
        Ok(()) => {
            println!("VALID");
            0
        }
        Err(e) => {
            println!("INVALID: {e}");
            3
        }
    }
}
