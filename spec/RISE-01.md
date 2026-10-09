# RISE-01 — Çekirdek Kayıt Biçimi

> Amaç: Deklanşör anında kanıt üret. Sonradan ekleme yok.

## 1. Çekirdek Formül

```
SHA256(raw frame) at shutter + sign + timestamp
```

- `raw frame`: Sensörden gelen işlenmemiş kare.
- `shutter`: Deklanşör anı, gecikme yok.
- `sign`: Cihaz anahtarıyla imza.
- `timestamp`: RFC 3161 damgası.

## 2. Kutu (Box) Yapısı

```
[ftyp] [mvex-uzantı] [jumb] [mdat]
```

- `ftyp`: Dosya tipi, ISO 21617-1:2026 uyumlu.
- `mvex-uzantı`: Kare hash listesi (her kareye bir SHA-256).
- `jumb`: JUMBF gömülü C2PA 2.4 manifesti.
- `mdat`: Ham medya baytları.

## 3. Hash ve İmza

- Hash: SHA-256 (zorunlu, tek algoritma).
- İmza varsayılan: ES256 (P-256 + ECDSA).
- İmza seçeneği: Ed25519 (ince cihazlar için).
- Her kare hash'i imzalı manifestin içindedir.

## 4. C2PA 2.4 Uyumu

- Manifest C2PA 2.4 şemasına uyar.
- JUMBF kutusu standart okuyucuda açılır.
- RISE kutuları C2PA doğrulayıcıyı bozmaz.

## 5. Manifest Alanları

| Alan | Açıklama |
|---|---|
| `rise_version` | Biçim sürümü (`1`) |
| `frame_hashes` | Kare SHA-256 listesi |
| `sig_alg` | `ES256` veya `Ed25519` |
| `timestamp_token` | RFC 3161 jetonu |
| `device_id` | Anonim cihaz kimliği |

## 6. Sürümlendirme

- `rise_version: 1` ile başlar.
- Yeni alan eklenebilir, eski alan silinemez.
- Okuyucu bilmediği alanı görmezden gelir.

## 7. Güven Zorunlulukları

- C2PA Trust List kontrolü zorunlu.
- Sertifika iptal (revocation) kontrolü zorunlu.
- Listeye ulaşılamazsa sonuç: `bilinmiyor`, asla `güvenli` değil.

## Referanslar

- C2PA 2.4, ISO 21617-1:2026, RFC 3161.
