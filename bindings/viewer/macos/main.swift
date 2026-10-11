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
}

enum RiseResult {
    case ok(cleanJPEG: Data, manifest: RiseManifest, hashMatches: Bool)
    case broken(reason: String)
}

/// Strips RISE APP11 segments (FF EB containing "rise_version") from a JPEG,
/// returning the clean original byte stream and the embedded manifest JSON.
func parseRise(_ data: Data) -> RiseResult {
    guard data.count > 4, data[0] == 0xFF, data[1] == 0xD8 else {
        return .broken(reason: "Not a JPEG-based .rise file.")
    }
    var out = Data(data[0...1])
    var manifestJSON: Data?
    var i = 2
    while i + 4 <= data.count {
        guard data[i] == 0xFF else { return .broken(reason: "JPEG desync at byte \(i).") }
        let marker = data[i + 1]
        if marker == 0xD9 { out.append(data[i...i + 1]); i += 2; break }
        if marker == 0xDA { // SOS: copy header, then entropy stream verbatim
            let seglen = Int(data[i + 2]) << 8 | Int(data[i + 3])
            out.append(data[i...i + 1 + seglen])
            i += 2 + seglen
            if i < data.count { out.append(data[i...]) }
            i = data.count
            break
        }
        let seglen = Int(data[i + 2]) << 8 | Int(data[i + 3])
        let end = i + 2 + seglen
        guard end <= data.count, seglen >= 2 else { return .broken(reason: "Bad segment length.") }
        let seg = data[i..<end]
        if marker == 0xEB, seg.contains("rise_version".data(using: .utf8)!) {
            if let b = manifestJSON { _ = b } else {
                if let s = seg.firstIndex(of: UInt8(ascii: "{")),
                   let e = seg.lastIndex(of: UInt8(ascii: "}")), s < e {
                    manifestJSON = seg[s...e]
                }
            }
        } else {
            out.append(seg)
        }
        i = end
    }
    guard let mj = manifestJSON,
          let manifest = try? JSONDecoder().decode(RiseManifest.self, from: mj) else {
        return .broken(reason: "No embedded RISE manifest found.")
    }
    // Hash check applies to single-frame (photo) seals.
    var matches = false
    if let first = manifest.frame_hashes.first {
        let digest = SHA256.hash(data: out).map { String(format: "%02x", $0) }.joined()
        matches = (digest == first)
    }
    return .ok(cleanJPEG: out, manifest: manifest, hashMatches: matches)
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
        case .ok(let clean, let manifest, let matches):
            imageView.image = NSImage(data: clean)
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

            infoField.stringValue =
                "device: \(device)   sealed: \(dateStr)   alg: \(alg)   assurance: \(assurance)\n" +
                "key: \(keyId)   merkle: \(rootShort)   frames: \(frames)\n" +
                "signature check + badge: riseproof.me"

            if frames > 1 {
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
            if frames == 1 {
                print(matches ? "SELFTEST-OK: UNALTERED single-frame, device=\(manifest.device_id ?? "?")"
                             : "SELFTEST-ALTERED: hash mismatch, device=\(manifest.device_id ?? "?")")
            } else {
                print("SELFTEST-OK: video manifest, frames=\(frames), device=\(manifest.device_id ?? "?")")
            }
        }
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
