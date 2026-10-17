# Scene Authenticity — Re-Capture Defense & Two-Axis Verification

> **Durum: TASARIM (kodda uygulanmadı).** `spec/` ile gerçek kod arasındaki farkın denetimi +
> teknik mimari + P0/P1/P2 yol haritası + mobil gerçeklik kontrolü.
> Kaynak: `spec/RISE-01.md` §5.1, `spec/threat-model.md` #2, `spec/solutions-physical.md`
> #1/#5/#7/#8/#10/#11, `spec/solutions-digital.md` #8, `PLAN.md` §7, `ROADMAP.md`.
>
> **Temel teşhis:** RISE şu an kriptografik olarak yalnızca **data origin & integrity**
> ("bu baytlar şu zaman damgasında, şu anahtarla imzalandı") kanıtı üretiyor;
> **scene authenticity** ("bu baytlar gerçek 3B dünyadan gelen fotonlarla oluştu") kanıtı üretmiyor.
>
> **Dil notu:** Bu doküman Türkçedir (analizin kaynak dili). `spec/` altındaki diğer dokümanlar
> İngilizcedir; talep halinde İngilizce sürüm eklenir.

---

## 0. Ön Tespit: `spec/` Dokümanlarındaki Savunmalar Kodda Var mı?

Kısa cevap: `capture_source: "screen"` beyanı dışındaki tüm fiziksel/re-capture savunmaları şu an
yalnızca tasarım dokümanında (`spec/`) yazılı; kodda uygulanmamış durumda. Ayrıca mevcut yakalama
kabuklarında güvenliği tersine çeviren kritik bir cihaz seçimi mantığı var:

| Spec Savunması | Kod Tabanındaki Gerçek Durum | Kanıt / Dosya Konumu |
|---|---|---|
| `threat-model` #2: Kare aralığı + damga sırası kontrolü → `suspicious` | **Yok (sadece spec).** `lib.rs` içindeki `VerifyDecision` yalnızca `Valid`, `Red`, `Unknown` içerir (`Suspicious` durumu yok). `ManifestV1` tek bir `timestamp_ms` tutar; kare başına zaman dizisi yoktur ve `main.rs` (`video_cmd`) ile `RiseCapture.kt` videodan sabit aralıklarla (1 fps / 200 ms) post-hoc kare çıkarır. | `lib.rs`, `verify.py` |
| `solutions-physical` #1 & #10: Nonce ışık challenge + moiré/yenileme artefaktı | **Yok (sadece spec).** Hiçbir kabukta flaş/ekran tetikleme veya `core/` içinde FFT/moiré frekans analizi kodu bulunmuyor. | `solutions-physical.md` |
| `solutions-physical` #5: Sahte USB kamera (USB = otomatik silver) ve sanal kamera reddi | **Tersine çalışıyor / eksik.** `capture.cpp`, `capture.swift`, `Capture.swift` ve `capture.sh` cihaz adında "USB", "UVC", "Rapoo" geçen harici kameraları dahili kameradan **önce** seçiyor. Windows `usb=1` bilgisini `.meta` yan dosyasına yazsa da bu bilgi imzalanan `ManifestV1` içine girmiyor. Sanal kamera (OBS, v4l2loopback) filtresi hiçbir OS'ta yok. | `capture.cpp`, `capture.swift`, `capture.sh` |
| OBS Binding Açığı: `rise-auto-seal.py` | **Aktif açık.** `rise-auto-seal.py` OBS kaydı bittiğinde `rise-core video <path> --device obs` çalıştırıyor; manifestte `capture_source` alanı boş (`None`) bırakıldığı için doğrulayıcı bu OBS kaydını kamera/dosya mührü sınıfında değerlendiriyor. | `rise-auto-seal.py` |
| `solutions-physical` #7, #8, #11: Şifreli MIPI, derinlik (L4), sensör seri no eşleşmesi | **Yok (sadece spec).** Donanım veya OS API'lerinden derinlik, sensör seri no veya MIPI durumu okuyan kod yok. | `solutions-physical.md` |
| `solutions-digital` #8: Screenshot laundering (artefakt + watermark) | **Kısmi (yalnızca beyan).** Görüntü içi sıkıştırma/watermark tespiti yok. Yalnızca `rise-core screen` kullanıldığında manifeste `capture_source: "screen"` yazılıyor ve `verify.py` uyarı gösteriyor. | `main.rs`, `verify.py` |
| Windows Gold-L2 Bağlaması: TPM + VBS (`attest.cpp`) | **Kopuk (self-asserted).** `REDTEAM_FINDINGS.md` (F7) içinde de not edildiği gibi `attest.exe` TPM'de P-256 anahtarı oluşturup `TIER=gold-L2` basıyor, ancak `rise-core sign` hâlâ diskteki yazılım PEM anahtarını (`--key`) kullanıyor; TPM imzası ve VBS quote manifeste bağlı değil. | `attest.cpp`, `REDTEAM_FINDINGS.md` |

---

## 1. Re-Capture Ailesinin Tehdit Taksonomisi

| # | Tehdit Vektörü | Saldırı Mekanizması ve Mevcut RISE'ta Neden Geçtiği | Tespit Doğası: Kanıtlanabilir mi / Yalnızca Sezgisel mi? |
|---|---|---|---|
| 1 | **Sanal Kamera** (OBS, ManyCam, v4l2loopback, DirectShow/CMIO) | Saldırgan AI görselini/videosunu kullanıcı uzayındaki sanal kamera sürücüsüne besler. `capture.cpp`, `capture.sh` veya `capture.swift` ilk bulduğu video kaynağını açıp YUY2/JPEG kareyi alır, `rise-core` ile hash'leyip imzalar. Kodda cihaz topolojisi / bus kontrolü olmadığı için meşru kamera çekimiyle birebir aynı manifest üretilir. | **İmzalı OS/uygulama ortamında kanıtlanabilir (deterministik):** Bootloader'ı kilitli ve App Attest / Play Integrity ile doğrulanan bir uygulamada OS kamera API'si sanal/harici kameraları deterministik olarak ayırt edip reddedebilir. Açık masaüstü OS'larda (Linux/Windows root/admin) ise kanıtlanamaz (kernel sürücüsü taklit edilebilir). |
| 2 | **Ekranı Yeniden Fotoğraflama / HDMI Yakalama** (Analog Hole & UVC capture card) | **2a (Optik):** Gerçek bir telefon/webcam yüksek PPI bir OLED ekrana doğrultulur. Fotonlar gerçek mercekten ve gerçek CMOS sensörden geçtiği için tüm donanım attestation'ları (StrongBox, SE, şifreli MIPI dahil) %100 geçer. **2b (HDMI→UVC):** HDMI capture card kendini standart USB UVC webcam olarak tanıtır, AI çıktısını ham YUY2 kare olarak basar. | **2a (optik ekran): Yalnızca sezgisel (olasılıksal).** Kriptografik hiçbir katman sensörün önündeki foton kaynağını kanıtlayamaz; yalnızca derinlik (paralaks), IMU ve optik artefaktlarla olasılıksal olarak yakalanır. **2b (HDMI UVC): Kanıtlanabilir (kural tabanlı).** Tüm harici USB/UVC cihazları donanım yolundan (`USB\VID_...`) tespit edilip Gold tier'dan deterministik olarak men edilir. |
| 3 | **Baskıyı Yeniden Fotoğraflama** (Print Re-capture) | AI görüntüsü yüksek kaliteli mat fotoğraf kağıdına basılır ve gerçek kamerayla fotoğraflanır. Ekran yenileme hızı (PWM/flicker), piksel ızgarası (moiré) ve ekran parlama artefaktları olmadığı için ekran tespit algoritmalarını tamamen atlatır. | **Yalnızca sezgisel (fiziksel tutarlılık).** Yalnızca 3B derinlik (LiDAR / çift kamera stereo disparity) veya kamera hareket ettirilirken oluşan 3B paralaks eksikliği (tüm noktaların tek bir 2B homografi düzleminde yatması) ile olasılıksal olarak tespit edilir. |
| 4 | **Daha Önce İmzalanmış Medyanın Replay'i** (Pre-signed / Stream Replay) | **4a (Dosya replay):** Geçmişte gerçekten çekilmiş ve imzalanmış bir medya, yeni bir olaymış gibi sunulur. **4b (Akış replay):** Daha önce kaydedilmiş gerçek bir kamera akışı (RAW/YUV), çekim anında sanal kameradan veya bellekten tekrar `rise-core`'a verilir. | **Kanıtlanabilir (kriptografik).** 4a: RFC 3161 QTSP zaman damgası çekim üst sınırını, imzalanan manifest içine gömülen taze verifier nonce'u (veya NIST randomness beacon / blok özeti) ise çekim alt sınırını kriptografik olarak kanıtlar. 4b: Canlı challenge-response (nonce) olmadan kanıtlanamaz; nonce varsa kanıtlanabilir. |
| 5 | **Sahte Attestation'lı Emülatör / VM** (QEMU, Android Emulator, swtpm) | Saldırgan emülatörde veya VM içinde sanal TPM (swtpm) / yazılım Keystore çalıştırır; sanal kameraya AI karesi verir. Mevcut `attest.cpp` yalnızca yerel olarak TPM'de anahtar açabildiğine baktığı (EK/AIK sertifika zincirini doğrulamadığı) için swtpm ile kolayca gold-L2 alır. | **Kanıtlanabilir (kriptografik — PKI sınırları dahilinde).** TPM Endorsement Key (EK) sertifikasının üretici (AMD/Intel/Infineon) köküne, Android Key Attestation zincirinin Google Hardware Root CA'ya (ve iptal edilen sızdırılmış keybox listesine), iOS App Attest'in Apple köküne sunucu tarafında doğrulanmasıyla kriptografik olarak kanıtlanır. |
| 6 | **OS Kamera Kaynağını Atlatan Root'lu Cihaz** (Frida / Zygisk / HAL Hook) | Gerçek bir telefonda donanım TEE/StrongBox anahtarı mevcuttur; ancak cihaz root'ludur (veya bootloader açıktır). Saldırgan `libcameraservice`, Camera2 `ImageReader` veya JNI `rise_hash_frame` fonksiyonunu Frida/LSPosed ile hook'lar; kameradan gelen YUV tamponunu imzadan hemen önce AI karesiyle değiştirir ve gerçek TEE anahtarıyla imzalatır. | **Kısmen kanıtlanabilir (bootloader/attestation) + kısmen sezgisel (runtime hooking).** Bootloader kilidi ve `verifiedBootState == VERIFIED` durumu donanım imzalı Key Attestation uzantısında kriptografik olarak kanıtlanır. Ancak kilitli bootloader üzerinde sıfırıncı gün (0-day) kernel/uygulama içi bellek enjeksiyonu varsa, sensör-içi imzalama (L4) olmadan bunu tespit etmek sezgiseldir. |

---

## 2. Savunma Derinliği (Defense-in-Depth) — (Etkinlik × Uygulanabilirlik) Sıralaması

Her katmanı **(Etkinlik / 10) × (Uygulanabilirlik / 10) = Kaldıraç Puanı** (100 üzerinden) ile sıraladık.

### Sıra 1: OS Kamera-Cihaz Kaynağı Doğrulaması ve Sanal/Loopback/Harici Cihaz Reddi

- **Puan:** Etkinlik 8/10 × Uygulanabilirlik 9.5/10 = **76 / 100**
- **Kanıt doğası:** Kilitli mobil OS'larda (iOS/Android) kanıtlanabilir (deterministik); masaüstü
  OS'larda (Windows/Linux) yüksek bariyerli sezgisel/kural tabanlı (kernel driver seviyesinde
  root/admin tarafından yanıltılabilir).
- **Platform bazlı somut uygulama:**
  - **iOS (`AVCaptureDevice.deviceType`):** `Capture.swift` içindeki `.external` cihaz tipini kaldır.
    Yalnızca `.builtInWideAngleCamera`, `.builtInUltraWideCamera`, `.builtInTelephotoCamera` kabul et;
    `.external` ve `.continuityCamera` kaynaklarını Gold/Silver kamera akışından tamamen reddet.
  - **Android (`Camera2 CameraCharacteristics`):** `RiseCapture.kt` içinde
    `LENS_FACING == LENS_FACING_EXTERNAL` olanları reddet (`LENS_FACING_BACK` veya
    `LENS_FACING_FRONT` zorunlu). Ek olarak `INFO_SUPPORTED_HARDWARE_LEVEL` değerinin `LEGACY`
    veya `EXTERNAL` olmadığını (`LIMITED`, `FULL` veya `LEVEL_3` olduğunu) ve
    `SENSOR_INFO_PHYSICAL_SIZE` ile `LENS_INFO_AVAILABLE_FOCAL_LENGTHS` değerlerinin fiziksel
    sınırlar içinde olduğunu doğrula.
  - **Windows (MediaFoundation + SetupAPI/CfgMgr32):** `capture.cpp` içindeki `FriendlyName`
    metin araması (`IsUsbSource`) tek başına güvensizdir çünkü OBS Virtual Camera adını
    değiştirebilir. `MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK` özniteliğini al:
    - Sembolik bağ `\\?\root#...` veya `\\?\swd#...` (Software Device) ile başlıyorsa sanal
      kameradır → doğrudan **RED**.
    - `CM_Get_DevNode_Property` ile `DEVPKEY_Device_EnumeratorName` ve ACPI `_PLD`
      (Physical Location of Device) sorgula: cihaz laptop kasasına dahili gömülü değilse
      (`usb=1` harici UVC ise) manifeste `"capture_source": "external_usb"` yaz ve rozeti
      maksimum **silver** ile sınırla.
  - **Linux (v4l2 + sysfs):** `/sys/class/video4linux/videoN/device` yolunu kontrol et.
    `v4l2loopback` ve `akvcam` gibi sanal kameralar `/sys/devices/virtual/video4linux/` altında
    yaşar ve fiziksel bir PCI/USB bus device symlink'ine sahip değildir → doğrudan **RED**.

### Sıra 2: Çekim Hattı Donanım Attestation'ı ve Anahtar Bağlama

- **Puan:** Etkinlik 9/10 × Uygulanabilirlik 8/10 = **72 / 100**
- **Kanıt doğası:** Kriptografik olarak kanıtlanabilir (üretici PKI köklerine kadar).
- **Neyi kanıtlar:**
  1. Özel anahtarın gerçek bir donanım çipinde (Android StrongBox/TEE KeyMint, iOS Secure
     Enclave, Windows fiziksel TPM 2.0) üretildiğini ve dışarı çıkarılamayacağını.
  2. Android Key Attestation X.509 uzantısı (`1.3.6.1.4.1.11129.2.1.17`) ve iOS App Attest
     üzerinden cihazın bootloader'ının kilitli (`VerifiedBootState == VERIFIED`), OS yama
     seviyesinin güncel ve çağrıyı yapan uygulamanın resmi imzalı RISE paketi
     (`applicationId` + imzalayan sertifika SHA-256 özeti) olduğunu.
- **Neyi kanıtlamaz:**
  1. Kameranın önündeki sahnenin gerçek 3B dünya olduğunu kanıtlamaz (ekran/baskı fotoğraflanabilir).
  2. Çekim anında kilitli bir cihazda bile uygulama kendi içinde yanlış API'yi çağırıyorsa veya
     bellek bozulması (memory corruption) varsa sensör-TEE arası ham piksel yolunu kanıtlamaz
     (çünkü hash TEE dışında, uygulama belleğinde alınıyor).
- **Mevcut kodda düzeltilmesi gereken kritik eksik:** Windows'ta `attest.cpp` sadece
  `TIER=gold-L2` yazdırıyor; imzayı gerçekten `NCryptSignHash` ile TPM içindeki anahtara
  attırmalı ve TPM EK/AIK sertifika zincirini + PCR/VBS quote'unu manifeste eklemelidir.
  Android'de `RiseKeystore.kt` `store.getCertificateChain(alias)` ile tam X.509 attestation
  zincirini manifeste koymalıdır (şu an sadece yaprak sertifikadan SPKI PEM alıyor).

### Sıra 3: Challenge-Response Liveness (Zamansal Nonce + Etkileşimli Hareket)

- **Puan:** Etkinlik 8.5/10 × Uygulanabilirlik 7.5/10 = **63.75 / 100**
- **Kanıt doğası:**
  - **Zaman penceresi (freshness / anti-replay): Kriptografik olarak kanıtlanabilir.** Uygulama
    çekim anında doğrulayıcı sunucudan (veya çevrimdışıysa son bilinen drand/NIST randomness
    beacon turundan) aldığı 32-baytlık `server_nonce` değerini hem KeyStore/AppAttest imzasına
    hem de manifeste dahil eder; çıktı RFC 3161 TSA ile damgalanır. Böylece medyanın
    `T_nonce` ile `T_tsa` arasındaki dar pencerede (ör. ≤10 saniye) üretildiği matematiksel
    olarak kanıtlanır.
  - **Sahne/poz challenge'ı (aktif liveness): Sezgisel / olasılıksal.** Yüksek riskli akışlarda
    (ör. kasko hasar, noter/KYC, saha denetimi) ekran üzerinde rastgele beliren bir hedefe
    doğru 1.5 saniyelik kamera eğme/kaydırma hareketi (pitch/yaw trajectory) istenir. Önceden
    kaydedilmiş bir videoyu veya sabit ekranı bu anlık yörüngeyle senkronize etmek gerçek
    zamanlı robotik/render düzeneği gerektirir.

### Sıra 4: Manifeste Bağlanan Çapraz Sensör Sinyalleri (IMU ↔ Optik Akış, Çift Lens Derinlik, ENF/Işık)

- **Puan:** Etkinlik 8/10 × Uygulanabilirlik 7/10 = **56 / 100**
- **Kanıt doğası:** Yalnızca sezgisel (fiziksel tutarlılık doğrulaması). Kriptografik kanıt
  değildir; ancak analog-hole (2B ekran ve baskı) saldırılarını yakalayan en güçlü katmandır:

| Çapraz Sinyal | Nasıl Çalışır? | Güvenirlik & Sınırlar |
|---|---|---|
| **IMU (Jiroskop/İvmeölçer) ↔ Kare Optik Akış Korelasyonu** | Deklanşör öncesi/sonrası 500 ms boyunca 100–200 Hz IMU verisi kaydedilir. Ardışık video/burst karelerinden hesaplanan global optik akış (homografi rotasyonu) ile jiroskop açısal hızı (`∫ ω dt`) arasındaki çapraz korelasyon (r > 0.85) ve faz gecikmesi (<15 ms) kontrol edilir. | **Yüksek.** Sanal kameraya dışarıdan verilen videoda telefonun el titremesiyle görüntü hareketi eşleşmez. Ekrana doğrultulmuş kamerada ise yakın planda perspektif kayması 2B düzlem homografisi ele verir. |
| **Çift Lens Disparity (Wide + Ultra-Wide) / LiDAR Derinlik** | Deklanşör anında ana ve ultra-geniş kameradan eşzamanlı iki kare (veya kısa burst ile hareket paralaksı) alınır. Hesaplanan derinlik haritası tek bir düzleme (`z = ax + by + c`, RANSAC inlier oranı > %92) oturuyorsa sahne düz bir ekran veya kağıt baskıdır. | **Yüksek.** 2B ekran ve kağıt baskı saldırılarını fiziksel geometriyle yakalar. Yalnızca gerçek 3B maket/heykel veya uzaktaki manzara çekimlerinde (sonsuz odak) ayırt ediciliği düşer. |
| **Ortam Işığı Titremesi ↔ Şebeke Frekansı (50/60 Hz ENF)** | Telefonun ortam ışığı sensörü (ALS) ve rolling-shutter satır parlaklık değişimi (100 Hz / 120 Hz aydınlatma titremesi), cihazın bulunduğu ülkenin elektrik şebekesi frekansı (ör. Türkiye/AB 50 Hz → 100 Hz flicker; ABD 60 Hz → 120 Hz) ile karşılaştırılır; ekran yenileme hızı (60/120/144 Hz) harmoniği aranır. | **Orta-Düşük.** Doğal gün ışığında veya DC LED aydınlatmada sinyal oluşmaz; yardımcı sinyal olarak kullanılmalıdır. |

### Sıra 5: Görüntü İçi İstatistiksel & Sensör Sinyalleri (Güvenilir vs. Zayıf Ayrımı)

- **Puan:** Etkinlik 4.5/10 × Uygulanabilirlik 6/10 = **27 / 100**
- **Kanıt doğası:** Tamamı yalnızca sezgisel (heuristic / istatistiksel). Tek başına hiçbir zaman
  "yeşil/gold" verme kriteri olamaz; yalnızca "şüpheli (downgrade)" bayrağı kaldırabilir:

| Sinyal | Güvenilirlik | Neden Güvenilir veya Zayıf? |
|---|---|---|
| **PRNU (Sensör Parmak İzi)** | Orta (yalnızca RAW/YUV'da) | Sensörün üretim kusurlarından kaynaklanan sabit desen gürültüsüdür. Ham Bayer/YUV karede ve aynı cihazdan alınmış referans karanlık/düz kare varsa güçlüdür. Ancak H.264/HEVC video sıkıştırması, telefonların çok kareli HDR/Gece Modu birleştirmesi ve çözünürlük ölçekleme PRNU korelasyonunu hızla bozar. |
| **Rolling-Shutter Satır Zamansallığı** | Orta (IMU ile birlikte) | CMOS sensör satırları yukarıdan aşağıya sırayla okunur (~10–15 ms okuma süresi). Hızlı kamera hareketinde dikey çizgilerin eğilmesi jiroskop verisiyle tam örtüşmelidir. Tek başına görüntüde ise yazılımla kolayca simüle edilebilir. |
| **Ekran Moiré & Yenileme Bandı** | Zayıf-Orta | Amatör ekran çekimlerinde FFT spektrumunda periyodik tepe noktaları üretir. Ancak yüksek PPI OLED bir ekranda kamerayı hafifçe odak dışı bırakmak (defocus), açıyı/mesafeyi ayarlamak veya mat ekran koruyucu kullanmak moiré'yi tamamen yok eder (`REDTEAM_FINDINGS.md` Round 4 A2 ile uyumlu). |
| **Lens Distorsiyonu & Specular/Işık Tutarlılığı** | Zayıf | Bir ekranı gerçek kamerayla fotoğrafladığında, gerçek kameranın kendi radyal lens distorsiyonu ve vinyeti zaten o kareye uygulanır! AI üreticileri de ışık/yansıma tutarlılığında insan ve klasik dedektör eşiklerini aşmış durumdadır. |
| **EXIF Makuliyeti** | Çok zayıf (çöp) | EXIF etiketleri (Make, Model, ExposureTime, ISO) tamamen istemci yazılımda oluşturulan düz metinlerdir; `exiftool` ile 1 milisaniyede gerçek bir iPhone 16 Pro profiliyle birebir değiştirilebilir. |

---

## 3. Asla Tamamen Çözülemeyecek Sınır (Analog Hole) ve Dürüst Rozet/UX Mimarisi

### Çözülemeyecek sınır

Hiçbir kriptografik protokol (C2PA, RISE veya donanım imzalı kameralar), optik sensör yüzeyine
çarpan fotonların gerçek bir fiziksel olaydan mı yoksa kalibre edilmiş bir projeksiyon/mikro-LED
sahnesinden mi geldiğini %100 kesinlikle kanıtlayamaz (**Analog Hole**). Sensör-içi imzalama
(Gold-L4) bile yalnızca "bu pikseller gerçekten bu CMOS sensörün ADC'sinden çıktı" der; sensörün
neye baktığını söylemez.

### Dürüst rozet ve UX tasarımı: iki eksenli doğrulama modeli

Tek bir Gold / Silver / Red rozeti, kullanıcıda "Gold = içerik gerçek, AI olamaz" yanılsaması
yaratır. Bunu önlemek için `ManifestV1` ve verify-web arayüzü doğrulamayı iki bağımsız eksene
ayırmalıdır:

**1. Eksen A — Kayıt Hattı Güvencesi (Pipeline Provenance — kriptografik kanıt):**
- **Gold-L2 (donanım onaylı kamera):** Kilitli cihaz + TEE/SE/StrongBox imzası + doğrulanmış
  dahili fiziksel kamera API'si + çoklu RFC 3161 TSA.
- **Silver (yazılım / harici kamera):** Yazılım anahtarı veya harici USB kamera.
- **Screen / Virtual (ekran veya sanal kayıt):** `capture_source: "screen" | "obs" | "virtual"`.
  Asla kamera rozeti almaz, mavi/gri ayrı bir "Ekran Kaydı" rozeti alır.

**2. Eksen B — Sahne Fizikselliği (Scene Liveness — olasılıksal / fiziksel tutarlılık):**
- **Liveness: 3B + hareket doğrulandı:** Çift kamera/LiDAR derinlik haritası düzlemsel değil
  VE IMU/optik akış korelasyonu geçti.
- **Liveness: doğrulanmadı (tek kare / düzlemsel):** Fiziksel kamerayla çekildi ama derinlik/
  hareket verisi yok veya sahne düz bir yüzeyden (ekran/baskı/duvar) ibaret.
- **Liveness: şüpheli (downgrade):** 2B ekran/moiré izi veya IMU-görüntü uyuşmazlığı tespit edildi.

### UX metin şablonları (verify-web ve CLI için)

- **Donanım onaylı + 3B sahne tutarlı:**
  > **Donanım Onaylı Kamera Çekimi (Gold-L2) · 3B Sahne Tutarlı**
  > Bu medya [Tarih/Saat] anında doğrulanmış bir dahili kamera sensöründen kaydedilmiş, donanım
  > güvenlik çipiyle imzalanmış ve sonrasında tek bir pikseli değiştirilmemiştir. Çoklu lens
  > derinlik ve hareket sensörü verileri 3 boyutlu fiziksel bir sahneyle uyumludur.

- **Donanım onaylı + liveness doğrulanmadı (kritik dürüstlük beyanı):**
  > **Donanım Onaylı Kamera Çekimi (Gold-L2) · Sahne Derinliği Doğrulanmadı**
  > Bu medya gerçek bir fiziksel kamerayla çekilmiş ve değiştirilmemiştir. Sınır: Çekim tek
  > kare/düzlemsel olduğu için kameranın gerçek bir olaya mı yoksa başka bir ekrana/baskıya mı
  > doğrultulduğu (analog hole) teknik olarak kanıtlanamaz.

- **OBS / ekran yakalama (`capture_source: "screen"` veya `"obs"`):**
  > **Zaman Damgalı Ekran/Yazılım Kaydı (Kamera Kanıtı Değildir)**
  > Bu dosya [Tarih/Saat] anında cihaz ekranında veya OBS sahnesinde görünen piksellerin
  > değiştirilmeden mühürlendiğini kanıtlar. Ekranda gösterilen içeriğin AI üretimi veya kurgu
  > olup olmadığını kanıtlamaz.

---

## 4. P0 / P1 / P2 Aşamalı Yol Haritası ve En Yüksek Kaldıraçlı İlk Adım

### En ucuz ve en yüksek kaldıraçlı İLK ADIM (1–2 gün, sıfır maliyet)

Sanal/OBS/harici kamera kapısını kapatmak ve `capture_source` alanını zorunlu hale getirmek:

1. `rise-auto-seal.py` içinde `rise-core video` komutuna `--capture-source obs` parametresi ekle
   (`main.rs` `Sign` ve `Video` alt komutlarına `--capture-source` bayrağı aç).
2. `capture.cpp`, `capture.swift`, `Capture.swift` ve `capture.sh` dosyalarında USB kamerayı
   dahili kameraya tercih eden ters mantığı kaldır; dahili fiziksel kamerayı birinci öncelik yap,
   sanal kameraları (`v4l2loopback`, Windows `SWD\`, OBS) tamamen reddet, USB kameralarda ise
   manifeste zorunlu `"capture_source": "external_usb"` yaz.
3. `verify.py` içinde `capture_source` değeri `"screen"`, `"obs"` veya `"external_usb"` olan hiçbir
   paketin kamera Gold adaylığına girmemesini ve açık kaynak uyarısı basmasını zorunlu kıl.

### Aşamalı yol haritası (P0 → P1 → P2)

| Faz | Kapsam ve Mühendislik Görevleri | Somut Kabul Kriterleri (Acceptance Criteria) |
|---|---|---|
| **P0 (1–2 hafta):** Sanal kamera reddi + kaynak sınıflandırma + Windows TPM imza bağlama | 1. Tüm OS kabuklarında sanal kamera (`v4l2loopback`, Windows `SWD\`/OBS, Android `EXTERNAL`) engellemesi ve dahili kamera önceliği. 2. `rise-core sign`/`video` için `--capture-source` parametresi ve `rise-auto-seal.py`'nin `"obs"` olarak etiketlenmesi. 3. Windows `attest.cpp` içindeki TPM P-256 anahtarının gerçekten manifesti `NCryptSignHash` ile imzalaması (F7 bulgusunun kapatılması). | - `v4l2loopback` veya OBS Virtual Camera aktifken capture araçları `ERR_VIRTUAL_CAMERA_REJECTED` (exit code != 0) ile çıkmalı. - OBS ile mühürlenen `.rvid` dosyası verify-web'e verildiğinde `capture_source: "obs"` ve ekran/yazılım uyarısı dönmeli (pytest testi). - Windows'ta TPM dışındaki bir PEM dosyasıyla üretilen hiçbir manifest gold-L2 alamamalı. |
| **P1 (3–6 hafta):** Mobil donanım attestation zinciri + sunucu nonce + IMU korelasyonu | 1. Android `RiseKeystore.kt` X.509 sertifika zinciri (`getCertificateChain`) + Play Integrity token doğrulamasının `verify-web/trust.py` içine bağlanması. 2. iOS `Attest.swift` Secure Enclave + `DCAppAttestService` assertion doğrulaması. 3. Çekim anında 500 ms IMU (jiroskop) penceresinin toplanıp video/burst optik akış korelasyon skoruyla (`imu_flow_corr`) manifeste gömülmesi. | - Bootloader'ı açık veya emülatör olan Android cihazda üretilen Key Attestation zinciri verify-web tarafından silver veya red'e düşürülmeli. - Sabit bir ekranda oynatılan videoyu elde tutulan telefonla çekerken el titremesi (IMU) ile video içi optik akış uyuşmazlığından `scene_liveness: "suspicious"` dönmeli. - Eski (replay) nonce ile yapılan imza denemeleri red almalı. |
| **P2 (2–3 ay):** Çift lens stereo derinlik (disparity) + iki eksenli rozet UX | 1. iOS (`AVCaptureMultiCamSession` / `AVCaptureDepthDataOutput`) ve Android (`LogicalMultiCamera` Wide+UltraWide) ile deklanşör anında eşzamanlı çift kare veya derinlik haritası yakalama. 2. Sahnenin 2B düzlem (ekran/kağıt baskı) olup olmadığını denetleyen RANSAC düzlemsellik testi (`planarity_score`). 3. verify-web arayüzünün Kayıt Hattı Güvencesi ve Sahne Fizikselliği olarak iki eksenli rozete geçirilmesi. | - Düz bir monitör veya kağıt baskı fotoğraflandığında `planarity_score > 0.92` nedeniyle `scene_liveness` alanı `"flat_surface_warning"` vermeli ve 3B Sahne Onayı alamamalı. - Gerçek 3B derinlikli sahnede (<3 metre) hem Gold-L2 hem `3D Scene Consistent` rozeti üretilmeli. |

---

## 5. Mobil Gerçeklik Kontrolü: Hangisi Pratikte Çalışır, Hangisi Kâğıt Üstünde Kalır?

Normal tüketici telefonlarında (standart iPhone ve Android cihazlar) `spec/solutions-physical.md`
içinde önerilen üç mekanizmanın gerçekçi mühendislik ve UX değerlendirmesi:

| Öneri | Mobil Gerçeklik Durumu | Neden? (UX, donanım ve maliyet analizi) |
|---|---|---|
| **1. Şifreli MIPI + sensör çıkışında hash (Gold-L4)** | **%100 kâğıt üstünde kalır** (üçüncü parti uygulama için imkânsız) | Standart iOS ve Android telefonlarda kamera sensörü ile SoC ISP arasındaki MIPI CSI-2 veri yolu donanımsal olarak kapalıdır ve işletim sistemi çekirdeği/ISP firmware'i tarafından yönetilir. Hiçbir App Store / Play Store uygulaması MIPI hattına erişemez, sensörün donanım seri numarasını kriptografik olarak doğrulayamaz veya ISP öncesi TEE içinde hash alamaz. Bu yalnızca Qualcomm/Apple'ın kendi kamera firmware'inde veya Leica/Sony/Nikon'un C2PA donanımlı özel kameralarında mümkündür. **Karar:** Gold-L4 tier'ını mobil uygulama kapsamından çıkarıp "ilerideki OEM/donanım üreticisi entegrasyonu" olarak işaretle. |
| **2. Nonce ışık challenge (flaş / ekran parlatma)** | **Genel çekimde kâğıt üstünde kalır; sadece <1.5 m kapalı alan nişinde çalışır** | **Fiziksel engel:** Gündüz dış mekanda (10.000–100.000 lüks) veya 2 metreden uzak nesnelerde telefonun LED flaşı ters-kare yasası (1/r²) yüzünden sahnede ölçülebilir bir yansıma farkı yaratamaz; gürültü tabanının altında kalır. **UX engeli:** Her fotoğraf/video çekiminde 300–600 ms boyunca rastgele flaş patlatmak doğal çekimi bozar, insanları rahatsız eder ve pil tüketir. **Karar:** Genel fotoğraf/video modunda zorunlu tutma (`solutions-physical.md` içindeki "her çekimde zorunlu" kuralını kaldır). Yalnızca "yakın mesafe belge / hasar / nesne doğrulama modu" (<1 metre, ör. sigorta araç hasarı veya sayaç okuma) için opsiyonel bir yüksek güvence adımı yap. |
| **3. Derinlik + IMU paralaks kontrolü** | **Pratikte uygulanır** (en yüksek kaldıraçlı mobil çözüm) | Özel LiDAR sensörü sadece Pro iPhone'larda, ToF ise çok az Android'de vardır; fakat neredeyse tüm modern akıllı telefonlarda ana + ultra-geniş çift kamera ve jiroskop (IMU) standarttır. Kullanıcı deklanşöre bastığında arka planda sıfır UX sürtünmesiyle (1) ana ve ultra-geniş kameradan eşzamanlı iki kare alıp stereo disparity (paralaks) hesaplamak ve (2) çekim anındaki 300 ms'lik doğal el mikro-hareketiyle optik akışı karşılaştırmak tamamen standart iOS (AVFoundation + CoreMotion) ve Android (Camera2 + SensorManager) API'leriyle bugün yazılabilir. |

---

## 6. P0 Uygulama Durumu (core yolu — TAMAM)

Yakalama sınıfı artık kaydediliyor, imza kapsamına giriyor ve doğrulayıcı tarafından
etiketleniyor. Yerelde doğrulandı:

| Değişiklik | Dosya | Kanıt |
|---|---|---|
| `sign` ve `video` üzerinde `--capture-source <camera\|screen\|obs\|virtual\|external_usb>`; bilinmeyen değer = sert hata (exit 2) | `core/src/main.rs` (`normalize_capture_source`, `SignArgs`, `VideoArgs`) | `rise-core sign --capture-source bogus` → rc 2; `--capture-source obs` → manifest `capture_source: "obs"` taşır |
| Değer damga/imzadan **önce** yazılıyor, dolayısıyla imza onu kapsıyor | `core/src/main.rs` (`sign_cmd`, `video_cmd`) | imzadan sonra `obs`→`camera` kurcalaması → `INVALID: invalid signature` (rc 3) |
| OBS kayıtları etiketleniyor | `bindings/obs/rise-auto-seal.py` | `rise-core video` çağrısına `--capture-source obs` eklendi |
| Kamera-dışı sınıflar doğrulayıcıda uyarı taşır | `verify-web/verify.py` (`_NON_CAMERA_SOURCES`, `_ok`) | verify-web `silver` rozeti + `capture_source: "obs"` + `capture_note` ("piksellerin gerçek bir kamera sensöründen geldiğini KANITLAMAZ") döner |
| Regresyon testleri | `verify-web/tests/test_badge.py` | `test_obs_capture_labeled_not_camera`, `test_virtual_and_external_usb_labeled` |
| **Sıra 1 kaynak kapısı** — her shell, sanal/loopback kaynağı kanıt hâline gelmeden önce reddeder | `bindings/{ios,macos}/capture/*.swift`, `bindings/android/capture/RiseCapture.kt`, `bindings/windows/capture/capture.cpp`, `bindings/linux/capture/cam_class.sh` | macOS: yazar makinesinde canlı numaralandırma kurulu OBS Virtual Camera'yı reddederken dahili lensi kabul etti (probe rc 0), `swiftc -typecheck` rc 0; iOS `swiftc -typecheck` rc 0; Android android-35'e karşı derleme-doğrulandı (kotlinc 2.3.20 + JRE 26.0.1, rc 0, tüm modül için 31 class — bu çalıştırma `FloatArray?` üzerindeki bir `isNullOrEmpty()` hatasını yakalayıp düzeltti) ve ardından çekim yolu gerçek bir telefonda **uçtan uca sürüldü** (fotoğraf + 14 karelik video mührü, ikisi de `capture_source:"camera"`, Mac'te yeniden doğrulandı); Linux `test_cam_class.sh` 6/6 (sanal → exit 10); **Windows düzenlemesi yalnızca statik** — bu makinede MSVC yok |
| Gold, donanım destekli anahtarı **manifestte değil, sabitlenmiş registry'de** beyan edilmiş olarak ister | `verify-web/verify.py` (`hardware_backing`, `_HARDWARE_BACKINGS`) | manifestin kendi `"hardware_backed": true` beyanı backing'i yükseltemez (yazılıma fail-closed); 82 pytest testi geçer |
| Windows arıza bilgileri dürüst kalır | `bindings/windows/attest/attest.cpp` | VBS "etkinleştirme talep edildi" olarak raporlanır ve hiçbir yüzey bunu "çalışıyor" diye okuyamaz (`running unverified`); TPM imza/quote üreticisi açıkça yoktur |

**P0 altında hâlâ açık:** Windows TPM `NCryptSignHash` bağlaması ile EK/AIK sertifika zinciri
ve PCR/VBS quote üretimi (F7 — `tpm_ecc_signature` / `tpm_cert_chain` / `tpm_quote` şu an
üreticisizdir, dolayısıyla hiçbir yüzey TPM'e bağlı bir imza iddia edemez). Sıra 1 OS kaynak
kapısı uygulandı: macOS/iOS typecheck'ten geçti (macOS ayrıca canlı numaralandırma ile
kanıtlandı); Android derleme-doğrulandı ve çekim yolu gerçek bir telefonda uçtan uca
sürüldü (fotoğraf + 14 karelik video mührü, ikisi de `capture_source:"camera"`; Mac
14/14 kare hash'ini, 56/56 tile'ı ve Merkle kökünü yeniden türetti, ES256 imzası
`video-001.pub` ile `Verified OK` döndü); ancak **Windows bacağı yalnızca statik** (yazar
makinesinde MSVC yok) ve **hiçbir bacak henüz düşmanca bir sanal kaynağa karşı sürülmedi**:
telefon LEGACY/EXTERNAL kamera sunmuyor, macOS yalnızca numaralandırma düzeyinde denendi
ve Linux'un `cam_class` testi sentetiktir.

---

## Referanslar

- `spec/RISE-01.md` §5.1 (capture_source sınıfları + analog-hole limiti)
- `spec/threat-model.md` #2 (screen re-capture), #1 (fake file), #4 (after-the-fact edit)
- `spec/solutions-physical.md` #1, #5, #7, #8, #10, #11
- `spec/solutions-digital.md` #8 (screenshot laundering)
- `spec/solutions-root.md` §1 (supply chain), §2 (OS), §5 (golden rules)
- `PLAN.md` §7 (Risks: "Screen re-capture", "Fake USB/MIPI source")
- `REDTEAM_FINDINGS.md` (F7 TPM binding; Round 4 A2 moiré)
- `bindings/obs/rise-auto-seal.py`, `core/src/main.rs` (`screen_cmd`, `video_cmd`),
  `core/src/lib.rs`, `verify-web/verify.py`, `verify-web/trust.py`
