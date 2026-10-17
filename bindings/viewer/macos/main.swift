// RISE Viewer — opens .rise files, shows the image and the seal status.
// Single-file AppKit app. No loops, no relaunch, no watchers.
// Self-test: RISE\ Viewer --selftest <file.rise>  prints status, exits 0.

import AppKit
import CryptoKit
import Foundation

// MARK: - .rise parsing

struct RiseManifest: Decodable {
    let rise_version: Int
    let frame_hashes: [String]
    let merkle_root: String?
    let sig_alg: String?
    let timestamp_ms: Int?
    let device_id: String?
    let assurance: String?
    let signing_key_id: String?
    let capture_source: String?
}

/// Origin disclaimers for non-camera capture classes (mirrors core's
/// `non_camera_note` and verify-web's `_NON_CAMERA_SOURCES`). The viewer must
/// never show a non-camera seal as camera evidence.
func nonCameraNote(_ src: String?) -> String? {
    switch src {
    case "screen": return "SCREEN/IMPORTED CAPTURE — the seal is real, the origin is NOT verified"
    case "obs": return "OBS/SOFTWARE CAPTURE — not from a real camera sensor"
    case "virtual": return "VIRTUAL CAMERA — software source, not camera evidence"
    case "external_usb": return "EXTERNAL USB CAMERA — camera evidence is capped"
    default: return nil
    }
}

enum RiseResult {
    case ok(image: Data?, manifest: RiseManifest, hashMatches: Bool)
    case broken(reason: String)
}

let pngSignature: [UInt8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]

func be32(_ data: Data, _ i: Int) -> Int {
    Int(data[i]) << 24 | Int(data[i + 1]) << 16 | Int(data[i + 2]) << 8 | Int(data[i + 3])
}

func decodeManifest(_ json: Data) -> RiseManifest? {
    try? JSONDecoder().decode(RiseManifest.self, from: json)
}

/// Strips RISE APP11 segments (FF EB containing "rise_version") from a JPEG,
/// returning the clean original byte stream and the embedded manifest JSON.
func parseRise(_ data: Data) -> RiseResult {
    // JPEG: APP11 JUMBF segment.
    if data.count > 3, data[0] == 0xFF, data[1] == 0xD8, data[2] == 0xFF {
        var out = Data(data[0...1])
        var manifestJSON: Data?
        var i = 2
        while i + 4 <= data.count {
            guard data[i] == 0xFF else { return .broken(reason: "JPEG desync at byte \(i).") }
            let marker = data[i + 1]
            if marker == 0xD9 { out.append(data[i...i + 1]); break }
            if marker == 0xDA {
                let seglen = be32(data, i + 2) >> 16
                out.append(data[i...i + 1 + seglen])
                i += 2 + seglen
                if i < data.count { out.append(data[i...]) }
                break
            }
            let seglen = be32(data, i + 2) >> 16
            let end = i + 2 + seglen
            guard end <= data.count, seglen >= 2 else { return .broken(reason: "Bad segment length.") }
            let seg = data[i..<end]
            if marker == 0xEB, seg.contains("rise_version".data(using: .utf8)!) {
                if manifestJSON == nil,
                   let s = seg.firstIndex(of: UInt8(ascii: "{")),
                   let e = seg.lastIndex(of: UInt8(ascii: "}")), s < e {
                    manifestJSON = seg[s...e]
                }
            } else {
                out.append(seg)
            }
            i = end
        }
        guard let mj = manifestJSON, let manifest = decodeManifest(mj) else {
            return .broken(reason: "No embedded RISE manifest found.")
        }
        var matches = false
        if let first = manifest.frame_hashes.first {
            let digest = SHA256.hash(data: out).map { String(format: "%02x", $0) }.joined()
            matches = (digest == first)
        }
        return .ok(image: out, manifest: manifest, hashMatches: matches)
    }

    // PNG: ancillary `risE` chunk.
    if data.count > 8, Array(data[0..<8]) == pngSignature {
        var out = Data(data[0..<8])
        var manifestJSON: Data?
        var i = 8
        while i + 8 <= data.count {
            let len = be32(data, i)
            let end = i + 8 + len + 4
            guard len >= 0, end <= data.count else { return .broken(reason: "PNG chunk out of range.") }
            let kind = data[i + 4..<i + 8]
            if kind == Data("risE".utf8) {
                if manifestJSON == nil { manifestJSON = data[i + 8..<i + 8 + len] }
            } else {
                out.append(data[i..<end])
            }
            if kind == Data("IEND".utf8) { break }
            i = end
        }
        guard let mj = manifestJSON, let manifest = decodeManifest(mj) else {
            return .broken(reason: "No embedded RISE manifest found.")
        }
        var matches = false
        if let first = manifest.frame_hashes.first, manifest.frame_hashes.count == 1 {
            let digest = SHA256.hash(data: out).map { String(format: "%02x", $0) }.joined()
            matches = (digest == first)
        }
        return .ok(image: out, manifest: manifest, hashMatches: matches)
    }

    // MP4: top-level `uuid` box (RISE UUID, C2PA convention) or legacy `rise` box
    // (video manifest — no image preview).
    if data.count > 8, String(data: data[4..<8], encoding: .ascii) == "ftyp" {
        let riseUuid: [UInt8] = [0x72, 0x69, 0x73, 0x65, 0x00, 0x00, 0x40, 0x80,
                                 0x00, 0x00, 0x00, 0x00, 0x52, 0x49, 0x53, 0x45]
        var i = 0
        while i + 8 <= data.count {
            let size = be32(data, i)
            if size == 0 || size < 8 || i + size > data.count { break }
            if data[i + 4..<i + 8] == Data("uuid".utf8),
               i + 24 <= data.count, Array(data[i + 8..<i + 24]) == riseUuid {
                guard let manifest = decodeManifest(data[i + 24..<i + size]) else {
                    return .broken(reason: "Corrupted embedded manifest.")
                }
                return .ok(image: nil, manifest: manifest, hashMatches: false)
            }
            if data[i + 4..<i + 8] == Data("rise".utf8) {
                guard let manifest = decodeManifest(data[i + 8..<i + size]) else {
                    return .broken(reason: "Corrupted embedded manifest.")
                }
                return .ok(image: nil, manifest: manifest, hashMatches: false)
            }
            i += size
        }
        return .broken(reason: "No embedded RISE manifest found.")
    }

    return .broken(reason: "Not a JPEG, PNG or MP4 based .rise file.")
}

// MARK: - UI

final class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var imageView: NSImageView!
    var statusField: NSTextField!
    var infoField: NSTextField!

    func applicationDidFinishLaunching(_ note: Notification) {
        let args = CommandLine.arguments
        if args.count >= 3, args[1] == "--selftest" {
            selftest(path: args[2])
            NSApplication.shared.terminate(self)
            return
        }
        buildWindow()
        if args.count >= 2, !args[1].hasPrefix("-") {
            load(path: args[1])
        } else {
            show(emptyState: true)
        }
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        guard let url = urls.first else { return }
        load(path: url.path)
    }

    @objc func openDocument(_ sender: Any?) {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = []
        panel.allowsOtherFileTypes = true
        panel.message = "Select a .rise file"
        if panel.runModal() == .OK, let url = panel.url {
            load(path: url.path)
        }
    }

    private func buildWindow() {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 960, height: 680),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered, defer: false)
        window.title = "RISE Viewer"
        window.center()

        let content = NSView(frame: window.contentRect(forFrameRect: window.frame))

        imageView = NSImageView(frame: NSRect(x: 0, y: 180, width: 960, height: 500))
        imageView.imageScaling = .scaleProportionallyUpOrDown
        imageView.autoresizingMask = [.width, .height]

        statusField = NSTextField(labelWithString: "")
        statusField.font = NSFont.monospacedSystemFont(ofSize: 15, weight: .bold)
        statusField.alignment = .center
        statusField.frame = NSRect(x: 0, y: 132, width: 960, height: 28)
        statusField.autoresizingMask = .width

        infoField = NSTextField(labelWithString: "")
        infoField.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        infoField.textColor = .systemGray
        infoField.alignment = .center
        infoField.lineBreakMode = .byTruncatingMiddle
        infoField.frame = NSRect(x: 16, y: 16, width: 928, height: 104)
        infoField.autoresizingMask = [.width, .minYMargin]

        let separator = NSBox(frame: NSRect(x: 0, y: 168, width: 960, height: 1))
        separator.boxType = .separator
        separator.autoresizingMask = .width

        content.addSubview(imageView)
        content.addSubview(separator)
        content.addSubview(statusField)
        content.addSubview(infoField)
        window.contentView = content

        let mainMenu = NSMenu()
        let appItem = NSMenuItem()
        let appMenu = NSMenu()
        appMenu.addItem(withTitle: "Quit RISE Viewer", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        appItem.submenu = appMenu
        mainMenu.addItem(appItem)
        let fileItem = NSMenuItem()
        let fileMenu = NSMenu(title: "File")
        fileMenu.addItem(withTitle: "Open…", action: #selector(openDocument(_:)), keyEquivalent: "o")
        fileItem.submenu = fileMenu
        mainMenu.addItem(fileItem)
        NSApplication.shared.menu = mainMenu

        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    private func load(path: String) {
        window.title = "RISE Viewer — " + (path as NSString).lastPathComponent
        guard let data = FileManager.default.contents(atPath: path) else {
            show(error: "Cannot read file.")
            return
        }
        switch parseRise(data) {
        case .broken(let reason):
            show(error: reason)
        case .ok(let imageData, let manifest, let matches):
            imageView.image = imageData.flatMap { NSImage(data: $0) }
            let frames = manifest.frame_hashes.count
            let dateStr = manifest.timestamp_ms.map { ms in
                let d = Date(timeIntervalSince1970: TimeInterval(ms) / 1000)
                let f = DateFormatter(); f.dateFormat = "yyyy-MM-dd HH:mm:ss"; f.timeZone = .current
                return f.string(from: d)
            } ?? "unknown time"
            let device = manifest.device_id ?? "?"
            let alg = manifest.sig_alg ?? "?"
            let assurance = manifest.assurance ?? "?"
            let keyId = manifest.signing_key_id ?? "?"
            let root = manifest.merkle_root ?? "—"
            let rootShort = root.count > 20 ? String(root.prefix(20)) + "…" : root

            let source = manifest.capture_source ?? "camera"
            infoField.stringValue =
                "device: \(device)   sealed: \(dateStr)   alg: \(alg)   assurance: \(assurance)\n" +
                "key: \(keyId)   origin: \(source)   merkle: \(rootShort)   frames: \(frames)\n" +
                "signature check + badge: riseproof.me"

            if let note = nonCameraNote(manifest.capture_source) {
                statusField.stringValue = "◈ " + note
                statusField.textColor = NSColor(red: 1.0, green: 0.72, blue: 0.1, alpha: 1)
            } else if frames > 1 {
                statusField.stringValue = "◈ SEALED VIDEO — \(frames) frames, verify per frame on riseproof.me"
                statusField.textColor = NSColor(red: 1.0, green: 0.72, blue: 0.1, alpha: 1)
            } else if matches {
                statusField.stringValue = "✓ UNALTERED — pixels match the sealed hash"
                statusField.textColor = NSColor(red: 0.2, green: 0.9, blue: 0.35, alpha: 1)
            } else {
                statusField.stringValue = "✗ ALTERED — file changed after sealing"
                statusField.textColor = NSColor(red: 1.0, green: 0.25, blue: 0.2, alpha: 1)
            }
        }
    }

    private func show(error: String) {
        imageView.image = nil
        statusField.stringValue = "✗ " + error
        statusField.textColor = .systemRed
        infoField.stringValue = "A .rise file is a JPEG (or MP4 proof container) with an embedded sealed manifest."
    }

    private func show(emptyState: Bool) {
        imageView.image = nil
        statusField.stringValue = "Open a .rise file — Cmd+O"
        statusField.textColor = NSColor(red: 1.0, green: 0.72, blue: 0.1, alpha: 1)
        infoField.stringValue = "RISE Viewer 0.2.1 — drop a .rise file or press Cmd+O"
    }

    private func selftest(path: String) {
        guard let data = FileManager.default.contents(atPath: path) else {
            print("SELFTEST-FAIL: cannot read \(path)"); exit(1)
        }
        switch parseRise(data) {
        case .broken(let reason):
            print("SELFTEST-FAIL: \(reason)"); exit(1)
        case .ok(_, let manifest, let matches):
            let frames = manifest.frame_hashes.count
            let origin = manifest.capture_source ?? "camera"
            if frames == 1 {
                print(matches ? "SELFTEST-OK: UNALTERED single-frame, device=\(manifest.device_id ?? "?"), origin=\(origin)"
                             : "SELFTEST-ALTERED: hash mismatch, device=\(manifest.device_id ?? "?"), origin=\(origin)")
            } else {
                print("SELFTEST-OK: video manifest, frames=\(frames), device=\(manifest.device_id ?? "?"), origin=\(origin)")
            }
        }
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
