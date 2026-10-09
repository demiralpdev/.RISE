# Linux Binding (thin shell)

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: TPM2 optional
- Task: camera frame -> core hash/attest call
- No real code yet, skeleton only.

## Gold-gate checklist

- No TPM2 key: no gold, target silver at most.
- No hardware attestation: gold-L4/L2 closed, downgrade to silver.
- Software key + clean chain = silver at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.
