# Physical Attack Solutions

> Core: `SHA256(raw frame) + sign + timestamp`. An unsigned frame is not evidence.

## Mandatory Rules

- Gold is issued only from an internal attested sensor; external sources never get gold.
- USB camera = automatic silver, anything above is forbidden.
- Encrypted MIPI is required for L4; unencrypted MIPI cannot grant L4.
- Nonce light challenge is mandatory.
- Depth is required for L4; no depth, no L4.
- Sensor serial matching is mandatory; a mismatched sensor is red.

## 1. Screen Re-capture

- Rule: frame interval + moire/refresh artifacts are checked.
- Technique: the scene must answer the nonce light challenge; a screen answer does not hold.
- Badge: red on failure; silver with no artifacts; gold if the L2/L4 sensor path is clean.

## 2. Printed Photo

- Rule: nonce light response + micro motion (parallax) are required.
- Technique: flash/nonce reflection change is measured; prints show no change.
- Badge: red with no response; silver single-view; gold with L2/L4 + depth.

## 3. 3D Mask

- Rule: depth + skin reflectance (IR/texture) are checked together.
- Technique: the depth map is cross-checked with liveness; mask texture is eliminated.
- Badge: red without depth; silver cap; gold with L4 depth + IR.

## 4. Projection / Hologram

- Rule: projection flicker + depth consistency are checked.
- Technique: the nonce light sequence is verified in the scene; projected light lags/distorts.
- Badge: red on mismatch; silver at most; gold with L4 only with real depth.

## 5. Fake USB Camera

- Rule: source identification mandatory; USB VID/PID + attestation required.
- Technique: without internal sensor attestation the gold path is closed.
- Badge: USB is always automatic silver; no gold; red without attestation.

## 6. Lens Overlay

- Rule: pre/post nonce frame comparison is performed.
- Technique: response to a known light pattern + edge/optical consistency are checked.
- Badge: red on deviation; silver if clean; gold with L2/L4 internal sensor.

## 7. HDMI / MIPI Injector

- Rule: encrypted MIPI + sensor signature are required; external injection is blocked.
- Technique: sensor output is hashed from an encrypted channel (`SHA256(raw frame) at shutter`).
- Badge: red for unencrypted/unsigned sources; silver if encrypted but external; L4 only with internal encrypted MIPI.

## 8. Depth Spoofing

- Rule: depth + RGB timestamps must match.
- Technique: depth/RGB sync and hardware signature are verified; injected depth is eliminated.
- Badge: red on mismatch; close to red without depth; gold with L4 dual-signed depth.

## 9. IR Replay

- Rule: every capture uses a fresh nonce IR pattern; replays are rejected.
- Technique: the nonce is single-use; recorded IR responses fall out of the time window.
- Badge: red on replay detection; silver with fresh nonce; gold with L2/L4 attested IR.

## 10. Light Challenge Bypass

- Rule: the nonce light challenge cannot be skipped; unanswered captures are not upgraded.
- Technique: the scene's reflection of a random color/duration light burst is measured.
- Badge: red if missing/invalid; silver if valid; gold with L2/L4 + sensor attestation.

## 11. Sensor Swap

- Rule: the sensor serial number must match the device key.
- Technique: attested serial check at boot; on swap the key is revoked + re-pairing required.
- Badge: red on mismatch; silver if matching; gold with L2/L4 only with factory matching.
