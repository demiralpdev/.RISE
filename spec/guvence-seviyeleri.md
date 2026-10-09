# Güvence Seviyeleri

> Üç seviye: silver, gold-L2, gold-L4. Her üst seviye altını kapsar.

## Silver (Yazılım)

- Anahtar cihazın normal alanında tutulur.
- İmza: ES256 varsayılan, Ed25519 seçenek.
- Formül aynı: `SHA256(raw frame) at shutter + sign + timestamp`.
- RFC 3161 damgası zorunlu.
- Kullanım: Günlük çekim, düşük risk.

## Gold-L2 (Korunaklı Donanım)

- Anahtar TEE / Secure Element içindedir, dışarı çıkmaz.
- İmza donanım içinde atılır.
- C2PA Trust List + revocation kontrolü zorunlu.
- Kullanım: Basın, sigorta, kurumsal kanıt.

## Gold-L4 (Sensör + Korunaklı Yol)

- Hash sensör çıkışında, işletim sistemine girmeden alınır.
- Korunaklı medya yolu (secure pipeline) kullanılır.
- C2PA 2.4 manifesti + JUMBF gömülüdür.
- ISO 21617-1:2026 kutu yapısına uyar.
- Kullanım: Mahkeme, kritik altyapı, yüksek risk.

## Özet Tablo

| Özellik | Silver | Gold-L2 | Gold-L4 |
|---|---|---|---|
| Anahtar yeri | Yazılım | TEE/SE | TEE/SE |
| Hash noktası | Uygulama | Uygulama | Sensör çıkışı |
| Damga (RFC 3161) | Var | Var | Var |
| Trust List + iptal | Var | Var | Var |

## Kural

- Seviye manifestin `guvence` alanına yazılır.
- Doğrulayıcı seviyeyi gösterir, yükseltmez.
