#!/bin/bash
# Tek kare yakala: Rapoo USB varsa onu, yoksa FaceTime HD'yi kullan.
# Çıktı: ../../target/frame-001.jpg (bindings/target/frame-001.jpg)
set -e
cd "$(dirname "$0")"

OUT="${1:-../../target/frame-001.jpg}"
mkdir -p "$(dirname "$OUT")"

# Cihaz önceliği: Rapoo (index 2) > FaceTime HD (index 1)
DEV="2"
if ! ffmpeg -f avfoundation -list_devices true -i "" 2>&1 | grep -q "Rapoo Camera"; then
  DEV="1"
fi
echo "Kamera index: $DEV"

if ffmpeg -f avfoundation -framerate 30 -video_size 1280x720 -i "$DEV" \
    -frames:v 1 -q:v 2 -y "$OUT" 2>capture.log; then
  :
else
  echo "UYARI: ilk deneme başarısız, varsayılan ayarla tekrar deneniyor..."
  ffmpeg -f avfoundation -i "$DEV" -frames:v 1 -q:v 2 -y "$OUT" 2>capture.log || {
    echo "HATA: kare alınamadı. capture.log'a bak."
    echo "İzin ipucu: Sistem Ayarları > Gizlilik ve Güvenlik > Kamera > Terminal'e izin ver."
    exit 1
  }
fi

if [ -s "$OUT" ]; then
  ls -lh "$OUT"
else
  echo "HATA: $OUT boş. İzin reddedilmiş olabilir:"
  echo "Sistem Ayarları > Gizlilik ve Güvenlik > Kamera > Terminal'e izin ver, tekrar çalıştır."
  exit 1
fi
