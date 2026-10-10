// Attest.swift — iOS Secure Enclave P-256 + App Attest flow sketch (I-102).
//
// PLAN.md I-102: Secure Enclave + DeviceCheck/App Attest, downgrade on weak attest.
// Spec: spec/assurance-levels.md (silver / gold-L2 / gold-L4).
//
// This file holds keys and fetches attestation objects ONLY.
// NO signing of frames here, NO manifest building, NO verification.
// Signing happens in core/; the badge decision happens in verify-web/.
// This binding only REPORTS what the hardware said; it never upgrades a tier.
//
// STATUS: STUB — NOT device-tested. Requires a real iPhone + Apple backend.
// DeviceCheck / App Attest lines are IOS-ONLY and WILL fail
// `swiftc -typecheck` on macOS. That failure is expected and honest.

import CryptoKit
import Foundation
#if canImport(DeviceCheck)
import DeviceCheck // IOS-ONLY: App Attest service (iOS 14+); absent on macOS.
#endif

/// Hardware key holder. Secure Enclave P-256, non-exportable by design.
public enum RiseKeystore {

    /// Creates a new Secure Enclave P-256 signing key.
    /// Throws on devices without Secure Enclave (falls back: NO gold, see table).
    public static func createKey() throws -> SecureEnclave.P256.Signing.PrivateKey {
        // IOS-ONLY (line below): SecureEnclave keys need an A9+ iPhone;
        // macOS typecheck may pass the type but real creation needs hardware.
        return try SecureEnclave.P256.Signing.PrivateKey()
    }

    /// Loads an existing key by its `keyID` blob (as returned at creation).
    public static func loadKey(dataRepresentation: Data) throws -> SecureEnclave.P256.Signing.PrivateKey {
        // IOS-ONLY (line below): same hardware requirement as createKey().
        return try SecureEnclave.P256.Signing.PrivateKey(dataRepresentation: dataRepresentation)
    }
}

/// App Attest flow sketch. Steps the host app must perform in order:
///  1. `DCAppAttestService.shared.generateKey()` (per-app attestation key).
///  2. `attestKey(_:clientDataHash:)` against Apple servers -> attestation object.
///  3. Send attestation object + keyID to YOUR server; verify receipt with Apple.
///  4. `generateAssertion(_:clientDataHash:)` per capture for freshness.
///  5. Hand attestation result + frame bytes to core/; core + verifier decide tier.
///
/// Pseudo-usage (host app, iOS only):
/// ```swift
/// // IOS-ONLY: DCAppAttestService exists on iOS 14+ only.
/// let service = DCAppAttestService.shared
/// let keyID = try await service.generateKey()
/// let challenge = Data(serverNonce) // 32+ bytes from your server
/// let hash = Data(SHA256.hash(data: challenge))
/// let attestation = try await service.attestKey(keyID, clientDataHash: hash)
/// // POST (keyID, attestation) to server -> server validates with Apple.
/// ```
public enum RiseAttest {

    /// Returns true only when App Attest is actually available on this device.
    /// Simulator / jailbroken / unsupported OS -> false -> automatic downgrade.
    public static func isAppAttestSupported() -> Bool {
        #if canImport(DeviceCheck)
        if #available(iOS 14.0, *) {
            // IOS-ONLY (line below): DCAppAttestService is iOS-only.
            return DCAppAttestService.shared.isSupported
        }
        return false
        #else
        return false // macOS typecheck path: honestly unsupported here.
        #endif
    }

    /// Downgrade decision the BINDING reports upstream.
    /// The binding never upgrades; core/verifier apply the final badge.
    public enum AttestOutcome: String {
        case strong   // Secure Enclave key + valid App Attest receipt
        case weak     // key OK but attest weak/missing (simulator, old OS, no receipt)
        case broken   // chain broken / replay suspected / validation failed
    }

    /// Maps an attestation result to the MAXIMUM tier the verifier may show.
    /// (spec/assurance-levels.md: silver / gold-L2 / gold-L4.)
    public static func maxTier(for outcome: AttestOutcome) -> String {
        switch outcome {
        case .strong:
            // Still capped by the rest of the chain: sensor path + Trust List
            // checks in core/verify-web decide between gold-L2 and gold-L4.
            return "gold-L2-or-higher (verifier decides; L4 needs sensor path proof)"
        case .weak:
            return "silver (weak attest -> L2/silver at most; gold-L4 closed)"
        case .broken:
            return "red (broken chain / replay suspicion; binding does not argue)"
        }
    }
}

// MARK: - Downgrade table (binding -> verifier contract)
//
// | # | Attestation state                        | Binding reports | Max badge  |
// |---|------------------------------------------|-----------------|------------|
// | 1 | Secure Enclave P-256 + valid App Attest  | strong          | gold-L2/L4*|
// | 2 | Secure Enclave key, App Attest weak      | weak            | L2/silver  |
// |   | (simulator, iOS < 14, no Apple receipt)  |                 | at most    |
// | 3 | No Secure Enclave (old device)           | weak            | silver     |
// | 4 | App Attest validation failed             | broken          | red        |
// | 5 | Replay suspected (nonce/challenge miss)  | broken          | red        |
// | 6 | Broken chain from core (any red signal)  | broken          | red        |
//
// * Row 1: L4 additionally requires sensor-path + protected-pipeline proof
//   (spec/assurance-levels.md Gold-L4); this binding cannot assert L4 alone.
//   On ANY suspicion: downgrade the badge, never upgrade.
