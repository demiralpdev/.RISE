# Android Binding (thin shell)

Responsibility: capture the frame, send it to `core/`. No signing logic here.

- Hardware store: StrongBox + Play Integrity
- Task: camera frame -> core hash/attest call
- No real code yet, skeleton only.

## Gold-gate checklist

- No gold without a StrongBox key + Play Integrity STRONG verdict.
- If STRONG is missing, downgrade from gold-L4 to gold-L2 or silver.
- BASIC verdict + clean chain = gold-L2 at most.
- Broken chain / replay suspicion = red, the binding does not argue.
- On suspicion, downgrade the badge, never upgrade.
