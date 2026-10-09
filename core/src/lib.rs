//! RISE çekirdek mantık — tek paylaşılan kaynak.
//! Formül: SHA256(ham kare) deklanşör anında.

use sha2::{Digest, Sha256};

/// Kare baytının SHA256 özeti (hex).
/// TODO: akışlı hash ile büyük kareleri parçalı işle.
pub fn sha256_frame(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex_encode(h.finalize())
}

/// Kareyi en fazla 4 eşit döşemeye böler.
fn dilimler(data: &[u8]) -> Vec<&[u8]> {
    // boş karede döşeme yok
    if data.is_empty() {
        return Vec::new();
    }
    // en fazla 4 parça
    let n = data.len().min(4);
    // tavan bölme ile parça boyu
    let boy = data.len().div_ceil(n);
    // eşit dilimle
    data.chunks(boy).collect()
}

/// Manifest JSON üretir (yerel, imzasız).
/// Alanlar: kare özeti, döşeme özetleri, kök, zaman, cihaz.
pub fn create_manifest(frame: &[u8], cihaz: &str, zaman: u64) -> String {
    // kare özeti
    let kare = sha256_frame(frame);
    // döşeme özetleri
    let dosemeler: Vec<String> = dilimler(frame).iter().map(|d| tile_hash(d)).collect();
    // merkle kökü
    let kok = merkle_root(&dosemeler);
    // JSON yaz
    serde_json::json!({
        "frame_hash": kare,
        "tile_hashes": dosemeler,
        "merkle_root": kok,
        "timestamp": zaman,
        "device": cihaz,
    })
    .to_string()
}

/// Manifestoyu imzalar (isimsiz iskelet).
/// TODO: gerçek anahtar ve imza ekle (çevrimdışı).
pub fn sign_manifest(manifest: &str) -> String {
    // TODO: imza formatını netleştir.
    format!("UNSIGNED:{}", manifest)
}

/// Manifestoyu yerel yeniden hesapla denetler.
/// Kare baytını özetleyip JSON ile karşılaştırır.
pub fn verify_manifest(manifest: &str, frame: &[u8]) -> bool {
    // JSON çöz
    let v: serde_json::Value = match serde_json::from_str(manifest) {
        Ok(v) => v,
        Err(_) => return false,
    };
    // beklenen alanları oku
    let Some(kare) = v.get("frame_hash").and_then(|x| x.as_str()) else {
        return false;
    };
    let Some(dizi) = v.get("tile_hashes") else {
        return false;
    };
    let Some(kok) = v.get("merkle_root").and_then(|x| x.as_str()) else {
        return false;
    };
    // kareyi yeniden özetle
    if sha256_frame(frame) != kare {
        return false;
    }
    // döşemeleri yeniden özetle
    let taze: Vec<String> = dilimler(frame).iter().map(|d| tile_hash(d)).collect();
    // diziyi tipli karşılaştır
    let bek: Vec<String> = match serde_json::from_value(dizi.clone()) {
        Ok(x) => x,
        Err(_) => return false,
    };
    if taze != bek {
        return false;
    }
    // kökü yeniden hesapla
    merkle_root(&taze) == kok
}

/// Döşeme baytının özeti (ham kare parçası).
/// Formül: SHA256(ham döşeme).
pub fn tile_hash(data: &[u8]) -> String {
    // kare özetiyle aynı formül
    sha256_frame(data)
}

/// Yaprak özetlerden Merkle kökü (hex).
/// Tek kalırsa yukarı taşınır.
pub fn merkle_root(hashes: &[String]) -> String {
    // boş küme: boş girdinin özeti
    if hashes.is_empty() {
        return sha256_frame(b"");
    }
    // çalışma katmanı
    let mut kat: Vec<String> = hashes.to_vec();
    // köke kadar katla
    while kat.len() > 1 {
        let mut ust = Vec::with_capacity((kat.len() + 1) / 2);
        let mut i = 0;
        while i < kat.len() {
            // sağ yoksa solu yinele
            let sag = if i + 1 < kat.len() { &kat[i + 1] } else { &kat[i] };
            // iki özeti birleştirip özetle
            let mut h = Sha256::new();
            h.update(kat[i].as_bytes());
            h.update(sag.as_bytes());
            ust.push(hex_encode(h.finalize()));
            i += 2;
        }
        kat = ust;
    }
    kat.into_iter().next().unwrap()
}
fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    let b = bytes.as_ref();
    let mut s = String::with_capacity(b.len() * 2);
    for v in b {
        // alt/üst nibble
        s.push(char::from_digit((v >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((v & 0xf) as u32, 16).unwrap());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bos_hash_beklenen() {
        // boş girdinin bilinen özeti
        assert_eq!(
            sha256_frame(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
    #[test]
    fn doseme_merkle_tutarli() {
        // döşeme kareyle aynı formül
        assert_eq!(tile_hash(b"abc"), sha256_frame(b"abc"));
        // boş küme boş özet verir
        assert_eq!(merkle_root(&[]), sha256_frame(b""));
        // tek yaprak aynen döner
        let tek = vec![tile_hash(b"a")];
        assert_eq!(merkle_root(&tek), tek[0]);
        // iki yaprak çift özetinden uzun kök üretir
        let iki = vec![tile_hash(b"a"), tile_hash(b"b")];
        let kok = merkle_root(&iki);
        assert_eq!(kok.len(), 64);
        assert_ne!(kok, iki[0]);
    }
    #[test]
    fn manifesto_dogrulama_turu() {
        // sabit girdi
        let kare = b"rise-tur";
        // üret
        let m = create_manifest(kare, "test-cihazi", 123);
        // doğru kare geçer
        assert!(verify_manifest(&m, kare));
        // yanlış kare kalır
        assert!(!verify_manifest(&m, b"yanlis"));
        // bozuk JSON kalır
        assert!(!verify_manifest("bozuk", kare));
    }
}
