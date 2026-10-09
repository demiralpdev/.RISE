# iOS Binding (thin shell)

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: Secure Enclave + App Attest
- Task: camera frame -> core hash/attest call
- No real code yet, skeleton only.

## Gold-gate checklist

- No gold without a Secure Enclave key + valid App Attest.
- If attestation is weak, downgrade from gold-L4 to gold-L2 or silver.
- Weak attestation + clean chain = gold-L2 at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.
