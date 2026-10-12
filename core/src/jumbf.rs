//! JUMBF (ISO 19566-5) APP11 embedding — the container C2PA 2.4 uses for JPEG.
//!
//! `pack` seals a C2PA 2.4 manifest store so standard validators (c2patool)
//! can read it:
//!
//! ```text
//! jumb [UUID=c2pa, label="c2pa"]                      outer store
//! └─ jumb [UUID=c2ma, label="urn:c2pa:<guid>"]        manifest
//!    ├─ jumb [UUID=c2as, label="c2pa.assertions"]     assertion store
//!    │  ├─ jumb [UUID=cbor, label="c2pa.hash.data"]   hard binding (exclusions)
//!    │  ├─ jumb [UUID=cbor, label="c2pa.actions"]    c2pa.created
//!    │  ├─ jumb [UUID=json, label="stds.schema-org.CreativeWork"]  dc:title/creator
//!    │  └─ jumb [UUID=json, label="rise.manifest"]   verbatim RISE manifest
//!    ├─ jumb [UUID=c2cl, label="c2pa.claim"]         V1 CBOR claim
//!    └─ jumb [UUID=c2cs, label="c2pa.signature"]     COSE_Sign1 ES256 + x5chain
//! ```
//!
//! The claim signature uses an ephemeral P-256 key with a self-signed
//! device certificate in `x5chain`. Validators report the credential as
//! untrusted (it is not on the C2PA trust list) while the claim signature,
//! assertion hashes, and data hash all validate. RISE-level authenticity
//! still comes from the signed `rise.manifest` assertion inside.
//!
//! Segments use the ISO 19566-5 JPEG XT APP11 form (`CI="JP"` + `En` + `Z`)
//! that C2PA readers scan for. Legacy `JUMBF\0` segments and flat `json`
//! superboxes still *read* (old sealed files keep verifying) but are no
//! longer written.

use crate::RiseError;

/// C2PA manifest store type UUID (`c2pa`, JUMBF registry).
const C2PA_UUID: [u8; 16] = [
    0x63, 0x32, 0x70, 0x61, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// C2PA manifest box UUID (`c2ma`).
const C2MA_UUID: [u8; 16] = [
    0x63, 0x32, 0x6D, 0x61, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// C2PA claim box UUID (`c2cl`).
const C2CL_UUID: [u8; 16] = [
    0x63, 0x32, 0x63, 0x6C, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// C2PA claim signature box UUID (`c2cs`).
const C2CS_UUID: [u8; 16] = [
    0x63, 0x32, 0x63, 0x73, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// C2PA assertion store UUID (`c2as`).
const C2AS_UUID: [u8; 16] = [
    0x63, 0x32, 0x61, 0x73, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// JUMBF JSON content UUID (`json`).
const JSON_UUID: [u8; 16] = [
    0x6A, 0x73, 0x6F, 0x6E, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// JUMBF CBOR content UUID (`cbor`).
const CBOR_UUID: [u8; 16] = [
    0x63, 0x62, 0x6F, 0x72, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// APP11 segment payload prefix (legacy writer only; still read back).
const JUMBF_PREFIX: &[u8] = b"JUMBF\x00";

/// JPEG XT APP11 common identifier.
const XT_CI: &[u8] = b"JP";

/// Box instance number (`En`) for our APP11 segments (mirrors c2pa's pick).
const XT_EN: [u8; 2] = [0x02, 0x11];

/// Max store bytes per APP11 segment (mirrors c2pa's marker sizing).
const XT_CHUNK: usize = 64000;

/// Label of the RISE assertion carrying the verbatim manifest.
const RISE_ASSERTION_LABEL: &str = "rise.manifest";

fn be32(n: usize) -> [u8; 4] {
    (n as u32).to_be_bytes()
}

/// Wraps a payload in a length-prefixed BMFF box.
fn boxed(typ: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&be32(8 + payload.len()));
    out.extend_from_slice(typ);
    out.extend_from_slice(payload);
    out
}

/// JUMBF description box: len + "jumd" + UUID + toggles + label + NUL.
/// Toggles 0x03 = requestable + label present (what C2PA readers index on).
fn desc_box(uuid: &[u8; 16], label: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(16 + 1 + label.len() + 1);
    payload.extend_from_slice(uuid);
    payload.push(0x03);
    payload.extend_from_slice(label.as_bytes());
    payload.push(0x00);
    boxed(b"jumd", &payload)
}

/// JUMBF superbox: len + "jumb" + description box + content boxes.
fn jumb_box(uuid: &[u8; 16], label: &str, children: &[u8]) -> Vec<u8> {
    let desc = desc_box(uuid, label);
    let mut payload = Vec::with_capacity(desc.len() + children.len());
    payload.extend_from_slice(&desc);
    payload.extend_from_slice(children);
    boxed(b"jumb", &payload)
}

/// Builds a JUMBF superbox: [len][jumb][desc box][json box].
/// Legacy constructor (pre-C2PA flat layout); kept for reading old files.
pub fn build_superbox(json_payload: &[u8]) -> Vec<u8> {
    // description box: len + "jumd" + UUID + toggles
    let desc_len = 4 + 4 + 16 + 1;
    let mut desc = Vec::with_capacity(desc_len);
    desc.extend_from_slice(&be32(desc_len));
    desc.extend_from_slice(b"jumd");
    desc.extend_from_slice(&C2PA_UUID);
    desc.push(0x00); // toggles: no label/id/signature

    // json content box: len + "json" + payload
    let json_len = 4 + 4 + json_payload.len();
    let mut json_box = Vec::with_capacity(json_len);
    json_box.extend_from_slice(&be32(json_len));
    json_box.extend_from_slice(b"json");
    json_box.extend_from_slice(json_payload);

    // superbox: len + "jumb" + content
    let super_len = 4 + 4 + desc.len() + json_box.len();
    let mut out = Vec::with_capacity(super_len);
    out.extend_from_slice(&be32(super_len));
    out.extend_from_slice(b"jumb");
    out.extend_from_slice(&desc);
    out.extend_from_slice(&json_box);
    out
}

// ---------------------------------------------------------------------------
// Minimal CBOR writer (RFC 8949) — only what claim/COSE need, no new deps.
// ---------------------------------------------------------------------------

struct Cbor {
    buf: Vec<u8>,
}

impl Cbor {
    fn new() -> Self {
        Cbor { buf: Vec::new() }
    }

    fn head(&mut self, major: u8, n: u64) {
        let m = major << 5;
        if n < 24 {
            self.buf.push(m | n as u8);
        } else if n < 256 {
            self.buf.push(m | 24);
            self.buf.push(n as u8);
        } else if n < 65536 {
            self.buf.push(m | 25);
            self.buf.extend_from_slice(&(n as u16).to_be_bytes());
        } else if n < 0x1_0000_0000 {
            self.buf.push(m | 26);
            self.buf.extend_from_slice(&(n as u32).to_be_bytes());
        } else {
            self.buf.push(m | 27);
            self.buf.extend_from_slice(&n.to_be_bytes());
        }
    }

    fn uint(&mut self, n: u64) {
        self.head(0, n);
    }

    /// Negative int: encodes -1-n (so neg(6) == -7, the ES256 COSE alg).
    fn neg(&mut self, n: u64) {
        self.head(1, n);
    }

    fn bytes(&mut self, b: &[u8]) {
        self.head(2, b.len() as u64);
        self.buf.extend_from_slice(b);
    }

    fn text(&mut self, s: &str) {
        self.head(3, s.len() as u64);
        self.buf.extend_from_slice(s.as_bytes());
    }

    fn array(&mut self, n: usize) {
        self.head(4, n as u64);
    }

    fn map(&mut self, n: usize) {
        self.head(5, n as u64);
    }

    fn tag(&mut self, n: u64) {
        self.head(6, n);
    }
}

// ---------------------------------------------------------------------------
// Claim + assertions
// ---------------------------------------------------------------------------

/// Assertion reference for the claim's `assertions` list.
struct AssertionRef {
    label: String,
    hash: Vec<u8>,
}

/// Builds one assertion superbox and returns (box bytes, box-payload hash).
/// The hash covers the superbox payload (desc box + content box), which is
/// what C2PA validators recompute (`calc_assertion_box_hash`).
fn assertion_box(uuid: &[u8; 16], label: &str, content: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    use sha2::Digest as _;
    let desc = desc_box(uuid, label);
    let mut h = sha2::Sha256::new();
    h.update(&desc);
    h.update(&content);
    let hash = h.finalize().to_vec();
    let mut payload = Vec::with_capacity(desc.len() + content.len());
    payload.extend_from_slice(&desc);
    payload.extend_from_slice(&content);
    (boxed(b"jumb", &payload), hash)
}

/// V1 CBOR claim fields (C2PA §10.2). Field set is exactly the V1 vocabulary —
/// readers reject unknown V1 fields, so nothing extra goes here.
struct ClaimFields<'a> {
    generator: &'a str,
    generator_name: &'a str,
    generator_version: &'a str,
    sig_uri: &'a str,
    refs: &'a [AssertionRef],
    format: &'a str,
    title: &'a str,
    instance_id: &'a str,
}

fn claim_cbor(f: &ClaimFields) -> Vec<u8> {
    let mut c = Cbor::new();
    c.map(8);
    c.text("claim_generator");
    c.text(f.generator);
    c.text("claim_generator_info");
    c.array(1);
    c.map(2);
    c.text("name");
    c.text(f.generator_name);
    c.text("version");
    c.text(f.generator_version);
    c.text("signature");
    c.text(f.sig_uri);
    c.text("assertions");
    c.array(f.refs.len());
    for r in f.refs {
        c.map(3);
        c.text("url");
        c.text(&format!("self#jumbf=c2pa.assertions/{}", r.label));
        c.text("alg");
        c.text("sha256");
        c.text("hash");
        c.bytes(&r.hash);
    }
    c.text("dc:format");
    c.text(f.format);
    c.text("alg");
    c.text("sha256");
    c.text("dc:title");
    c.text(f.title);
    c.text("instanceID");
    c.text(f.instance_id);
    c.buf
}

// ---------------------------------------------------------------------------
// COSE_Sign1 (ES256) — protected {1: -7}, payload = claim CBOR.
// ---------------------------------------------------------------------------

/// CBOR-encoded protected header `{1: -7}` (ES256).
fn cose_protected() -> Vec<u8> {
    let mut c = Cbor::new();
    c.map(1);
    c.uint(1);
    c.neg(6); // -7
    c.buf
}

/// `Sig_structure = ["Signature1", protected, external_aad, payload]`.
fn cose_sig_structure(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut c = Cbor::new();
    c.array(4);
    c.text("Signature1");
    c.bytes(protected);
    c.bytes(b"");
    c.bytes(payload);
    c.buf
}

/// Full COSE_Sign1 object (CBOR tag 18) with the device cert in `x5chain`.
fn cose_sign1(
    key: &p256::ecdsa::SigningKey,
    cert_der: &[u8],
    claim: &[u8],
) -> Result<Vec<u8>, RiseError> {
    use p256::ecdsa::signature::Signer as _;
    let protected = cose_protected();
    let to_sign = cose_sig_structure(&protected, claim);
    let sig: p256::ecdsa::Signature = key.sign(&to_sign);
    let raw = sig.to_bytes();
    let mut c = Cbor::new();
    c.tag(18);
    c.array(4);
    c.bytes(&protected);
    c.map(1);
    c.text("x5chain");
    c.array(1);
    c.bytes(cert_der);
    c.bytes(claim);
    c.bytes(&raw);
    Ok(c.buf)
}

// ---------------------------------------------------------------------------
// Minimal DER writer + self-signed P-256 device certificate.
// ---------------------------------------------------------------------------

fn der_len(n: usize, out: &mut Vec<u8>) {
    if n < 128 {
        out.push(n as u8);
    } else if n < 256 {
        out.push(0x81);
        out.push(n as u8);
    } else if n < 65536 {
        out.push(0x82);
        out.extend_from_slice(&(n as u16).to_be_bytes());
    } else {
        out.push(0x83);
        out.push((n >> 16) as u8);
        out.extend_from_slice(&((n & 0xFFFF) as u16).to_be_bytes());
    }
}

fn der_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + content.len());
    out.push(tag);
    der_len(content.len(), &mut out);
    out.extend_from_slice(content);
    out
}

fn der_seq(content: &[u8]) -> Vec<u8> {
    der_tlv(0x30, content)
}

fn der_oid(body: &[u8]) -> Vec<u8> {
    der_tlv(0x06, body)
}

fn der_null() -> Vec<u8> {
    vec![0x05, 0x00]
}

fn der_integer(bytes: &[u8]) -> Vec<u8> {
    // strip leading zeroes, force positive
    let mut s = 0;
    while s + 1 < bytes.len() && bytes[s] == 0 {
        s += 1;
    }
    let body = &bytes[s..];
    if body[0] & 0x80 != 0 {
        let mut v = vec![0x00];
        v.extend_from_slice(body);
        der_tlv(0x02, &v)
    } else {
        der_tlv(0x02, body)
    }
}

fn der_utf8string(s: &str) -> Vec<u8> {
    der_tlv(0x0C, s.as_bytes())
}

fn der_utctime(s: &str) -> Vec<u8> {
    debug_assert_eq!(s.len(), 13);
    der_tlv(0x17, s.as_bytes())
}

fn der_bitstring(data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x00]; // no unused bits
    v.extend_from_slice(data);
    der_tlv(0x03, &v)
}

fn der_octetstring(data: &[u8]) -> Vec<u8> {
    der_tlv(0x04, data)
}

fn der_explicit(n: u8, data: &[u8]) -> Vec<u8> {
    der_tlv(0xA0 | n, data)
}

// OID bodies (without tag/length).
const OID_ECDSA_SHA256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02];
const OID_EC_PUBLIC_KEY: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
const OID_PRIME256V1: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07];
const OID_COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03];
const OID_AUTH_KEY_ID: &[u8] = &[0x55, 0x1D, 0x23];
const OID_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x0F];
const OID_EXT_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x25];
const OID_EMAIL_PROTECTION: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x04];

/// Unix seconds → `YYMMDDHHMMSSZ` (valid through 2049, past our cert window).
fn unix_to_utctime(secs: u64) -> String {
    // Howard Hinnant's civil-from-days algorithm.
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = (if m <= 2 { y + 1 } else { y } % 100) as u64;
    format!(
        "{:02}{:02}{:02}{:02}{:02}{:02}Z",
        year,
        m as u64,
        d as u64,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Self-signed end-entity certificate for the ephemeral claim key.
///
/// Extensions are the C2PA validator's minimum for a non-CA cert:
/// authorityKeyIdentifier, keyUsage=digitalSignature (critical), and
/// extendedKeyUsage=emailProtection. Without them validators log
/// `signingCredential.invalid` ("certificate params incorrect").
fn self_signed_cert_der(
    cn: &str,
    key: &p256::ecdsa::SigningKey,
    serial: &[u8; 8],
    not_before_secs: u64,
) -> Result<Vec<u8>, RiseError> {
    use p256::ecdsa::signature::Signer as _;
    use sha2::Digest as _;

    let vk = p256::ecdsa::VerifyingKey::from(key);
    let point = vk.to_sec1_point(false);
    let point_bytes = point.as_bytes(); // 65 bytes: 0x04 || X || Y

    // SKI/AKI fingerprint: first 20 bytes of SHA-256 over the point.
    let mut h = sha2::Sha256::new();
    h.update(point_bytes);
    let ski = h.finalize()[..20].to_vec();

    // RDN: CN=<device>
    let mut rdn_attr = Vec::new();
    rdn_attr.extend_from_slice(&der_oid(OID_COMMON_NAME));
    rdn_attr.extend_from_slice(&der_utf8string(cn));
    let mut rdn = Vec::new();
    rdn.extend_from_slice(&der_seq(&rdn_attr));
    let name = der_seq(&der_tlv(0x31, &rdn));

    // Signature algorithm: ecdsa-with-SHA256.
    let mut sig_alg = Vec::new();
    sig_alg.extend_from_slice(&der_oid(OID_ECDSA_SHA256));
    sig_alg.extend_from_slice(&der_null());
    let sig_alg = der_seq(&sig_alg);

    // SPKI: ecPublicKey + prime256v1 + uncompressed point.
    let mut spki_alg = Vec::new();
    spki_alg.extend_from_slice(&der_oid(OID_EC_PUBLIC_KEY));
    spki_alg.extend_from_slice(&der_oid(OID_PRIME256V1));
    let mut spki = Vec::new();
    spki.extend_from_slice(&der_seq(&spki_alg));
    spki.extend_from_slice(&der_bitstring(point_bytes));
    let spki = der_seq(&spki);

    // Validity: now → now + 90 days.
    let mut validity = Vec::new();
    validity.extend_from_slice(&der_utctime(&unix_to_utctime(not_before_secs)));
    validity.extend_from_slice(&der_utctime(&unix_to_utctime(not_before_secs + 90 * 86400)));
    let validity = der_seq(&validity);

    // Extensions: AKI + KeyUsage(digitalSignature, critical) + EKU(emailProtection).
    let mut aki_inner = vec![0x80];
    der_len(ski.len(), &mut aki_inner);
    aki_inner.extend_from_slice(&ski);
    let mut exts = Vec::new();
    let mut aki = Vec::new();
    aki.extend_from_slice(&der_oid(OID_AUTH_KEY_ID));
    aki.extend_from_slice(&der_octetstring(&der_seq(&aki_inner)));
    exts.extend_from_slice(&der_seq(&aki));
    let mut ku = Vec::new();
    ku.extend_from_slice(&der_oid(OID_KEY_USAGE));
    ku.push(0x01);
    ku.push(0x01);
    ku.push(0xFF); // critical = TRUE
    ku.extend_from_slice(&der_octetstring(&der_tlv(0x03, &[0x07, 0x80])));
    exts.extend_from_slice(&der_seq(&ku));
    let mut eku = Vec::new();
    eku.extend_from_slice(&der_oid(OID_EXT_KEY_USAGE));
    eku.extend_from_slice(&der_octetstring(&der_seq(&der_oid(OID_EMAIL_PROTECTION))));
    exts.extend_from_slice(&der_seq(&eku));
    let extensions = der_explicit(3, &der_seq(&exts));

    // TBS + signature.
    let mut tbs = Vec::new();
    tbs.extend_from_slice(&der_explicit(0, &der_integer(&[0x02])));
    tbs.extend_from_slice(&der_integer(serial));
    tbs.extend_from_slice(&sig_alg);
    tbs.extend_from_slice(&name);
    tbs.extend_from_slice(&validity);
    tbs.extend_from_slice(&name); // self-signed: subject == issuer
    tbs.extend_from_slice(&spki);
    tbs.extend_from_slice(&extensions);
    let tbs = der_seq(&tbs);

    let sig: p256::ecdsa::Signature = key.sign(&tbs);
    let mut cert = Vec::new();
    cert.extend_from_slice(&tbs);
    cert.extend_from_slice(&sig_alg);
    cert.extend_from_slice(&der_bitstring(&sig.to_bytes()));
    Ok(der_seq(&cert))
}

// ---------------------------------------------------------------------------
// Manifest store assembly
// ---------------------------------------------------------------------------

/// Claim generator string, e.g. "RISE rise-core/0.2.0".
fn claim_generator() -> String {
    format!("RISE rise-core/{}", env!("CARGO_PKG_VERSION"))
}

/// Pulls display metadata out of the RISE manifest; falls back when the
/// payload is not RISE JSON (legacy tests embed bare fragments).
fn seal_meta(manifest_json: &[u8]) -> (String, String) {
    let device = serde_json::from_slice::<serde_json::Value>(manifest_json)
        .ok()
        .and_then(|v| {
            v.get("device_id")
                .and_then(|d| d.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown-device".to_string());
    let title = format!("RISE sealed frame ({device})");
    (device, title)
}

fn random_hex(rng: &mut impl rand::TryRngCore, n: usize) -> Result<Vec<u8>, RiseError> {
    let mut b = vec![0u8; n];
    rng.try_fill_bytes(&mut b)
        .map_err(|e| RiseError::Crypto(format!("rng: {e}")))?;
    Ok(b)
}

/// Random v4 UUID string.
fn uuid4(rng: &mut impl rand::TryRngCore) -> Result<String, RiseError> {
    let b = random_hex(rng, 16)?;
    let mut hex = String::with_capacity(36);
    for (i, byte) in b.iter().enumerate() {
        let mut v = *byte;
        if i == 6 {
            v = (v & 0x0F) | 0x40;
        } // version 4
        if i == 8 {
            v = (v & 0x3F) | 0x80;
        } // variant 10
        if [4, 6, 8, 10].contains(&i) {
            hex.push('-');
        }
        hex.push(char::from_digit((v >> 4) as u32, 16).unwrap());
        hex.push(char::from_digit((v & 0x0F) as u32, 16).unwrap());
    }
    Ok(hex)
}

/// CBOR data-hash assertion (`c2pa.hash.data`): hard binding of the asset
/// with the manifest-store byte range excluded.
fn data_hash_cbor(exclusion_len: usize, hash: &[u8; 32]) -> Vec<u8> {
    let mut c = Cbor::new();
    c.map(5);
    c.text("exclusions");
    c.array(1);
    c.map(2);
    c.text("start");
    c.uint(2);
    c.text("length");
    c.uint(exclusion_len as u64);
    c.text("name");
    c.text("jumbf manifest");
    c.text("alg");
    c.text("sha256");
    c.text("hash");
    c.bytes(hash);
    c.text("pad");
    c.bytes(b"");
    c.buf
}

/// CBOR actions assertion: a creation claim.
fn actions_cbor() -> Vec<u8> {
    let mut c = Cbor::new();
    c.map(1);
    c.text("actions");
    c.array(1);
    c.map(1);
    c.text("action");
    c.text("c2pa.created");
    c.buf
}

/// JSON CreativeWork assertion with Dublin Core title/creator.
fn creative_work_json(title: &str, creator: &str) -> Vec<u8> {
    serde_json::json!({
        "@type": "CreativeWork",
        "dc:title": title,
        "dc:creator": creator,
    })
    .to_string()
    .into_bytes()
}

/// Assembles the full C2PA manifest store for the given RISE manifest.
/// `data_hash`/`exclusion_len` bind the final asset bytes (two-pass fill).
fn build_store(
    manifest_json: &[u8],
    manifest_label: &str,
    instance_id: &str,
    key: &p256::ecdsa::SigningKey,
    cert_der: &[u8],
    data_hash: &[u8; 32],
    exclusion_len: usize,
) -> Result<Vec<u8>, RiseError> {
    let (device, title) = seal_meta(manifest_json);
    let generator = claim_generator();

    // Assertion boxes (hashes cover each superbox payload).
    let mut refs = Vec::new();
    let mut children = Vec::new();
    let assertions: Vec<(&[u8; 16], &str, Vec<u8>)> = vec![
        (
            &CBOR_UUID,
            "c2pa.hash.data",
            boxed(b"cbor", &data_hash_cbor(exclusion_len, data_hash)),
        ),
        (&CBOR_UUID, "c2pa.actions", boxed(b"cbor", &actions_cbor())),
        (
            &JSON_UUID,
            "stds.schema-org.CreativeWork",
            boxed(b"json", &creative_work_json(&title, &device)),
        ),
        (
            &JSON_UUID,
            RISE_ASSERTION_LABEL,
            boxed(b"json", manifest_json),
        ),
    ];
    for (uuid, label, content) in &assertions {
        let (abox, hash) = assertion_box(uuid, label, content.clone());
        children.extend_from_slice(&abox);
        refs.push(AssertionRef {
            label: label.to_string(),
            hash,
        });
    }
    let assertion_store = jumb_box(&C2AS_UUID, "c2pa.assertions", &children);

    // Claim + COSE signature.
    let sig_uri = format!("self#jumbf=/c2pa/{manifest_label}/c2pa.signature");
    let claim = claim_cbor(&ClaimFields {
        generator: &generator,
        generator_name: "rise-core",
        generator_version: env!("CARGO_PKG_VERSION"),
        sig_uri: &sig_uri,
        refs: &refs,
        format: "image/jpeg",
        title: &title,
        instance_id,
    });
    let claim_box = jumb_box(&C2CL_UUID, "c2pa.claim", &boxed(b"cbor", &claim));
    let cose = cose_sign1(key, cert_der, &claim)?;
    let sig_box = jumb_box(&C2CS_UUID, "c2pa.signature", &boxed(b"cbor", &cose));

    let mut manifest_children = Vec::new();
    manifest_children.extend_from_slice(&assertion_store);
    manifest_children.extend_from_slice(&claim_box);
    manifest_children.extend_from_slice(&sig_box);
    let manifest = jumb_box(&C2MA_UUID, manifest_label, &manifest_children);
    Ok(jumb_box(&C2PA_UUID, "c2pa", &manifest))
}

/// Embeds the store with ISO 19566-5 JPEG XT APP11 segments (`CI En Z`).
/// Continuation segments repeat the store's LBOX+TBOX before their chunk.
fn xt_embed_with_chunk(jpg: &[u8], store: &[u8], chunk: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(jpg.len() + store.len() + 64);
    out.extend_from_slice(&jpg[..2]); // SOI
    for (i, piece) in store.chunks(chunk).enumerate() {
        out.extend_from_slice(&[0xFF, 0xEB]);
        let mut payload = Vec::with_capacity(8 + 8 + piece.len());
        payload.extend_from_slice(XT_CI);
        payload.extend_from_slice(&XT_EN);
        payload.extend_from_slice(&(i as u32 + 1).to_be_bytes());
        if i > 0 {
            // duplicate LBOX+TBOX per ISO 19566-5
            payload.extend_from_slice(&store[..8.min(store.len())]);
        }
        payload.extend_from_slice(piece);
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(&payload);
    }
    out.extend_from_slice(&jpg[2..]);
    out
}

fn xt_embed(jpg: &[u8], store: &[u8]) -> Vec<u8> {
    xt_embed_with_chunk(jpg, store, XT_CHUNK)
}

/// Byte range + SHA-256 of our CAI APP11 region in an embedded file.
/// The region is the run of APP11 segments right after SOI; the hash
/// covers everything else (the data-hash exclusion rule).
fn cai_region_hash(file: &[u8]) -> (usize, [u8; 32]) {
    use sha2::Digest as _;
    let mut len = 0usize;
    let mut pos = 2usize;
    let mut first = true;
    while pos + 4 <= file.len() && file[pos] == 0xFF && file[pos + 1] == 0xEB {
        let seg_len = u16::from_be_bytes([file[pos + 2], file[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > file.len() {
            break;
        }
        let content = &file[pos + 4..pos + 2 + seg_len];
        if content.len() < 28 || content[0..2] != *XT_CI {
            break;
        }
        if first {
            if content[24..28] != *b"c2pa" {
                break;
            }
            first = false;
        }
        len += 2 + seg_len;
        pos += 2 + seg_len;
    }
    let mut h = sha2::Sha256::new();
    h.update(&file[..2]);
    h.update(&file[2 + len..]);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&h.finalize());
    (len, hash)
}

/// Current Unix time in seconds.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Seals the RISE manifest into the JPEG as a C2PA 2.4 manifest store.
/// Two passes fill the data-hash exclusion (sizes are deterministic, so
/// the hash/length fixpoint converges immediately).
pub fn seal_jpeg(jpg: &[u8], manifest_json: &[u8]) -> Result<Vec<u8>, RiseError> {
    use rand::TryRngCore as _;
    if jpg.len() < 2 || jpg[0] != 0xFF || jpg[1] != 0xD8 {
        return Err(RiseError::RootMismatch);
    }
    // Ephemeral claim key + self-signed device certificate.
    let mut rng = rand::rngs::OsRng;
    let mut key_bytes = [0u8; 32];
    rng.try_fill_bytes(&mut key_bytes)
        .map_err(|e| RiseError::Crypto(format!("rng: {e}")))?;
    let key = p256::ecdsa::SigningKey::from_bytes(&key_bytes.into())
        .map_err(|e| RiseError::Crypto(format!("keygen: {e}")))?;
    let mut serial = [0u8; 8];
    rng.try_fill_bytes(&mut serial)
        .map_err(|e| RiseError::Crypto(format!("rng: {e}")))?;
    serial[0] &= 0x7F; // force positive INTEGER, fixed width
    let (device, _) = seal_meta(manifest_json);
    let cert = self_signed_cert_der(&format!("RISE device {device}"), &key, &serial, now_secs())?;
    let manifest_label = format!("urn:c2pa:{}", uuid4(&mut rng)?);
    let instance_id = format!("xmp:iid:{}", uuid4(&mut rng)?);

    let mut hash = [0u8; 32];
    let mut excl = 0usize;
    let mut out = Vec::new();
    for _ in 0..5 {
        let store = build_store(
            manifest_json,
            &manifest_label,
            &instance_id,
            &key,
            &cert,
            &hash,
            excl,
        )?;
        out = xt_embed(jpg, &store);
        let (l2, h2) = cai_region_hash(&out);
        if l2 == excl && h2 == hash {
            break;
        }
        excl = l2;
        hash = h2;
    }
    Ok(out)
}

/// Embeds the JUMBF payload into a JPEG as APP11 segment(s), right after SOI.
/// Now seals a full C2PA 2.4 manifest store (XT form); the payload rides
/// verbatim in the `rise.manifest` assertion so `unpack` output is unchanged.
pub fn jpeg_embed(jpg: &[u8], json_payload: &[u8]) -> Result<Vec<u8>, RiseError> {
    seal_jpeg(jpg, json_payload)
}

// ---------------------------------------------------------------------------
// Extraction (reads both the C2PA XT form and legacy JUMBF\0 segments)
// ---------------------------------------------------------------------------

/// Reads one length-prefixed box header; returns (total_len, type).
fn box_head(data: &[u8]) -> Option<(usize, [u8; 4])> {
    if data.len() < 8 {
        return None;
    }
    let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if len < 8 || len > data.len() {
        return None;
    }
    Some((len, [data[4], data[5], data[6], data[7]]))
}

/// Parses a JUMBF description-box payload; returns (uuid, label).
fn desc_info(payload: &[u8]) -> Option<([u8; 16], String)> {
    if payload.len() < 17 {
        return None;
    }
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&payload[..16]);
    let tog = payload[16];
    let label = if tog & 0x01 != 0 {
        let end = payload[17..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| 17 + p)
            .unwrap_or(payload.len());
        String::from_utf8_lossy(&payload[17..end]).into_owned()
    } else {
        String::new()
    };
    Some((uuid, label))
}

/// Recursively finds the `rise.manifest` assertion's json payload.
fn find_rise_manifest(data: &[u8], depth: usize) -> Option<Vec<u8>> {
    if depth > 8 {
        return None;
    }
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let (blen, btype) = box_head(&data[pos..])?;
        let payload = &data[pos + 8..pos + blen];
        if &btype == b"jumb" && payload.len() >= 8 {
            // first child is the description box
            let (dlen, dtype) = box_head(payload)?;
            if &dtype == b"jumd" {
                let label = desc_info(&payload[8..dlen])
                    .map(|(_, l)| l)
                    .unwrap_or_default();
                let children = &payload[dlen..];
                if label == RISE_ASSERTION_LABEL {
                    // direct json child carries the manifest
                    let mut cpos = 0usize;
                    while cpos + 8 <= children.len() {
                        let (clen, ctype) = box_head(&children[cpos..])?;
                        if &ctype == b"json" {
                            return Some(children[cpos + 8..cpos + clen].to_vec());
                        }
                        cpos += clen;
                    }
                    return None;
                }
                if let Some(hit) = find_rise_manifest(children, depth + 1) {
                    return Some(hit);
                }
            }
        }
        pos += blen;
    }
    None
}

/// Extracts the json box payload from JUMBF superbox bytes.
/// Reads the legacy flat layout directly, and the C2PA store by finding
/// the `rise.manifest` assertion.
pub fn parse_json_payload(superbox: &[u8]) -> Result<Vec<u8>, RiseError> {
    // header: len + "jumb"
    if superbox.len() < 8 || &superbox[4..8] != b"jumb" {
        return Err(RiseError::Json(
            serde_json::from_str::<serde_json::Value>("").unwrap_err(),
        ));
    }
    let mut pos = 8usize;
    // walk content boxes (legacy flat layout)
    while pos + 8 <= superbox.len() {
        let blen = u32::from_be_bytes([
            superbox[pos],
            superbox[pos + 1],
            superbox[pos + 2],
            superbox[pos + 3],
        ]) as usize;
        if blen < 8 || pos + blen > superbox.len() {
            break;
        }
        let btype = &superbox[pos + 4..pos + 8];
        if btype == b"json" {
            return Ok(superbox[pos + 8..pos + blen].to_vec());
        }
        pos += blen;
    }
    // C2PA store layout: recurse into the manifest for rise.manifest
    if let Some(hit) = find_rise_manifest(&superbox[8..], 0) {
        return Ok(hit);
    }
    Err(RiseError::RootMismatch)
}

/// Extracts the concatenated JUMBF superbox from all APP11 segments.
/// Reads ISO 19566-5 XT segments (`JP` + En + Z, LBOX+TBOX repeated on
/// continuations) and legacy `JUMBF\0` segments.
pub fn jpeg_extract(jpg: &[u8]) -> Option<Vec<u8>> {
    // not a JPEG
    if jpg.len() < 2 || jpg[0] != 0xFF || jpg[1] != 0xD8 {
        return None;
    }
    let mut legacy: Vec<u8> = Vec::new();
    // En -> list of (Z, segment index, data-after-header)
    type SegGroups = Vec<(Vec<u8>, Vec<(u32, usize, Vec<u8>)>)>;
    let mut groups: SegGroups = Vec::new();
    let mut pos = 2usize;
    // walk segments
    while pos + 4 <= jpg.len() {
        if jpg[pos] != 0xFF {
            break;
        }
        let marker = jpg[pos + 1];
        // SOS: entropy-coded data follows, stop scanning
        if marker == 0xDA {
            break;
        }
        let seg_len = u16::from_be_bytes([jpg[pos + 2], jpg[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > jpg.len() {
            break;
        }
        if marker == 0xEB {
            // APP11: legacy prefix or XT common header
            let payload = &jpg[pos + 4..pos + 2 + seg_len];
            if payload.starts_with(JUMBF_PREFIX) {
                legacy.extend_from_slice(&payload[6..]);
            } else if payload.len() > 28 && payload[0..2] == *XT_CI {
                let en = payload[2..4].to_vec();
                let z = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                let data = payload[8..].to_vec();
                match groups.iter_mut().find(|(e, _)| *e == en) {
                    Some((_, segs)) => segs.push((z, pos, data)),
                    None => groups.push((en, vec![(z, pos, data)])),
                }
            }
        }
        pos += 2 + seg_len;
    }
    // prefer the CAI store (XT group whose first segment carries c2pa)
    let mut groups_by_pos = groups;
    groups_by_pos
        .sort_by_key(|(_, segs)| segs.iter().map(|(_, p, _)| *p).min().unwrap_or(usize::MAX));
    for (_, mut segs) in groups_by_pos {
        segs.sort_by_key(|(z, p, _)| (*z, *p));
        if segs
            .first()
            .map(|(_, _, d)| d.len() > 20 && d[16..20] == *b"c2pa")
            .unwrap_or(false)
        {
            let mut store = Vec::new();
            for (i, (_, _, d)) in segs.iter().enumerate() {
                if i == 0 {
                    store.extend_from_slice(d);
                } else if d.len() >= 8 {
                    // strip the repeated LBOX+TBOX
                    store.extend_from_slice(&d[8..]);
                }
            }
            if !store.is_empty() {
                return Some(store);
            }
        }
    }
    if legacy.is_empty() {
        None
    } else {
        Some(legacy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superbox_carries_c2pa_uuid() {
        let sb = build_superbox(b"{}");
        assert_eq!(&sb[4..8], b"jumb");
        // c2pa UUID sits inside the description box (8+4+4 offset)
        assert!(sb[16..32].iter().zip(C2PA_UUID.iter()).all(|(a, b)| a == b));
    }

    #[test]
    fn json_payload_roundtrip() {
        let payload = br#"{"rise_version":1}"#;
        let sb = build_superbox(payload);
        assert_eq!(parse_json_payload(&sb).unwrap(), payload.to_vec());
    }

    #[test]
    fn jpeg_embed_extract_roundtrip() {
        // minimal JPEG: SOI + APP0-ish + SOS-ish tail
        let jpg = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22,
        ];
        let payload = br#"{"merkle_root":"aa"}"#;
        let embedded = jpeg_embed(&jpg, payload).unwrap();
        // APP11 marker present right after SOI
        assert_eq!(&embedded[2..4], &[0xFF, 0xEB]);
        let extracted = jpeg_extract(&embedded).unwrap();
        assert_eq!(parse_json_payload(&extracted).unwrap(), payload.to_vec());
        // the original image bytes survive after the embedded segments
        assert!(embedded.ends_with(&jpg[2..]));
    }

    #[test]
    fn extract_unmarked_returns_none() {
        let jpg = vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02];
        assert!(jpeg_extract(&jpg).is_none());
        // not a JPEG at all
        assert!(jpeg_extract(b"notjpeg").is_none());
    }

    #[test]
    fn legacy_superbox_still_reads() {
        // old sealed files (flat json superbox in JUMBF\0 segments) keep verifying
        let payload = br#"{"rise_version":1,"device_id":"old"}"#;
        let sb = build_superbox(payload);
        let jpg = [0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22];
        let mut out = Vec::from(&jpg[..2]);
        for chunk in sb.chunks(60000) {
            out.extend_from_slice(&[0xFF, 0xEB]);
            let seg_len = 2 + 6 + chunk.len();
            out.extend_from_slice(&(seg_len as u16).to_be_bytes());
            out.extend_from_slice(JUMBF_PREFIX);
            out.extend_from_slice(chunk);
        }
        out.extend_from_slice(&jpg[2..]);
        let extracted = jpeg_extract(&out).unwrap();
        assert_eq!(parse_json_payload(&extracted).unwrap(), payload.to_vec());
    }

    #[test]
    fn cbor_writer_vectors() {
        // map(1) {"a": 1}
        let mut c = Cbor::new();
        c.map(1);
        c.text("a");
        c.uint(1);
        assert_eq!(c.buf, vec![0xA1, 0x61, 0x61, 0x01]);
        // negative: -7
        let mut c = Cbor::new();
        c.neg(6);
        assert_eq!(c.buf, vec![0x26]);
        // 32-byte bstr head
        let mut c = Cbor::new();
        c.bytes(&[0u8; 32]);
        assert_eq!(&c.buf[..2], &[0x58, 0x20]);
        // tag 18
        let mut c = Cbor::new();
        c.tag(18);
        assert_eq!(c.buf, vec![0xD2]);
    }

    #[test]
    fn unix_to_utctime_vectors() {
        assert_eq!(unix_to_utctime(0), "700101000000Z");
        assert_eq!(unix_to_utctime(1760150400), "251011024000Z");
    }

    #[test]
    fn seal_has_c2pa_layout() {
        let jpg = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22,
        ];
        let payload = br#"{"rise_version":1,"device_id":"test-cam"}"#;
        let sealed = seal_jpeg(&jpg, payload).unwrap();
        // first APP11 uses the XT common header with the c2pa marker
        assert_eq!(&sealed[2..4], &[0xFF, 0xEB]);
        let seg_len = u16::from_be_bytes([sealed[4], sealed[5]]) as usize;
        let content = &sealed[6..6 + seg_len - 2];
        assert_eq!(&content[0..2], b"JP");
        assert_eq!(&content[24..28], b"c2pa");
        // store parses: outer label "c2pa", manifest label urn:c2pa:
        let store = jpeg_extract(&sealed).unwrap();
        let (_, outer_label) = desc_info(&store[16..]).unwrap();
        assert_eq!(outer_label, "c2pa");
        // rise.manifest assertion round-trips verbatim
        assert_eq!(parse_json_payload(&store).unwrap(), payload.to_vec());
    }

    #[test]
    fn cose_signature_self_verifies() {
        use p256::ecdsa::signature::Verifier as _;
        let mut key_bytes = [7u8; 32];
        key_bytes[0] &= 0x7F;
        let key = p256::ecdsa::SigningKey::from_bytes(&key_bytes.into()).unwrap();
        let claim = b"test-claim-cbor";
        let cose = cose_sign1(&key, b"fakecert", claim).unwrap();
        // tag 18 + array(4)
        assert_eq!(&cose[..2], &[0xD2, 0x84]);
        // verify the ECDSA signature over the Sig_structure
        let vk = p256::ecdsa::VerifyingKey::from(&key);
        let to_sign = cose_sig_structure(&cose_protected(), claim);
        let sig = p256::ecdsa::Signature::from_slice(&cose[cose.len() - 64..]).unwrap();
        vk.verify(&to_sign, &sig).unwrap();
    }

    #[test]
    fn cert_signature_self_verifies() {
        use p256::ecdsa::signature::Verifier as _;
        let mut key_bytes = [9u8; 32];
        key_bytes[0] &= 0x7F;
        let key = p256::ecdsa::SigningKey::from_bytes(&key_bytes.into()).unwrap();
        let serial = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let der = self_signed_cert_der("RISE device t", &key, &serial, 1760150400).unwrap();
        // SEQUENCE tag
        assert_eq!(der[0], 0x30);
        // CN present
        assert!(der.windows(13).any(|w| w == b"RISE device t"));
        // EKU emailProtection OID present
        assert!(der.windows(8).any(|w| w == OID_EMAIL_PROTECTION));
        // trailing BIT STRING is the 65-byte ECDSA signature; verify over TBS
        // (the first child element inside the outer Certificate SEQUENCE).
        let (_, outer_hdr) = der_length(&der[1..]);
        let inner = &der[1 + outer_hdr..];
        let (_, tbs_total) = tbs_extent(inner);
        let tbs = &inner[..tbs_total];
        assert_eq!(tbs[0], 0x30);
        let vk = p256::ecdsa::VerifyingKey::from(&key);
        let sig = p256::ecdsa::Signature::from_slice(&der[der.len() - 64..]).unwrap();
        vk.verify(tbs, &sig).unwrap();
    }

    /// Byte extent of the first DER element (the TBS Certificate).
    fn tbs_extent(der: &[u8]) -> (usize, usize) {
        assert_eq!(der[0], 0x30);
        let (len, hdr) = der_length(&der[1..]);
        (len, 1 + hdr + len)
    }

    fn der_length(b: &[u8]) -> (usize, usize) {
        if b[0] < 128 {
            (b[0] as usize, 1)
        } else if b[0] == 0x81 {
            (b[1] as usize, 2)
        } else if b[0] == 0x82 {
            (u16::from_be_bytes([b[1], b[2]]) as usize, 3)
        } else {
            panic!("long DER length");
        }
    }

    #[test]
    fn multisegment_reassembly() {
        // force many XT segments with a tiny chunk size
        let jpg = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22,
        ];
        let payload = br#"{"rise_version":1,"device_id":"chunk-test"}"#;
        let mut rng = rand::rngs::OsRng;
        let store = build_store(
            payload,
            "urn:c2pa:11111111-2222-4333-8444-555555555555",
            "xmp:iid:aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
            &ephemeral_key(&mut rng),
            b"cert",
            &[0u8; 32],
            0,
        )
        .unwrap();
        assert!(store.len() > 300);
        let embedded = xt_embed_with_chunk(&jpg, &store, 100);
        // several APP11 segments were written
        let count = embedded.windows(2).filter(|w| *w == [0xFF, 0xEB]).count();
        assert!(count > 3);
        let back = jpeg_extract(&embedded).unwrap();
        assert_eq!(back, store);
        assert_eq!(parse_json_payload(&back).unwrap(), payload.to_vec());
    }

    #[cfg(test)]
    fn ephemeral_key(rng: &mut impl rand::TryRngCore) -> p256::ecdsa::SigningKey {
        let mut b = [0u8; 32];
        rng.try_fill_bytes(&mut b).unwrap();
        b[0] &= 0x7F;
        // retry loop is overkill for tests; low bytes are virtually always valid
        p256::ecdsa::SigningKey::from_bytes(&b.into()).unwrap_or_else(|_| {
            b[31] = b[31].wrapping_add(1).max(1);
            p256::ecdsa::SigningKey::from_bytes(&b.into()).unwrap()
        })
    }
}
