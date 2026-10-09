//! RISE CLI iskeleti.

use clap::{Parser, Subcommand};
use std::fs;
use rise_core::{create_manifest, sha256_frame, sign_manifest, verify_manifest};

/// Komut satırı giriş noktası.
#[derive(Parser)]
#[command(name = "rise-core", about = "RISE iskelet CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

/// Alt komutlar.
#[derive(Subcommand)]
enum Cmd {
    /// Dosyanın SHA256 özetini basar.
    Hash { /// Okunacak dosya yolu.
        file: String },
    /// Örnek manifestoyu imzalar.
    Sign,
    /// Örnek manifestoyu doğrular.
    Verify,
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        // kare dosyası özeti
        Cmd::Hash { file } => {
            // TODO: dosyayı parçalı oku.
            match fs::read(&file) {
                Ok(b) => println!("{}", sha256_frame(&b)),
                Err(e) => eprintln!("okuma hatası: {e}"),
            }
        }
        // imza iskeleti
        Cmd::Sign => {
            // örnek kareyle besle
            let m = create_manifest(b"ornek", "cli", 0);
            println!("{}", sign_manifest(&m));
        }
        // doğrulama iskeleti
        Cmd::Verify => {
            // aynı kareyle denetle
            let kare = b"ornek";
            let m = create_manifest(kare, "cli", 0);
            println!("{}", verify_manifest(&m, kare));
        }
    }
}
