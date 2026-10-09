# macOS Binding (ince kabuk)

Sorumluluk: kareyi yakala, `core/`'a gönder. İmza mantığı burada yok.

- Donanım deposu: Secure Enclave
- Görev: kamera karesi → core hash/attest çağrısı
- Gerçek kod yok, sadece iskelet.

## Gold-gate checklist

- Secure Enclave anahtar yoksa gold verme.
- Platform attest zayıfsa gold-L4 yerine gold-L2 veya silver'a düşür.
- Zayıf attest + sağlam zincir = en fazla gold-L2.
- Zincir kırık/replay şüphesi = red, binding itiraz etmez.
- Şüphede rozeti yükseltme, düşür.
