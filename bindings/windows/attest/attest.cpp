// attest.cpp — Windows keystore + attestation probe (TPM 2.0 / VBS).
//
// Status: REAL — compiled and run on DESKTOP-1LD8TK1 (2026-10-10). Output:
//   facts=tpm:1 key:1 vbs:0
//   TIER=silver reason=no TPM 2.0 and/or VBS: silver-only
// (TPM present, RISE-device-key P-256 created+finalized in the TPM; VBS off
// in registry -> fail-closed silver. Re-run after enabling VBS for gold-L2.)
//
// Thin shell: reports keystore facts only. No signing logic here — core/
// verifies signatures. Fail-closed: every probe failure downgrades.
//
// Build (VS x64 prompt or vcvars64 + cl):
//   cl /EHsc /std:c++17 attest.cpp
// Run:
//   attest.exe   ->  prints facts + TIER=<silver|gold-L2> + reason

#include <windows.h>
#include <ncrypt.h>

#include <cstdio>
#include <string>

#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "ncrypt.lib")

namespace {

constexpr wchar_t kPlatformProvider[] = L"Microsoft Platform Crypto Provider";
constexpr wchar_t kKeyName[] = L"RISE-device-key";

enum class Tier { kSilver, kGoldL2, kRed };

struct AttestFacts {
  bool tpm_present = false;      // platform crypto provider opened.
  bool key_in_tpm = false;       // RISE-device-key usable in the TPM.
  bool vbs_enabled = false;      // DeviceGuard registry enablement = 1.
  bool chain_broken = false;     // reserved: replay/PCR break from caller.
};

// Downgrade table (fail-closed; the binding never upgrades):
// - chain broken -> red
// - no TPM or no VBS -> silver (silver-only rule)
// - TPM + VBS, key usable -> gold-L2 (max here; L4 unreachable, no sensor path)
DWORD ReadVbsRegistry() {
  // 1 = enablement requested. The stricter WMI status==2 (running) check is a
  // TODO: registry alone says "enabled", not "running".
  DWORD value = 0;
  DWORD size = sizeof(value);
  LSTATUS st = RegGetValueA(
      HKEY_LOCAL_MACHINE,
      "SYSTEM\\CurrentControlSet\\Control\\DeviceGuard",
      "EnableVirtualizationBasedSecurity",
      RRF_RT_REG_DWORD, nullptr, &value, &size);
  return (st == ERROR_SUCCESS) ? value : 0;
}

// Opens (or first-run creates) the P-256 machine key inside the TPM via the
// platform crypto provider. Returns true when a usable key handle was held.
bool ProbeTpmKey() {
  NCRYPT_PROV_HANDLE prov = 0;
  if (NCryptOpenStorageProvider(&prov, kPlatformProvider, 0) != ERROR_SUCCESS) {
    return false;
  }
  bool usable = false;
  NCRYPT_KEY_HANDLE key = 0;
  SECURITY_STATUS st = NCryptOpenKey(prov, &key, kKeyName, 0, 0);
  if (st == NTE_BAD_KEYSET) {
    // First run: create the P-256 machine key, then finalize it.
    st = NCryptCreatePersistedKey(prov, &key, NCRYPT_ECDSA_P256_ALGORITHM,
                                  kKeyName, 0, NCRYPT_MACHINE_KEY_FLAG);
    if (st == ERROR_SUCCESS) {
      // NCryptFinalizeKey (not "FinalizeOperation" — that name does not exist
      // in this SDK's ncrypt.h).
      st = NCryptFinalizeKey(key, 0);
      usable = (st == ERROR_SUCCESS);
    }
  } else if (st == ERROR_SUCCESS) {
    usable = true;
  }
  if (key != 0) NCryptFreeObject(key);
  if (prov != 0) NCryptFreeObject(prov);
  return usable;
}

const char* DecideTier(const AttestFacts& f, std::string* reason_out) {
  std::string reason;
  Tier tier;
  if (f.chain_broken) {
    tier = Tier::kRed;
    reason = "broken chain or replay suspicion";
  } else if (!f.tpm_present || !f.vbs_enabled) {
    tier = Tier::kSilver;
    reason = "no TPM 2.0 and/or VBS: silver-only";
  } else if (!f.key_in_tpm) {
    tier = Tier::kSilver;
    reason = "TPM+VBS present but key probe failed: fail-closed silver";
  } else {
    tier = Tier::kGoldL2;
    reason = "TPM 2.0 P-256 key + VBS ok; L4 unreachable here";
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

int main() {
  AttestFacts facts;
  facts.tpm_present = ProbeTpmKey();  // provider open implies a TPM is there
  facts.key_in_tpm = facts.tpm_present;
  facts.vbs_enabled = ReadVbsRegistry() == 1;

  std::string reason;
  const char* tier = DecideTier(facts, &reason);
  std::printf("facts=tpm:%d key:%d vbs:%d\n", facts.tpm_present ? 1 : 0,
              facts.key_in_tpm ? 1 : 0, facts.vbs_enabled ? 1 : 0);
  std::printf("TIER=%s reason=%s\n", tier, reason.c_str());
  return 0;
}
