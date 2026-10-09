# Solutions: Platform Cases (10)

## Global Rules
- Sharing is verify-link only; no file/PDF submission.
- Badge language is fixed: Gold/Silver/Red + unverifiable.
- Multi-TSA mandatory; a single TSA signature is invalid.
- Key rotation 90 days; the old key goes on the revocation list.

## 1. Manifest strip
- UX/rule: if the platform strips it, no badge is shown; a link is required.
- Technique: the manifest hash is checked in verify-web.

## 2. Gold/Silver confusion
- UX/rule: badge language is fixed, one explanation template.
- Technique: strings outside the tier enum are forbidden.

## 3. Insider
- UX/rule: one person cannot approve; 2 signatures required.
- Technique: RBAC + signing authority separation, append-only log.

## 4. Bribed TSA
- UX/rule: "single TSA cannot be trusted" disclaimer.
- Technique: multi-TSA cross-stamping; rejection on mismatch.

## 5. GPS mock
- UX/rule: automatic downgrade notification on mock suspicion.
- Technique: mock = auto downgrade; Play Integrity check.

## 6. Witness collusion
- UX/rule: witnesses on the same device/IP are flagged.
- Technique: witness independence score below threshold -> downgrade.

## 7. Court rejection
- UX/rule: "Not evidence, preliminary finding" disclaimer mandatory.
- Technique: eIDAS report output (PAdES/XAdES) generation.

## 8. Screenshot sharing
- UX/rule: the share button only produces a verify link.
- Technique: screenshots carry no metadata, the link is signed.

## 9. Privacy off
- UX/rule: warning that gold is not issued with location off.
- Technique: no no-GPS gold tier; silver at most.

## 10. Old phones
- UX/rule: silver is suggested when hardware is insufficient.
- Technique: old phone = auto silver; API-level check.
