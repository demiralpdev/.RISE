# Fiziksel Saldırı Çözümleri

> Çekirdek: `SHA256(raw frame) + sign + timestamp`. İmzasız kare kanıt sayılmaz.

## Zorunlu Kurallar

- Gold yalnızca dahili tasdikli sensörden verilir; harici kaynak gold alamaz.
- USB kamera = otomatik silver, üstü yasak.
- L4 için şifreli MIPI zorunlu; şifresiz MIPI L4 veremez.
- Nonce ışık meydan okuması (light challenge) zorunlu.
- L4 için derinlik (depth) zorunlu; derinliksiz L4 yok.
- Sensör seri eşleşmesi zorunlu; eşleşmeyen sensör red.

## 1. Ekran Yeniden Çekme

- Kural: Kare aralığı + moiré/yenileme izi kontrol edilir.
- Teknik: Nonce ışık meydan okumasına sahne yanıtı aranır; ekran yanıtı tutmaz.
- Rozet: Başarısızsa red; iz yoksa silver; L2/L4 sensör yolu temizse gold.

## 2. Basılı Fotoğraf

- Kural: Nonce ışık yanıtı + mikro hareket (paralaks) istenir.
- Teknik: Flaş/nonce yansıma değişimi ölçülür; baskıda değişim yoktur.
- Rozet: Yanıt yoksa red; tek gözle silver; L2/L4 + depth ile gold.

## 3. 3B Maske

- Kural: Derinlik + cilt yansıması (IR/doku) birlikte aranır.
- Teknik: Depth haritası canlılıkla çaprazlanır; maske dokusu elenir.
- Rozet: Depth yoksa red; silver üst sınır; L4 depth + IR ile gold.

## 4. Projeksiyon / Hologram

- Kural: Projeksiyon titremesi + depth tutarlılığı kontrol edilir.
- Teknik: Nonce ışık sırası sahnede doğrulanır; yansıtılan ışık gecikir/bozulur.
- Rozet: Uyumsuzsa red; en fazla silver; L4 yalnızca gerçek depth ile gold.

## 5. Sahte USB Kamera

- Kural: Kaynak tanımlama zorunlu; USB VID/PID + tasdik istenir.
- Teknik: Dahili sensör tasdiki yoksa gold yolu kapatılır.
- Rozet: USB her durumda otomatik silver; gold verilmez; tasdiksizse red.

## 6. Lens Üstü Yerleşim (Overlay)

- Kural: Çekim öncesi/sonrası nonce kare karşılaştırması yapılır.
- Teknik: Bilinen ışık desenine yanıt + kenar/optik tutarlılık bakılır.
- Rozet: Sapma varsa red; temizse silver; L2/L4 iç sensörle gold.

## 7. HDMI / MIPI Enjektör

- Kural: Şifreli MIPI + sensör imzası aranır; harici enjeksiyon engellenir.
- Teknik: Sensör çıkışı şifreli kanaldan hashlenir (`SHA256(raw frame) at shutter`).
- Rozet: Şifresiz/imasız kaynak red; şifreli ama harici ise silver; L4 yalnızca dahili şifreli MIPI.

## 8. Derinlik Sahteciliği

- Kural: Derinlik + RGB zaman damgası eşleşmek zorunda.
- Teknik: Depth/RGB senkronu ve donanım imzası doğrulanır; enjekte depth elenir.
- Rozet: Eşleşmezse red; depth yoksa silver üst sınır değil red'e yakın; L4 çift imzalı depth ile gold.

## 9. IR Tekrar Oynatma

- Kural: Her çekimde taze nonce IR deseni kullanılır, tekrar kabul edilmez.
- Teknik: Nonce tek kullanımlık sayılır; kayıtlı IR yanıtı zaman aşımına düşer.
- Rozet: Tekrar tespitinde red; taze nonce + silver; L2/L4 tasdikli IR ile gold.

## 10. Işık Meydan Okumasını Atlatma

- Kural: Nonce light challenge atlanamaz; yanıtsız kayıt yükselmez.
- Teknik: Rastgele renk/süre ışık patlamasına sahne yansıması ölçülür.
- Rozet: Yanıt yok/geçersizse red; geçerliyse silver; L2/L4 + sensör tasdikiyle gold.

## 11. Sensör Değişimi

- Kural: Sensör seri no cihaz anahtarıyla eşleşmek zorunda.
- Teknik: Açılışta tasdikli seri kontrolü; değişimde anahtar iptal + yeniden eşleşme gerekir.
- Rozet: Eşleşmezse red; eşleşirse silver; L2/L4 yalnızca fabrikasyon eşleşmeyle gold.
