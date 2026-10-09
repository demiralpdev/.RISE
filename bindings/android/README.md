# Android Binding (ince kabuk)

Sorumluluk: kareyi yakala, `core/`'a gönder. İmza mantığı burada yok.

- Donanım deposu: StrongBox + Play Integrity
- Görev: kamera karesi → core hash/attest çağrısı
- Gerçek kod yok, sadece iskelet.

## Gold-gate checklist

- StrongBox anahtar + Play Integrity STRONG verdict yoksa gold verme.
- STRONG yoksa gold-L4 yerine gold-L2 veya silver'a düşür.
- BASIC verdict + sağlam zincir = en fazla gold-L2.
- Zincir kırık/replay şüphesi = red, binding itiraz etmez.
- Şüphede rozeti yükseltme, düşür.
