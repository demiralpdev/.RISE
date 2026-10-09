# Linux Binding (ince kabuk)

Sorumluluk: kareyi yakala, `core/`'a gönder. İmza mantığı burada yok.

- Donanım deposu: TPM2 opsiyonel
- Görev: kamera karesi → core hash/attest çağrısı
- Gerçek kod yok, sadece iskelet.

## Gold-gate checklist

- TPM2 anahtar yoksa gold verme, en fazla silver hedefle.
- Donanım attest yoksa gold-L4/L2 kapalı, silver'a düşür.
- Yazılım anahtar + sağlam zincir = en fazla silver.
- Zincir kırık/replay şüphesi = red, binding itiraz etmez.
- Şüphede rozeti yükseltme, düşür.
