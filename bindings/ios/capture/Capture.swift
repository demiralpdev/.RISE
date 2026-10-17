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

    /// Rank 1 source gate. SECURITY INVARIANT: only a built-in lens may enter a
    /// camera flow. A virtual/loopback camera (OBS Virtual Camera, a CoreMediaIO
    /// DAL plugin) is reported as an EXTERNAL device, so every type that is not
    /// a known built-in fails closed here instead of being trusted downstream.
    public static func isAcceptableCameraSource(_ device: AVCaptureDevice) -> Bool {
        switch device.deviceType {
        case .builtInWideAngleCamera:
            return true
        #if os(iOS)
        // IOS-ONLY
        case .builtInUltraWideCamera, .builtInTelephotoCamera:
            return true
        #endif
        default:
            return false
        }
    }

    /// Device priority: built-in wide-angle back, then built-in wide-angle
    /// front, then the first accepted built-in device. Mirrors the macOS
    /// pickDevice() priority; an external/USB/virtual source is never selected.
    public static func pickDevice() -> AVCaptureDevice? {
        #if os(iOS)
        // IOS-ONLY
        let kinds: [AVCaptureDevice.DeviceType] = [
            .builtInWideAngleCamera,
            .builtInUltraWideCamera,
            .builtInTelephotoCamera,
        ]
        #else
        let kinds: [AVCaptureDevice.DeviceType] = [.builtInWideAngleCamera]
        #endif

        // 1. Back wide-angle (primary rear camera).
        if let back = AVCaptureDevice.default(
            .builtInWideAngleCamera, for: .video, position: .back
        ), isAcceptableCameraSource(back) { return back }

        // 2. Front wide-angle.
        if let front = AVCaptureDevice.default(
            .builtInWideAngleCamera, for: .video, position: .front
        ), isAcceptableCameraSource(front) { return front }

        // 3. First discovered device that passes the source gate.
        let discovery = AVCaptureDevice.DiscoverySession(
            deviceTypes: kinds,
            mediaType: .video,
            position: .unspecified
        )
        return discovery.devices.first(where: isAcceptableCameraSource)
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
