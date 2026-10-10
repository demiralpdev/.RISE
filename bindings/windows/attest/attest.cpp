// attest.cpp — Windows keystore + attestation sketch (TPM 2.0 / VBS).
//
// HONESTY HEADER: NOT compiled, NOT tested — STUB. No Windows toolchain on
// this machine (static review only). Thin shell: reports keystore facts to
// core//verify-web. No signing logic here (keystore + attestation proof only).
//
// Build (on Windows, VS Developer Prompt):
//   cl /EHsc /std:c++17 attest.cpp ncrypt.lib tbs.lib
// Run:
//   attest.exe
//   Prints one line: TIER=<silver|gold-L2|gold-L4?> + reason.
//
// Keystore: platform key via NCrypt (MS_PLATFORM_KEY_STORAGE_PROVIDER),
// P-256 ECDSA. The key never leaves the TPM/Pluton; this file only opens the
// key handle and asks for an attestation blob. core/ verifies signatures.
//
// VBS status check: reads the DeviceGuard/VBS enablement state
// (see reference flow below). VBS confirmation is required for any gold tier.

#include <cstdio>
#include <string>

namespace {

// --- Windows/NCrypt includes (Windows-only; see honesty header) ---
// #include <windows.h>
// #include <ncrypt.h>
// #define NCRYPT_PLATFORM_PROVIDER L"Microsoft Platform Crypto Provider"

enum class Tier { kSilver, kGoldL2, kRed };

struct AttestFacts {
  bool tpm_present = false;      // TPM 2.0 reachable via TBS/NCrypt.
  bool key_in_tpm = false;       // P-256 key opened from platform provider.
  bool vbs_enabled = false;      // VBS / HVCI confirmation.
  bool attest_quote_ok = false;  // TPM quote / key attestation blob verifies.
  bool chain_broken = false;     // Replay suspicion, PCR mismatch, any break.
};

// Downgrade table (fail-closed; the binding never upgrades):
//
// | Facts                                  | Tier     | Reason                  |
// |----------------------------------------|----------|-------------------------|
// | !tpm_present OR !vbs_enabled           | silver   | no TPM/VBS -> silver-only |
// | tpm + VBS, but !attest_quote_ok        | gold-L2* | weak attest -> L2/silver |
// | chain_broken (any suspicion)           | red      | broken -> red           |
// | tpm + VBS + quote ok + clean chain     | gold-L2  | max for this binding    |
//
// * Weak attestation + clean chain = gold-L2 at most (never L4: USB webcam
//   and no protected sensor path on this binding). Any USB source caps at
//   silver regardless of attestation strength (see capture.cpp note).
//
// Gold-L4 is NOT reachable from this binding: no sensor-output hash point
// and no protected media path. A caller asking for L4 gets L2 at most.

const char* DecideTier(const AttestFacts& f, std::string* reason_out) {
  std::string reason;
  Tier tier;
  if (f.chain_broken) {
    tier = Tier::kRed;
    reason = "broken chain or replay suspicion";
  } else if (!f.tpm_present || !f.vbs_enabled) {
    tier = Tier::kSilver;
    reason = "no TPM 2.0 and/or VBS: silver-only";
  } else if (!f.key_in_tpm || !f.attest_quote_ok) {
    tier = Tier::kGoldL2;
    // Weak attestation + clean chain still lands here (L2 at most), and a
    // verifier may further cap to silver for USB sources.
    reason = "weak attestation: L2 at most, silver for USB sources";
  } else {
    tier = Tier::kGoldL2;
    reason = "TPM 2.0 P-256 key + VBS + quote ok; L4 unreachable here";
  }
  if (reason_out != nullptr) *reason_out = reason;
  switch (tier) {
    case Tier::kGoldL2:
      return "gold-L2";
    case Tier::kRed:
      return "red";
    case Tier::kSilver:
    default:
      return "silver";
  }
}

}  // namespace

// Real flows (reference; require Windows SDK):
//
// VBS check:
//   HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\DeviceGuard
//     "EnableVirtualizationBasedSecurity" == 1  AND
//   Win32_DeviceGuard WMI class: VirtualizationBasedSecurityStatus == 2
//     (2 = enabled and running). Anything else => vbs_enabled = false.
//
// TPM P-256 key (platform provider):
//   NCryptOpenStorageProvider(&hProv, MS_PLATFORM_KEY_STORAGE_PROVIDER, 0);
//   NCryptOpenKey(hProv, &hKey, L"RISE-device-key", 0, 0);
//   // Key created once with NCRYPT_MACHINE_KEY_FLAG + TPM attestation blob
//   // (NCRYPTBUFFER_TPM_SEAL_PASSWORD / attestation quote via TBS).
//   // This file never calls NCryptSignHash; signing belongs to core/.

int main() {
  AttestFacts facts;  // All false on this machine: no TPM/VBS query ran.
  std::string reason;
  const char* tier = DecideTier(facts, &reason);
  std::printf("STUB: attest.cpp was not compiled or tested on Windows.\n");
  std::printf("TIER=%s reason=%s\n", tier, reason.c_str());
  return 0;
}
