# Çözüm: Dijital Saldırılar (11)

Stack: C2PA 2.4 JUMBF + ES256 + Rekor v2. İlke: server-side verify zorunlu.
Trust List 2026-01-01'de dondurulur + OCSP zorunlu. Zaman kanıtı yalnız RFC3161 QTSP; Rekor zaman kanıtı değildir.

## 1. Self-signed sertifika
- Kural: Yalnız Trust List'teki CA kabul edilir; self-signed reddedilir.
- Teknik: Zincir doğrulama + OCSP (hard-fail) + Trust List pin.
- Residual: CA uzlaşması; şeffaflık logu ile tespit edilir.

## 2. Sertifika hırsızlığı
- Kural: Kısa ömürlü sertifika + rotasyon zorunlu.
- Teknik: Süre ≤ 90 gün, otomatik rotasyon, HSM/sunucuda anahtar, sızıntıda iptal.
- Residual: Çalınma–iptal penceresi; OCSP ile kısaltılır.

## 3. İptal edilmiş sertifika kullanımı
- Kural: OCSP zorunlu, soft-fail yasak.
- Teknik: Her doğrulamada OCSP/CRL sorgusu; yanıtsız = untrusted.
- Residual: OCSP responder kesintisinde erişim durur (fail-closed).

## 4. Zaman damgası sahteciliği
- Kural: RFC3161 QTSP zorunlu; Rekor zamanı kanıt sayılmaz.
- Teknik: QTSP token JUMBF'a gömülür, zincir + nonce doğrulanır.
- Residual: QTSP uzlaşması; çoklu QTSP çapraz kontrolle azaltılır.

## 5. Rekor fork / log bölünmesi
- Kural: Rekor v2 dahililik kanıtı + witness eşiği aranır.
- Teknik: Signed checkpoint, tutarlılık kanıtı, çoklu witness.
- Residual: Uzun süreli split-view; izleme ile tespit edilir.

## 6. Tile swap (parça değiştirme)
- Kural: Tile Merkle root ES256 ile imzalanır.
- Teknik: Her tile hash'i ağaca bağlanır, root manifestte imzalı, sıra değişimi reddedilir.
- Residual: Kaynak tile zaten bozuksa tespit edilemez.

## 7. Metadata strip (JUMBF sökme)
- Kural: Strip = untrusted.
- Teknik: Manifest yoksa/bağ kopuksa "doğrulanamaz" etiketi, yeşil verilmez.
- Residual: Meşru kırpma da cezalandırılır (kabul edilen bedel).

## 8. Ekran görüntüsü aklama
- Kural: Screenshot = kırmızı + filigran tespiti.
- Teknik: Ekran/sıkıştırma izi + filigran yokluğu → kaynak kanıtı sayılmaz.
- Residual: Yüksek kalite yeniden çekimde iz zayıflar.

## 9. AI inpainting / düzenleme
- Kural: Her piksel değişimi assertion ile beyan edilir.
- Teknik: C2PA action (createdBy/editing) + pad hash karşılaştırma; beyan yoksa kırmızı.
- Residual: Anlamsal ama piksel-tutarlı sahtecilikte insan incelemesi gerekir.

## 10. Anahtar çıkarımı (key extraction)
- Kural: Kısa ömür + rotasyon + sunucu tarafı imza.
- Teknik: İmza cihazda değil sunucu/HSM'de, anahtar asla istemcide durmaz.
- Residual: Sunucu uzlaşması; izleme + rotasyonla sınırlanır.

## 11. Play Integrity spoof
- Kural: Tek başına güvenilmez; sunucu doğrulama şart.
- Teknik: Play Integrity + backend nonce + cihaz bağlama; yalnız sinyal yeşil vermez.
- Residual: Root/emülatör atlatmaları; katmanlı kontrolle azaltılır.
