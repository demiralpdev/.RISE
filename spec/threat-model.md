# Threat Model

> 4 attacks + a mitigation for each. Kept simple.

## 1. Fake File

- Attack: an invented video is uploaded as a real file.
- Why it fails: no `SHA256(raw frame) at shutter + sign + timestamp`.
- Mitigation: signature + RFC 3161 timestamp mandatory. Unsigned files are `untrusted`.

## 2. Screen Re-capture

- Attack: content is played on another screen and re-recorded with a camera.
- Why it is hard: gold-L4 sensor-path stamps and frame timings become inconsistent.
- Mitigation: check frame intervals and stamp order. Mark `suspicious` on mismatch.

## 3. Key Theft

- Attack: the device key is copied and fake content is signed.
- Silver mitigation: rotate the key, add the old one to the revocation list.
- Gold mitigation: TEE/SE keys never leave the hardware. C2PA Trust List + revocation checks are mandatory.

## 4. After-the-fact Edit

- Attack: frames are cut, inserted, or filtered after capture.
- Why it is detected: the frame SHA-256 list changes, the signature fails.
- Mitigation: every frame hash is in the manifest (C2PA 2.4 + JUMBF). One changed frame fails verification.

## Global Rules

- If the C2PA Trust List is unreachable the decision is `unknown`.
- A revoked certificate = `untrusted`, no exceptions.
- References: C2PA 2.4, ISO 21617-1:2026, RFC 3161.
