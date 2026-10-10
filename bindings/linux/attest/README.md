# Linux Attestation (thin shell, report-only)

> STUB: untested on real Linux hardware (author machine is macOS). Logic is
> static-checked only (`bash -n`, plus shellcheck where available).

- Script: `detect.sh` — TPM2 presence check, prints `key=value` lines.
- Policy: **Linux ceiling is silver — never gold.** The gold path is closed
  on Linux even when a TPM2 is present (spec `solutions-physical.md` §5:
  USB/external sources never get gold; without an internal attested sensor
  the gold path is closed).
- No signing logic here. The binding reports; `core/` decides.

## Signals

| Signal | Meaning |
| --- | --- |
| `TPM2_PRESENT=yes` | `/dev/tpm0`, `/dev/tpmrm0`, or `tpm2_pcrread` works |
| `TPM2_PRESENT=no` | No TPM2 device found; silver cap (no hardware attestation) |
| `CEILING=silver` | Always silver on Linux |
| `GOLD_PATH=closed` | Always closed on Linux |

## Decision table

- TPM2 present + clean chain = silver at most.
- No TPM2 key = silver at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.
