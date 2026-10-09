# verify-web (doğrulayıcı iskelet)

Yüklenen kanıtı gösterir: altın / gümüş / kırmızı rozet.

## Rozet karar ağacı

- gold-L4: donanım mühür + STRONG verdict + zincir sağlam → tam yasal ağırlık.
- gold-L2: donanım mühür + BASIC verdict + zincir sağlam → sınırlı ağırlık.
- silver: yazılım anahtar veya zincirde küçük eksik → bilgi amaçlı.
- red: imza geçersiz, zincir kırık veya replay → reddet.
- Kural: STRONG yoksa gold-L4 verme, bir basamak düşür.
- Kural: zincir kırık/replay varsa direkt red ver.
- Kural: şüphede rozeti yükseltme, düşür.
- Rozetler hukuki ağırlığı belirler: gold-L4 > gold-L2 > silver > red.
- Ekranda her rozetin gerekçesi tek satır gösterilir.
- Gerçek doğrulama yok, yer tutucu; sonraki adım core çıktısını rozete bağla.
