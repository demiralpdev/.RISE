# Çözüm Platformu Spec — 10 Vaka

## Kurallar (global)
- Paylaşım yalnız verify link; dosya/PDF gönderimi yok.
- Rozet dili sabit: Gold/Silver/Bronze + doğrulanmadı.
- Multi-TSA zorunlu; tek TSA imzası geçersiz.
- Key rotation 90 gün; eski key revoke listesinde.

## 1. Manifest strip
- UX/kural: Store kesintisinde rozet gösterilmez, link gerekir.
- Teknik: Manifest hash verify-web'de kontrol edilir.

## 2. Gold/Silver karışıklığı
- UX/kural: Rozet dili sabit, açıklama metni tek şablon.
- Teknik: Tier enum dışında string yasak.

## 3. Insider
- UX/kural: Tek kişi onaylayamaz, 2 imza şart.
- Teknik: RBAC + imza yetkisi ayrımı, log append-only.

## 4. Rüşvetli TSA
- UX/kural: Tek TSA'ya güvenilmez ibaresi.
- Teknik: Multi-TSA çapraz damga, uyumsuzlukta ret.

## 5. GPS mock
- UX/kural: Mock şüphesinde otomatik düşürme bildirimi.
- Teknik: Mock=auto downgrade; SafetyNet/Play Integrity kontrolü.

## 6. Tanık danışıklığı
- UX/kural: Aynı cihaz/IP tanıklar işaretlenir.
- Teknik: Witness independence score < eşikse düşür.

## 7. Mahkeme reddi
- UX/kural: "Delil değil, ön bulgu" ibaresi zorunlu.
- Teknik: eIDAS rapor çıktısı (PAdES/XAdES) üretimi.

## 8. Ekran görüntüsü paylaşımı
- UX/kural: Paylaş butonu yalnız verify link üretir.
- Teknik: Screenshot meta veri taşımaz, link imzalı.

## 9. Gizlilik kapalı
- UX/kural: Konum kapalıyken Gold verilmez uyarısı.
- Teknik: No-GPS gold tier yok; en fazla Silver.

## 10. Eski telefonlar
- UX/kural: Donanım yetmezse Silver önerilir.
- Teknik: Old phone=auto silver; API seviyesi kontrolü.
