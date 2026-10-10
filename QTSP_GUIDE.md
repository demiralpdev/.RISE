# Qualified TSA (eIDAS) — Integration Guide

> Technical side is DONE: `core/src/timestamp.rs` speaks RFC 3161 against any
> TSA URL (tested live against FreeTSA), multi-TSA cross-check is implemented,
> and stamps are embedded in the manifest. What remains is a PAID subscription
> with identity verification — a user action, not a code change.

## What a qualified timestamp buys

- FreeTSA (current): technical proof of existence, **no legal weight**.
- Qualified TSA (eIDAS Art. 41(2)): **legal presumption** — the timestamp is
  valid across the EU courts without further proof. Turkish equivalent:
  qualified electronic timestamp under the Turkish e-Signature Law (5070).

## Options (2026)

| Option | Region | Requires | Rough cost |
|---|---|---|---|
| **KamuSM e-Damga** | TR | e-İmza first (Mobil İmza via GSM operator, or smart card through e-Devlet) | small annual fee + per-stamp |
| **DigiCert / GlobalSign / Entrust qualified timestamp** | EU (eIDAS) | identity/org validation (video or documents) | ~100-300 EUR/yr |
| FreeTSA (current) | none | nothing | free, no legal presumption |

## Recommended flow (Turkey)

1. e-Devlet login -> activate **Mobil İmza** at your GSM operator
   (Turkcell/Vodafone/Türk Telekom; annual, cheap, no smart card needed).
2. Subscribe to **KamuSM e-Damga / qualified timestamp** with that e-İmza.
3. Receive: TSA endpoint URL (+ credentials if the plan needs them).
4. Give the URL (+ creds) to the project -> one-line config:
   `rise-core stamp --in m.json --tsa <QTSP-URL> --tsa2 https://freetsa.org/tsr`
   (qualified stamp first, FreeTSA as the second cross-checked token).
5. The court packet (`verify-web/report.py`) already carries the TSA tokens;
   with a QTSP token the report gains the eIDAS/5070 legal presumption.

## Checklist

- [ ] Mobil İmza activation (GSM operator, via e-Devlet)
- [ ] KamuSM e-Damga subscription (or an EU QTSP if EU-wide weight is wanted)
- [ ] TSA endpoint URL + credentials handed to the project
- [ ] Stamp a real manifest with the qualified TSA + cross-check vs FreeTSA
- [ ] Court packet regenerated with the qualified token
