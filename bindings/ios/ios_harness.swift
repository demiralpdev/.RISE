import Foundation

// On-device/on-simulator proof harness: hash fixed proof bytes through the
// Rust core FFI and print them for comparison with the desktop rise-core.
let proof = Array("rise-ios-sim-proof-001".utf8)
var out = [CChar](repeating: 0, count: 65)
proof.withUnsafeBufferPointer { iptr in
    out.withUnsafeMutableBufferPointer { optr in
        rise_hash_frame(iptr.baseAddress, iptr.count, optr.baseAddress)
    }
}
print("IOS_HASH: \(String(cString: out))")
