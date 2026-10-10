#!/bin/bash
# Probe TPM2 presence on this Linux machine. Report-only: no signing logic here.
# Policy: Linux ceiling is silver — the gold path is closed on Linux (see README.md).
# Output: key=value lines on stdout; exit 0 always (absence of TPM2 is not an error).
set -e
cd "$(dirname "$0")"

TPM2_PRESENT="no"
TPM2_DETAIL="none"

if [ -e /dev/tpm0 ] || [ -e /dev/tpmrm0 ]; then
  TPM2_PRESENT="yes"
  TPM2_DETAIL="device-node"
fi
if command -v tpm2_pcrread >/dev/null 2>&1; then
  if tpm2_pcrread >/dev/null 2>&1; then
    TPM2_PRESENT="yes"
    TPM2_DETAIL="tpm2_pcrread-ok"
  else
    # Tools installed but no usable TPM: keep any device-node finding, else record the failure.
    if [ "$TPM2_PRESENT" = "no" ]; then
      TPM2_DETAIL="tpm2-tools-no-tpm"
    fi
  fi
fi

echo "TPM2_PRESENT=$TPM2_PRESENT"
echo "TPM2_DETAIL=$TPM2_DETAIL"
echo "CEILING=silver"
echo "GOLD_PATH=closed"
if [ "$TPM2_PRESENT" = "no" ]; then
  echo "NOTE=no TPM2 device; silver cap applies (no hardware attestation)."
else
  echo "NOTE=TPM2 present ($TPM2_DETAIL); ceiling stays silver, gold path stays closed on Linux."
fi
