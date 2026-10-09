# macOS Capture (minimal)

Tek kare yakalar → `../../target/frame-001.jpg` (yani `bindings/target/`).
Öncelik: **Rapoo USB** varsa o, yoksa FaceTime HD.

## Çalıştır

```bash
cd bindings/macos/capture
./capture.sh
```

Alternatif (saf Swift, ffmpeg'siz):

```bash
swiftc capture.swift -o capture-swift
./capture-swift
```

## İzin

İlk çalıştırmada macOS kamera izni ister. Reddedilirse:

**Sistem Ayarları → Gizlilik ve Güvenlik → Kamera → Terminal'e izin ver**, tekrar çalıştır.

## Dosyalar

- `capture.sh` — ffmpeg ile 1 kare (önerilen)
- `capture.swift` — AVCapture ile 1 kare (ffmpeg yoksa)
- `capture.log` — son çalışmanın ffmpeg çıktısı (hata ayıklama için)
