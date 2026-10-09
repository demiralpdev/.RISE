# verify-web (verifier skeleton)

Shows the uploaded evidence: gold / silver / red badge.

## Badge decision tree

- gold-L4: hardware seal + STRONG verdict + clean chain -> full legal weight.
- gold-L2: hardware seal + BASIC verdict + clean chain -> limited weight.
- silver: software key or small chain gap -> informational.
- red: invalid signature, broken chain, or replay -> reject.
- Rule: no STRONG, no gold-L4; drop one tier.
- Rule: broken chain / replay -> straight red.
- Rule: on suspicion, downgrade the badge, never upgrade.
- Badges define legal weight: gold-L4 > gold-L2 > silver > red.
- Each badge shows a one-line reason on screen.
- MS1: real verification wired to `core/` output; Trust List/OCSP come later.

## Run

```bash
cd verify-web
python3 -m pytest tests/ -q        # badge tree tests
uvicorn app:app --port 8088        # MVP API
```
