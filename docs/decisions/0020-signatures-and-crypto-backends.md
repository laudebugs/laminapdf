# 0020 — Signatures, crypto backends and key storage per platform

- Status: Accepted (owner sign-off 2026-11-25, including §5 revocation and §11 QES)
- Date: 2026-11-25

## Context

[ADR 0007](0007-save-planner.md) fixed how a signed document is saved: signing is two-phase, runs after any rewrite, and every later save appends. It left open who builds the signature, which algorithms and PAdES levels lamina writes, where private keys live on each platform, and how lamina decides whether a signed file is still trustworthy. Round E10 ([full-editor §E10](../plans/full-editor.md#e10-signatures)) needs those answers before any code.

Three facts shape the answer. First, a signature is a CMS `SignedData` over the file's `/ByteRange`, and PAdES (ETSI EN 319 142-1) adds attributes, timestamps and a Document Security Store on top ([research ch. 04 §8](../research/pdf/04-interactive-security.md#8-digital-signatures-128)). Second, the file's bytes never leave the user's machine ([ADR 0011](0011-cloudflare-topology-and-privacy.md)), yet timestamp authorities (TSAs) and revocation servers sit on the network, and most of them send no CORS headers. Third, validation is the harder half: the published attacks (universal signature forgery, incremental saving, signature wrapping, shadow and certification attacks) all pass a validator that checks only the CMS ([ch. 04 §8.8](../research/pdf/04-interactive-security.md#88-attacks-on-signatures-ruhr-universität-bochum-pdf-insecurity-research)).

We surveyed three implementations. pyHanko (Python, MIT) gives the model for the signer interface, placeholder sizing, DSS writing and the revision-diff validator (`DiffPolicy`). Apache PDFBox (Java) signs through a callback that receives the byte stream, which fits two-phase external signing but leaves validation to other libraries. The EU DSS library (Java, LGPL) is the reference validator for ETSI EN 319 102-1, and we use it as an oracle for results, never as a dependency. None is usable from Rust or WebAssembly, so lamina builds its own on RustCrypto and borrows the structure of pyHanko.

## Decision

### 1. One new core crate, `laminapdf-sign`, and edge adapters

`laminapdf-sign` is a **core** crate (bytes in, bytes out; no network, no clock, no filesystem, no OS keystore; builds for `wasm32`). It owns:

- **Build**: the signature dictionary and widget, the `/Contents` placeholder, `/ByteRange`, the digest, and the CMS `SignedData` (CAdES attributes).
- **Validate**: structural checks, CMS verification, chain building against supplied anchors, the revision diff, and the result model of §8.
- **Time and revocation encoding**: building an RFC 3161 `TimeStampReq`, parsing and verifying a `TimeStampResp`, parsing OCSP and CRL responses, and writing the DSS and document timestamps. Fetching is never here.

It depends on `laminapdf-cos`, `-model` and `-save`; the façade re-exports it behind a `sign` feature. Everything that touches the world sits in edge code: key stores and PKCS#11 in `laminapdf-desktop-os`, WebCrypto and file-based PKCS#12 in `@laminapdf/sdk`, and HTTP for TSAs, OCSP and CRLs in the edge that hosts the call (desktop engine, the web app's fetch, the CLI). [E10i](../todos/ready/E10i-platform-signers.md) places each adapter exactly.

### 2. Formats and levels

- **Default output:** `/SubFilter /ETSI.CAdES.detached`, SHA-256, signing-certificate-v2 and message-digest attributes, `/M` as the claimed time. That combination is PAdES **B-B**.
- **B-T** when a TSA is configured: an RFC 3161 token on the signature value as an unsigned attribute. A **document timestamp** (`/Type /DocTimeStamp`, `ETSI.RFC3161`) is a separate operation.
- **B-LT and B-LTA** (the DSS and a covering document timestamp) are not built in E10. The DSS writer and the OCSP and CRL parsers land with the validator (E10g) because validation needs them; the signing flow for B-LT follows as its own todo when the owner asks for it. The data model reserves the fields so no format change is needed.
- `adbe.pkcs7.detached` is read, validated and, on request, written, for viewers that lack PAdES support. `adbe.x509.rsa_sha1` and `adbe.pkcs7.sha1` are read and reported as deprecated; lamina never writes them.
- **Algorithms:** digests SHA-256 (default), SHA-384, SHA-512; SHA-1 is validated only, and reported as weak. Signatures: RSA PKCS#1 v1.5 and RSA-PSS, ECDSA P-256 and P-384, and Ed25519 (ISO/TS 32002). The validator accepts the same set and reports anything else as `unsupported algorithm`, never as valid.

### 3. The `Signer` trait is two-phase and does no I/O of its own

```rust
pub trait Signer {
    fn certificates(&self) -> &[Certificate];          // signer first, then the chain it holds
    fn algorithm(&self) -> SignatureAlgorithm;
    /// Signs `to_be_signed`, the DER of the signed attributes. Local keys answer at once; others may take a user prompt.
    fn sign(&self, to_be_signed: &[u8]) -> Result<Vec<u8>, SignError>;
}
```

The planner's two-phase flow ([ADR 0007 §4](0007-save-planner.md#4-canonical-order)) is the primary API, so a signer that cannot answer in one call still works:

1. `prepare(doc, params) -> PreparedSignature` assembles the revision in memory, computes the digest over the base and that revision, and returns the DER to be signed.
2. Anyone (a smart card, WebCrypto, a server, another person's machine) turns that DER into a signature value.
3. `finish(prepared, signature, timestamp_token?) -> Revision` builds the CMS, injects it into `/Contents` and returns the bytes to append.

`Signer` is the synchronous convenience over those steps (local keys, PKCS#12, test keys). Async signers (WebCrypto, OS keychains, PKCS#11 with a PIN dialog, remote services) call the three steps themselves in the edge, where an executor exists. Only the signed attributes and a hash cross a boundary, never the document: this keeps external and remote signing (`F.11`, signing workflows through external services) possible without a new design.

### 4. Placeholder sizing

`/Contents` is a fixed-length hex string, so the CMS must fit. `prepare` sizes it from what it knows (the certificate chain the signer holds, the algorithm, the attributes, and 6 KiB for a timestamp token when a TSA is set), rounds up to 1 KiB, and `finish` fails with `SignError::PlaceholderTooSmall { needed }` rather than truncating. Because `prepare` has not written anything yet, the caller retries with the reported size and pays one more digest, not a second save. Unused bytes are zero-padded; the validator reads the CMS by its own DER length, so padding never counts.

### 5. RFC 3161 split and the TSA proxy

Core builds the request (SHA-256 or stronger hash, nonce, `certReq` true) and verifies the response: status, the imprint equals our hash, the nonce, the TSA certificate's `id-kp-timeStamping` extended key usage, and the signature. The **transport** is the edge's job.

Browsers cannot call most TSAs (no CORS), so the web reader reaches them through an optional **TSA proxy Worker**. Its contract:

- `POST /tsa` with `{ "tsa": "<allow-listed id>", "request": "<base64 DER TimeStampReq>" }`. The request holds a hash and a nonce, so the Worker never sees a document.
- It forwards the DER to the TSA's URL, found in a static allow-list inside the Worker (no URL from the client), and returns the DER response unchanged with `content-type: application/timestamp-reply`.
- It logs nothing from the body: no hash, no nonce, no TSA response. Logs hold the allow-list id, the status code and the latency, in line with ADR 0011's rule that no request carries document content.
- Limits: request bodies over 4 KiB are refused; each client gets a modest rate limit; TSAs on the list must publish their policy and terms, and the list ships in the repository so a change is a reviewed PR.
- The desktop app and the CLI call the TSA directly and never use the Worker.

Revocation material (OCSP, CRL, issuer certificates found through AIA) is fetched at the edge the same way: a direct call on desktop and the CLI, and a second Worker route on the web.

That route (`/revocation`) cannot use an allow-list, because the URLs come from each certificate and no list of certificate authorities stays complete. It takes only URLs read from a certificate in the file, and it applies the SSRF defences ADR 0011 requires: `http` and `https` only, no private or link-local addresses, no redirects off the original host, and limits on body size and time. It carries the rate limit of the TSA route.

The web reader does not fetch revocation by default, because every fetch tells a third party which certificate the user checked. A "Check revocation" button on each signature makes the fetch on request, and the result reads "revocation not checked" until then. Revocation data already inside the file (B-LT) is always used. Desktop and CLI fetch by default.

### 6. Key storage per platform

| Platform | Keys | Notes |
| --- | --- | --- |
| Desktop (macOS) | Keychain through Security.framework, including Secure Enclave keys (ECDSA P-256) | The OS prompts; lamina never sees the private key |
| Desktop (Windows) | Certificate store and CNG, including smart cards through the OS | Same |
| Desktop (Linux) | PKCS#11 modules (smart cards, HSMs, tokens) that the user points at; PKCS#12 files | No single OS store, so PKCS#11 is the path |
| Desktop (all) | PKCS#11 as a second route; PKCS#12 import for a one-off file | PIN entry happens in lamina's UI, held in memory for the call |
| Web | WebCrypto non-extractable keys the user generates (kept in IndexedDB, the owner of the browser profile controls them); PKCS#12 imported in the page and used from memory for that session | A self-signed or imported certificate; no OS store exists |
| External | Any service or device that answers the prepare step of §3 | `F.11`, signing through external services |

PKCS#12 parsing uses RustCrypto's `pkcs12` and `pkcs8` crates and supports modern PBES2 only; the legacy RC2 and 3DES containers are refused with a message that says how to re-export. A private key never goes into a log, an error, a crash report ([ADR 0026](0026-error-and-diagnostic-contract.md)) or telemetry, and `Zeroize` wraps every in-memory copy.

### 7. Crypto crates

lamina writes no primitives. The core uses the RustCrypto family, each `MIT OR Apache-2.0` (or `Apache-2.0 OR MIT`) except `ed25519-dalek` (`BSD-3-Clause`, on `deny.toml`'s allow list): `sha2`, `rsa`, `p256`, `p384`, `ecdsa`, `ed25519-dalek`, `x509-cert`, `der`, `cms`, `pkcs8`, `pkcs12`, `spki`, `const-oid`. WebCrypto serves only the browser edge, for keys it holds. Two caveats are recorded here and checked again in [E10d](../todos/ready/E10d-pades-signing-core.md) with `cargo deny` before the first line of use:

- `rsa` carries RUSTSEC-2023-0071 (the Marvin timing side channel on private-key operations). The attack needs an observer who can time many private-key operations. A signing app on the user's own machine gives no such observer, so lamina allows RSA signing from a PKCS#12 key held in memory, and keeps RSA keys in OS stores and cards out of lamina's memory altogether. Keys the web reader generates use ECDSA. [E10d](../todos/ready/E10d-pades-signing-core.md) adds the advisory to `deny.toml` with a reason and a review date, and drops it when upstream fixes the crate.
- The RustCrypto crates have no joint audit; the `dalek` curves had one (Quarkslab, 2019). We accept this because they are the maintained, licence-compatible, `no_std`-capable option, and because the alternatives (`ring` and `aws-lc-rs` have no CMS, and OpenSSL is a C dependency that breaks ADR 0001's portability rules) cost more. E10d confirms each crate's licence and advisories with `cargo deny` before first use.

### 8. Validation result model

`validate` never returns a boolean. Each signature gets:

```rust
pub struct SignatureReport {
    pub field: LaminaPdfId,
    pub kind: SignatureKind,            // Approval | Certification | DocTimestamp | UsageRights
    pub integrity: Integrity,           // Intact | Broken | Malformed(reason)
    pub cryptography: Verdict,          // the CMS verifies, and with which algorithms
    pub trust: Trust,                   // Trusted(anchor source) | Untrusted | Unknown | Expired | Revoked | NotChecked
    pub coverage: Coverage,             // WholeFile | EarlierRevision { uncovered_bytes }
    pub modifications: ModificationLevel, // None | PermittedLtv | FormFill | Annotations | Other(findings)
    pub time: SigningTime,              // trusted (token) or claimed (/M)
    pub signer: SignerInfo,             // subject, issuer, serial, key usage, chain
}
```

It enforces the checklist of [ch. 04 §8.8](../research/pdf/04-interactive-security.md#88-attacks-on-signatures-ruhr-universität-bochum-pdf-insecurity-research) as hard failures, never skips: a missing, empty or non-hex `/Contents`, a `/ByteRange` that is not `[0 b c d]` with `b` the exact offset of this signature's `<`, a gap that is exactly the hex string, and `c+d` at a genuine revision end. The revision diff classifies every object that changed after the signed revision as permitted (DSS, document timestamps, new signatures, values of unlocked fields and their appearances, within DocMDP and FieldMDP) or `Other`; anything reachable from rendering that was redefined counts as `Other`. Validation never runs JavaScript or optional-content actions. [E10h](../todos/ready/E10h-revision-diff-and-attacks.md) ships a regression corpus of the published attack files.

### 9. Trust stores

Trust is a list of anchors from named **sources**, and every `Trusted` result names its source, so a user can see why.

- **Desktop:** the operating system's store, read at the edge and passed in, plus the EU Trusted Lists (EUTL) snapshot lamina bundles, plus anchors the user adds.
- **Web and CLI without a store:** the EUTL snapshot lamina bundles at release (refreshed each release by a task; E10g measures its size), plus anchors the user adds.
- **Not bundled:** the Adobe Approved Trust List (AATL). Its use requires an agreement with Adobe, and we do not redistribute it. A user who has the list can import it. `Trusted` therefore means "a source lamina names vouches for this chain"; lamina does not claim Acrobat's green tick.
- **Self-signed and imported certificates** validate as `Intact` and cryptographically valid with trust `Untrusted`, shown as such.

### 10. Where the pieces meet the save planner

Signing makes the document `AppendOnly` ([ADR 0007](0007-save-planner.md)). The signature field is created or reused by the form-authoring ops (E9) and filled by `prepare`/`finish`; certification (DocMDP `/P`) and field locking (FieldMDP, `/Lock`) are set in the same revision and enforced by the planner's `violations` afterwards. Signing runs after any pending rewrite in the same save, and never before one.

### 11. Qualified signatures (eIDAS QES)

lamina does not claim to produce qualified electronic signatures. A QES needs a qualified signature creation device, which means a certificate and key held by a Qualified Trust Service Provider; The owner decided on 2026-11-25: lamina builds the PAdES container and takes the provider's signature through the external signer of §3 (option A), and a desktop user with an eID card or QSCD token signs through PKCS#11 (option B, E10i). lamina plans no remote-signing client and no named provider partnership; a user who wants one starts a new decision. The validator may label a signature "qualified" from the bundled EU trusted list. [full-editor §9 Q5](../plans/full-editor.md#9-open-questions) records the answer.

## Consequences

- Signing and validation build for the browser, the desktop and the CLI from one core, and a key that cannot leave a card or a Secure Enclave still signs, because the digest and the CMS are built apart from the key.
- Validation can say "this signature is intact, but the file changed afterwards", with the changes classified, which is what Acrobat and pyHanko do and what most viewers do not.
- lamina carries a crate (`laminapdf-sign`) and an edge surface that need security review, a fuzz target for the CMS and signature-dictionary parsers (added in E10g), and a bundled trust list that someone must refresh each release.
- A TSA proxy Worker is one more deployed surface. It holds no state, no secrets and no document bytes, and the web reader works without it, only without a trusted timestamp.
- `rsa`'s timing advisory limits where RSA keys can be used until the upstream fix.

## Alternatives considered

- **Port pyHanko or call it through a sidecar.** Python in a sidecar breaks the browser and the single-binary desktop, and a port would carry its asn1crypto assumptions. We reuse its structure instead.
- **OpenSSL or a platform CMS API (CryptoAPI, Security.framework) for CMS.** Those are C or OS-specific, unavailable in WebAssembly, and would give three behaviours. Only key operations are delegated to the OS, never the CMS.
- **`ring` or `aws-lc-rs` for primitives.** Fast and audited, but no CMS, X.509 chain building or PKCS#12, so they would sit beside RustCrypto and add a second family without removing the first.
- **Sign everything with WebCrypto on the web and the OS on desktop, no Rust crypto.** That choice would leave validation without a portable implementation, and validation is where lamina differs from other viewers.
- **Send the document or its hash to a hosted signing service.** Breaks ADR 0011's privacy promise; the external signer of §3 lets a user who wants a service use one, on their own responsibility.
- **Bundle the AATL.** Not redistributable, as §9 explains.
