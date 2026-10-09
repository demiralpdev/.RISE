# Çözüm Kök — Kök Düzeltmeler Sentezi

> Kaynak: `RISE-01`, `cozum-dijital/fiziksel/platform`, `tehdit-modeli`, `guvence-seviyeleri`.
> Çekirdek: `SHA256(raw frame) at shutter + sign + timestamp`. İmzasız kare kanıt sayılmaz.

## 1. Tedarik (donanım kökü)
- Gold yalnızca dahili tasdikli sensörden; harici kaynak gold alamaz.
- Sensör seri eşleşmesi zorunlu; eşleşmezse red, değişimde anahtar iptal.
- L4 için şifreli MIPI zorunlu; şifresiz MIPI L4 veremez.
- USB kamera = otomatik silver; tasdiksiz kaynak red.
- Anahtar Gold'da TEE/SE içinde, dışarı çıkmaz; Silver'da kısa ömür + rotasyon.

## 2. OS (yol ve derleme)
- L4 hash sensör çıkışında alınır, OS'a girmeden; secure pipeline kullanılır.
- Reproducible builds zorunlu: aynı kaynak = aynı bit, SBOM yayınlanır.
- OS tasdiki + Play Integrity yalnız sinyal; server-side verify şart.
- Mock/GPS kapalılıkta otomatik düşürme (en fazla Silver).
- Eski cihaz = otomatik silver; API seviyesi kontrolü.

## 3. Kripto (imza ve zaman)
- İmza: ES256 varsayılan, Ed25519 seçenek; PQC-ready: ML-DSA hazırlığı.
- Dual-sign path: klasik + PQC çift imza yolu açık, tek algoya kilitlenme yok.
- Zaman: RFC 3161 QTSP zorunlu, multi-TSA çapraz damga; tek TSA geçersiz.
- Rekor v2 zaman kanıtı değildir; yalnız dahililik + witness eşiği.
- Trust List + OCSP hard-fail; ulaşılamazsa `bilinmiyor`, iptalse `güvensiz`.
- LTV: uzun dönem doğrulama için zincir + damga + iptal kanıtı saklanır.
- Rotasyon ≤ 90 gün; eski anahtar revoke listesinde.

## 4. Yönetişim (güven dağıtımı)
- No single trust point: hiçbir kişi, TSA, CA veya sunucu tek başına güven vermez.
- Multisig: kritik onayda 2 imza şart; tek kişi onaylayamaz (RBAC + yetki ayrımı).
- Log append-only; insider işlemi çift imzalı ve denetlenebilir.
- Witness independence score < eşikse düşür; aynı cihaz/IP tanık işaretlenir.
- Paylaşım yalnız verify link; strip = untrusted, screenshot = kırmızı.
- Mahkeme dili: "delil değil, ön bulgu"; eIDAS raporu (PAdES/XAdES) üretilir.

## 5 Altın Kural
1. İmzasız kare kanıt değildir: hash + imza + damga üçlüsü eksikse red.
2. Tek noktaya güvenme: multi-TSA + Trust List + OCSP birlikte aranır.
3. Yolu kilitle: L4 dahili şifreli MIPI + depth + nonce light challenge.
4. Çift imzayla yönet: kritik kararda multisig, dual-sign path ile geleceğe hazır.
5. Fail-closed kal: şüphede düşür, listeye erişilemezse asla yeşil verme; LTV sakla.
