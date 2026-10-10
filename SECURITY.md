# Security Policy

## Scope

Security-sensitive code in this repository:

- `core/src/signer.rs` — frame sealing / signature logic
- `core/src/timestamp.rs` — timestamp issuance and validation

Vulnerabilities in signature verification, timestamp forgery, or trust-badge
demotion logic are in scope. Platform bindings (`bindings/`) and the demo
verifier UI (`verify-web/`) are best-effort only.

## Reporting a Vulnerability

Please report security issues via
[GitHub private vulnerability reporting](../../security/advisories/new)
for this repository. Do not open a public issue for a suspected vulnerability.

Include: affected version, steps to reproduce, and impact assessment.
We will acknowledge receipt and keep you updated on the fix.

## Bug Bounty

.rise currently offers **no bug bounty program**. Reports are welcome and
credited, but there is no monetary reward.

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x:                |
