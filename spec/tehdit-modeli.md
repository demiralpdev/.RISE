# Tehdit Modeli

> 4 saldırı + her birine karşı önlem. Basit tutuldu.

## 1. Sahte Dosya

- Saldırı: Baştan uydurma video dosya olarak yüklenir.
- Neden tutmaz: `SHA256(raw frame) at shutter + sign + timestamp` yok.
- Önlem: İmza + RFC 3161 damgası zorunlu. İmzasız dosya `güvensiz` sayılır.

## 2. Ekranı Yeniden Çekme

- Saldırı: Başka ekran oynatılır, kamerayla yeniden kaydedilir.
- Neden zor: Gold-L4 sensör yol damgası ve kare zamanları tutarsız olur.
- Önlem: Kare aralıklarını ve damga sırasını kontrol et. Uyumsuzsa `şüpheli` yaz.

## 3. Anahtar Çalınması

- Saldırı: Cihaz anahtarı kopyalanır, sahte içerik imzalanır.
- Önlem Silver: Anahtarı yenile, eskisini iptal listesine ekle.
- Önlem Gold: TEE/SE anahtarı dışarı vermez. C2PA Trust List + revocation kontrolü zorunlu.

## 4. Sonradan Düzenleme (Edit)

- Saldırı: Kayıttan sonra kare kesilir, eklenir, filtre uygulanır.
- Neden belli olur: Kare SHA-256 listesi değişir, imza tutmaz.
- Önlem: Her kare hash'i manifesttedir (C2PA 2.4 + JUMBF). Tek kare değişse doğrulama başarısız olur.

## Genel Kurallar

- C2PA Trust List'e ulaşılamazsa karar `bilinmiyor` olur.
- İptal edilmiş sertifika = `güvensiz`, istisnası yok.
- Referanslar: C2PA 2.4, ISO 21617-1:2026, RFC 3161.
