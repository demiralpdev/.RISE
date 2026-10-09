// capture.swift — AVCapture ile tek kare yakalar, JPEG yazar.
// Derleme: swiftc capture.swift -o capture-swift
// Çalıştırma: ./capture-swift [çıktı-yolu]
// Öncelik: adında "Rapoo"/"UVC" geçen cihaz, yoksa varsayılan video cihazı.
import AVFoundation
import AppKit

let outPath = CommandLine.arguments.count > 1
    ? CommandLine.arguments[1]
    : "../../target/frame-001.jpg"

func pickDevice() -> AVCaptureDevice? {
    let session = AVCaptureDevice.DiscoverySession(
        deviceTypes: [.builtInWideAngleCamera, .external],
        mediaType: .video, position: .unspecified)
    let devs = session.devices
    if devs.isEmpty { return nil }
    // Rapoo / USB UVC öncelikli (silver tier: harici kamera)
    if let usb = devs.first(where: {
        $0.localizedName.localizedCaseInsensitiveContains("Rapoo")
        || $0.localizedName.localizedCaseInsensitiveContains("UVC")
        || $0.localizedName.localizedCaseInsensitiveContains("USB")
    }) { return usb }
    // yoksa FaceTime HD, en son ilk bulunan
    return devs.first(where: {
        $0.localizedName.localizedCaseInsensitiveContains("FaceTime")
    }) ?? devs.first
}

switch AVCaptureDevice.authorizationStatus(for: .video) {
case .notDetermined:
    // Senkron bekle: ilk çalıştırmada sistem izni sorar.
    let sem = DispatchSemaphore(value: 0)
    AVCaptureDevice.requestAccess(for: .video) { _ in sem.signal() }
    _ = sem.wait(timeout: .now() + 60)
default: break
}

guard AVCaptureDevice.authorizationStatus(for: .video) == .authorized else {
    fputs("HATA: kamera izni yok. Sistem Ayarları > Gizlilik ve Güvenlik > Kamera'dan izin ver.\n", stderr)
    exit(1)
}

guard let device = pickDevice() else {
    fputs("HATA: kamera bulunamadı.\n", stderr)
    exit(1)
}
print("Kamera: \(device.localizedName)")

guard let input = try? AVCaptureDeviceInput(device: device) else {
    fputs("HATA: kamera açılamadı (başka uygulama kullanıyor olabilir).\n", stderr)
    exit(1)
}

let session = AVCaptureSession()
session.sessionPreset = .hd1280x720
session.addInput(input)

let photoOut = AVCapturePhotoOutput()
session.addOutput(photoOut)
session.startRunning()
defer { session.stopRunning() }

// Pozlama otursun diye kısa bekle
Thread.sleep(forTimeInterval: 1.0)

class Delegate: NSObject, AVCapturePhotoCaptureDelegate {
    var data: Data?
    let sem = DispatchSemaphore(value: 0)
    func photoOutput(_ o: AVCapturePhotoOutput,
                     didFinishProcessingPhoto p: AVCapturePhoto,
                     error: Error?) {
        data = p.fileDataRepresentation()
        sem.signal()
    }
}
let del = Delegate()
photoOut.capturePhoto(with: AVCapturePhotoSettings(), delegate: del)
_ = del.sem.wait(timeout: .now() + 15)

guard let jpg = del.data else {
    fputs("HATA: kare alınamadı.\n", stderr)
    exit(1)
}
let url = URL(fileURLWithPath: outPath)
try? FileManager.default.createDirectory(
    at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
do {
    try jpg.write(to: url)
    print("Yazıldı: \(outPath) (\(jpg.count) bayt)")
} catch {
    fputs("HATA: yazılamadı: \(error)\n", stderr)
    exit(1)
}
