# macOS Binding (thin shell)

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: Secure Enclave
- Task: camera frame -> core hash/attest call
- Reference implementation: see `capture/`.

## Gold-gate checklist

- No gold without a Secure Enclave key.
- If platform attestation is weak, downgrade from gold-L4 to gold-L2 or silver.
- Weak attestation + clean chain = gold-L2 at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.
