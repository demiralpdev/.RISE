// Capture.swift — iOS single-frame capture (I-101).
//
// PLAN.md I-101: AVFoundation frame capture + raw byte output.
// Mirrors bindings/macos/capture/capture.swift structure:
// pickDevice priority, permission handling, manifest via core CLI.
//
// NOTE: NSCameraUsageDescription is REQUIRED in the host app's Info.plist,
// e.g. <key>NSCameraUsageDescription</key><string>...</string>.
// Without it the app crashes on first camera access (iOS enforced).
//
// Flow: capture frame -> write JPEG -> hand bytes to core CLI:
//   rise-core hash <jpeg>   # frame byte verification
//   rise-core sign ...      # signing happens in core, NEVER here.
// This file does NO signing, NO hashing for trust, NO timestamping.
//
// STATUS: STUB — NOT device-tested. Typechecked with `swiftc -typecheck`
// on macOS where possible; iOS-only lines are marked `// IOS-ONLY`.

import AVFoundation
import Foundation

/// Single-frame capture helper. Thin shell: frame bytes out, trust in core.
public enum RiseCapture {

    /// Device priority: USB/UVC external first, else back wide-angle default.
    /// Mirrors the macOS pickDevice() priority (Rapoo/UVC/USB first).
    public static func pickDevice() -> AVCaptureDevice? {
        let discovery = AVCaptureDevice.DiscoverySession(
            // Cross-platform subset so `swiftc -typecheck` passes on macOS.
            // On iOS this session also covers dual/triple rear cameras behind
            // .builtInWideAngleCamera; the back default below picks the rear.
            // IOS-ONLY note: .builtInDualCamera / .builtInTripleCamera exist
            // on iOS 10.2+/13+ but are API_UNAVAILABLE on macOS, so they are
            // deliberately NOT listed here.
            deviceTypes: [
                .builtInWideAngleCamera,
                .external
            ],
            mediaType: .video,
            position: .unspecified
        )
        let devices = discovery.devices
        if devices.isEmpty { return nil }

        // 1. USB / UVC external camera first (silver tier: external source).
        //    Name match mirrors macOS ("Rapoo"/"UVC"/"USB").
        if let usb = devices.first(where: {
            $0.localizedName.localizedCaseInsensitiveContains("UVC")
                || $0.localizedName.localizedCaseInsensitiveContains("USB")
                || $0.localizedName.localizedCaseInsensitiveContains("Rapoo")
        }) { return usb }

        // 2. Back wide-angle default (primary iPhone rear camera).
        if let back = AVCaptureDevice.default(
            .builtInWideAngleCamera, for: .video, position: .back
        ) { return back }

        // 3. Last resort: first available device.
        return devices.first
    }

    /// Camera permission state. Call before capture; presents system prompt
    /// on first use (NSCameraUsageDescription must be set, see header).
    public static func ensurePermission(timeoutSeconds: TimeInterval = 60) -> Bool {
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            return true
        case .notDetermined:
            let sem = DispatchSemaphore(value: 0)
            AVCaptureDevice.requestAccess(for: .video) { _ in sem.signal() }
            _ = sem.wait(timeout: .now() + timeoutSeconds)
            return AVCaptureDevice.authorizationStatus(for: .video) == .authorized
        default:
            return false
        }
    }

    /// Captures ONE still frame and writes it as JPEG to `outputPath`.
    /// - Returns: byte count written.
    /// - Note: session preset .hd1280x720 mirrors the macOS binding.
    @discardableResult
    public static func captureSingleFrame(to outputPath: String) throws -> Int {
        guard ensurePermission() else {
            throw RiseCaptureError.noPermission
        }
        guard let device = pickDevice() else {
            throw RiseCaptureError.noDevice
        }
        guard let input = try? AVCaptureDeviceInput(device: device) else {
            throw RiseCaptureError.openFailed(name: device.localizedName)
        }

        let session = AVCaptureSession()
        session.sessionPreset = .hd1280x720
        guard session.canAddInput(input) else {
            throw RiseCaptureError.openFailed(name: device.localizedName)
        }
        session.addInput(input)

        let photoOut = AVCapturePhotoOutput()
        guard session.canAddOutput(photoOut) else {
            throw RiseCaptureError.captureFailed
        }
        session.addOutput(photoOut)
        session.startRunning()
        defer { session.stopRunning() }

        // Short wait for exposure / white balance to settle (mirrors macOS).
        Thread.sleep(forTimeInterval: 1.0)

        let delegate = PhotoDelegate()
        photoOut.capturePhoto(with: AVCapturePhotoSettings(), delegate: delegate)
        _ = delegate.semaphore.wait(timeout: .now() + 15)

        guard let jpg = delegate.jpegData else {
            throw RiseCaptureError.captureFailed
        }
        let url = URL(fileURLWithPath: outputPath)
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try jpg.write(to: url)
        return jpg.count
    }
}

/// Errors are transport errors only — never trust verdicts.
public enum RiseCaptureError: Error {
    case noPermission
    case noDevice
    case openFailed(name: String)
    case captureFailed
}

private final class PhotoDelegate: NSObject, AVCapturePhotoCaptureDelegate {
    var jpegData: Data?
    let semaphore = DispatchSemaphore(value: 0)

    func photoOutput(
        _ output: AVCapturePhotoOutput,
        didFinishProcessingPhoto photo: AVCapturePhoto,
        error: Error?
    ) {
        jpegData = photo.fileDataRepresentation()
        semaphore.signal()
    }
}
