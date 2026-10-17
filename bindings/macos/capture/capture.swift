// capture.swift — captures a single frame via AVCapture, writes a JPEG.
// Build: swiftc capture.swift -o capture-swift
// Run: ./capture-swift [output-path]
// Priority: built-in camera only; a virtual/external source is rejected.
import AVFoundation
import AppKit

let outPath = CommandLine.arguments.count > 1
    ? CommandLine.arguments[1]
    : "../../target/frame-001.jpg"

// SECURITY INVARIANT: only a built-in lens may be captured. A virtual camera
// (OBS Virtual Camera, a CoreMediaIO DAL plugin) is reported as an EXTERNAL
// device, and it can also be set as the system default video device, so every
// non-built-in type fails closed here.
func isAcceptableCameraSource(_ device: AVCaptureDevice) -> Bool {
    switch device.deviceType {
    case .builtInWideAngleCamera:
        return true
    default:
        return false
    }
}

func pickDevice() -> AVCaptureDevice? {
    let session = AVCaptureDevice.DiscoverySession(
        deviceTypes: [.builtInWideAngleCamera],
        mediaType: .video, position: .unspecified)
    // 1. The system default video device, if it passes the source gate.
    if let def = AVCaptureDevice.default(for: .video),
       isAcceptableCameraSource(def) { return def }
    // 2. First discovered device that passes the source gate.
    return session.devices.first(where: isAcceptableCameraSource)
}

switch AVCaptureDevice.authorizationStatus(for: .video) {
case .notDetermined:
    // synchronous wait: the system prompts on first run
    let sem = DispatchSemaphore(value: 0)
    AVCaptureDevice.requestAccess(for: .video) { _ in sem.signal() }
    _ = sem.wait(timeout: .now() + 60)
default: break
}

guard AVCaptureDevice.authorizationStatus(for: .video) == .authorized else {
    fputs("ERROR: no camera permission. System Settings > Privacy & Security > Camera.\n", stderr)
    exit(1)
}

guard let device = pickDevice() else {
    fputs("ERROR: no camera found.\n", stderr)
    exit(1)
}
print("Camera: \(device.localizedName)")

guard let input = try? AVCaptureDeviceInput(device: device) else {
    fputs("ERROR: camera could not be opened (another app may be using it).\n", stderr)
    exit(1)
}

let session = AVCaptureSession()
session.sessionPreset = .hd1280x720
session.addInput(input)

let photoOut = AVCapturePhotoOutput()
session.addOutput(photoOut)
session.startRunning()
defer { session.stopRunning() }

// short wait for exposure to settle
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
    fputs("ERROR: could not capture a frame.\n", stderr)
    exit(1)
}
let url = URL(fileURLWithPath: outPath)
try? FileManager.default.createDirectory(
    at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
do {
    try jpg.write(to: url)
    print("Written: \(outPath) (\(jpg.count) bytes)")
} catch {
    fputs("ERROR: could not write: \(error)\n", stderr)
    exit(1)
}
