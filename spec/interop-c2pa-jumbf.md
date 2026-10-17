# C2PA / .rise JUMBF interop study

Date: 2026-10-17. Tools: `c2patool` 0.28.2 (c2pa-rs 0.91.2), `rise-core` 0.2.5,
OpenSSL ed25519 test cert, ffmpeg 64x64 base image. Method: produce one C2PA-sealed
JPEG and one .rise-sealed JPEG from the SAME base asset, dump both segment tables,
then dual-seal the same file and validate with both readers.

## 1. Artifacts

| File | Size | Producer |
|---|---|---|
| test.jpg | 247 B | ffmpeg lavfi single frame (base for both) |
| c2pa.jpg | 33,579 B | c2patool, ed25519 self-signed cert, assertions: `c2pa.actions.v2`, `stds.schema-org.CreativeWork` — `validation_state: Valid` |
| rise.jpg | 3,675 B | `rise-core keygen/sign/pack` (ES256, `p256-soft`), unpack roundtrip OK |
| dual.jpg | 37,007 B | `rise-core pack` applied ON TOP of c2pa.jpg (both seals in one file) |

## 2. Segment layout (parsed directly, not via any vendor reader)

| Segment | c2pa.jpg | rise.jpg |
|---|---|---|
| APP11 (JUMBF carrier) | off=20 (after APP0), len=33,330 | off=2 (FIRST, before APP0), len=3,426 |
| APP0 (JFIF) | off=2 | off=3,430 |
| COM ("Lavc62.28.102") | yes | yes |
| DQT/DHT/SOF0/SOS | unchanged from base | unchanged from base |

## 3. APP11 framing is IDENTICAL (ISO 19566-5)

Both files start the APP11 payload with the same 10 bytes:

```
c2pa.jpg:  4a50 0211 00000001 00008228 6a756d62  ("JP", version/marker echo, seq=1, box len, "jumb")
rise.jpg:  4a50 0211 00000001 00000d58 6a756d62
```

A JUMBF-aware JPEG reader can walk into both superboxes with the same parser.
Difference is scale and content, not transport:
- C2PA superbox: 33,320 B — claim store, X.509 chain, COSE `c2pa.signature`,
  per-assertion hashes (`c2pa.hash.data` binds the source bytes), thumbnail.claim.
- .rise superbox: 3,416 B — one ManifestV1 JSON (tile-Merkle + frame SHA256 +
  ES256 signature), no certs by design (fail-closed trust lives in verify-web).

## 4. Dual-seal experiment (the load-bearing result)

`rise-core pack` was run on c2pa.jpg, producing dual.jpg with TWO independent
APP11 "JP" superboxes (`c2pa` + `rise`). Both readers then validated the SAME file:

- `c2patool dual.jpg` -> `validation_state: Valid`, active manifest
  `urn:c2pa:3cacee87-...` (c2pa_rs ignores the foreign .rise APP11 segment).
- `rise-core unpack dual.jpg` -> returns OUR ManifestV1 with
  `frame_hashes[0] = sha256(c2pa.jpg)` (the walker lands on the appended .rise
  box, NOT inside the 33 KB C2PA box).

Real-world confirmation of the fuzzed phantom-box property (REDTEAM F1): a
size-honoring walker coexists with a genuine foreign JUMBF superbox. The two
sealing formats are mutually transparent.

## 5. REA (v6.5.0) attempt — honest negative

`rea analyze` and `rea inspect-artifact` both reject still images
(`unsupported_target: unsupported binary format`) — REA's analyzers target
binaries, apps, and web bundles. Image interop work needs domain tools
(c2patool + a segment parser); this study used those.

## 6. Findings and follow-ups

1. Transport compatibility is proven, not assumed: same APP11 encapsulation,
   bidirectional coexistence on one file.
2. Segment position: .rise writes APP11 FIRST (before JFIF APP0); c2pa.rs writes
   it after APP0. Both are legal (APPn anywhere before SOS), but moving .rise's
   APP11 after APP0 would match the common JFIF layout — candidate tweak in
   `core/src/jumbf.rs` packer.
3. dual.jpg is ready-made interop KAT material: a committed fixture +
   `test_dual_seal` (unpack returns rise manifest, c2patool keeps Valid) would
   lock the coexistence property forever.
