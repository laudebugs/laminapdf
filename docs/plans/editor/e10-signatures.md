# E10: signatures

**Status:** Draft

E10 ships two kinds of signing and names them differently everywhere: a picture of a signature on the page, and a certificate signature that proves who signed and that nothing changed afterwards. It also ships what makes the second kind mean something, validation that catches changes made after signing. The owner is asked to approve the two new ops (`Sign` and `AddDocumentTimestamp`, §2.2), the save rules of §5 and the split into eight todos (§7); §10 lists what is settled and what stays open.

Validation is the harder half. The published attacks (universal signature forgery, incremental saving, signature wrapping, shadow and certification attacks) all pass a validator that checks only the cryptography, so the round's tests lean on a corpus of attack files more than on signing itself. [ADR 0020](../../decisions/0020-signatures-and-crypto-backends.md) already fixed how lamina signs, with which crypto, and where keys live; this plan turns it into ops, save rules, fixtures, tests and pull requests.

- **Round:** [full-editor §E10](../full-editor.md#e10-signatures), phase 5, milestone M8 Forms and signing
- **Feature-plan todo:** [E10b](../../todos/done/E10b-signatures-feature-plan.md)
- **Follows:** ADR [0020](../../decisions/0020-signatures-and-crypto-backends.md) (the crate, PAdES levels, the two-phase `Signer`, key storage, the TSA proxy, the validation result, trust sources), [0005](../../decisions/0005-document-model.md) (the overlay and `LaminaPdfId`s), [0006](../../decisions/0006-op-log-and-undo.md) (ops, barriers and undo), [0007](../../decisions/0007-save-planner.md) (the save planner; §4 fixes the order redact, rewrite, sign), [0010](../../decisions/0010-platform-adapter.md) (capability flags), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (privacy, the Worker rules), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (error codes)
- **Builds on:** [E9](e9-form-authoring.md) (E9f's signature fields, `/Lock` and `/SV`; E9k's checks), [E4](e4-markup-and-drawing-annotations.md) (Ink), [E5](e5-review-stamps-interchange.md) (Stamp), [E6](e6-adding-content.md) (`AddContentObject`), [T2c](../../todos/done/T2c-reveal-command.md) (the revision index)
- **Design:** [G01 Add a signature image](../../design/paper/sign-protect-redact.md#g01-add-a-signature-image), [G02 Sign with a certificate](../../design/paper/sign-protect-redact.md#g02-sign-with-a-certificate), [G03 Certify and lock](../../design/paper/sign-protect-redact.md#g03-certify-and-lock), [G04 Signature validation](../../design/paper/sign-protect-redact.md#g04-signature-validation), [G05 Revision diff and attack warnings](../../design/paper/sign-protect-redact.md#g05-revision-diff-and-attack-warnings), [G11 Phone: sign a document](../../design/paper/sign-protect-redact.md#g11-phone-sign-a-document)

## 1. Where things stand

The planner already understands signatures. Nothing makes, checks or draws one.

- **Detection.** `laminapdf-save`'s `SaveConstraint` lists each signature as a `SignatureInfo` (its field, kind, signature dictionary, the revision it closes, and where its `/ByteRange` ends), reads DocMDP, FieldMDP, `/Lock` and UR3, and reports `Restriction::DocMdp`, `FieldMdp`, `UsageRights`, `SignedBytes` and the signed-widget restriction E9k added. `laminapdf-cos`'s `revision_index` splits a file into its revisions ([T2c](../../todos/done/T2c-reveal-command.md)).
- **No signer.** `Op` has no signing op. There is no `laminapdf-sign` crate, no CMS code, no `/Contents` placeholder writer and no validator. `SetFieldValue` already refuses a signature field (`field-is-signature`).
- **Signed fixtures.** `fixtures/signed/make.py` (pyHanko) writes approval files (`approval.pdf`, `two-signatures`), certified ones (`certified-p1`, `-p2`, `-p3`, `form-certified-p2`), locked ones (`locked`, `form-locked-include` and `-exclude`), usage-rights ones (`usage-rights` and its form variants), and `timestamp.pdf`, `signed-encrypted.pdf` and `scripts-locked-total.pdf`. The planner's tests use them today, and they are the validator's first inputs.
- **Visual signatures.** E4's Ink, E5's Stamp (an image or typed text) and E6b's `AddContentObject` can already put a picture on a page. What they lack is a capture step, a library of saved signatures and the labelling that keeps users from mistaking the picture for a certificate signature.
- **Signature fields.** E9f, still ready, adds `FieldSpec::Signature` with `/Lock` and `/SV`. E10d needs it, so the cryptographic half of the round waits on E9f and, through it, on E9k, E9b and E9e. E10c, which needs only E4d, E5c and E6b, can start now.
- **The matrix.** F.01–F.11 already have `round = "E10"`. The rows exist, so this plan adds none. F.04, F.07 and F.10 target `read`, and this plan lowers F.11 to `write` in core and `pdf` only (§2).

## 2. The operations

The round implements eleven rows of the [feature inventory](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms). Only two need new ops. Visual signatures reuse ops that exist, and the rest are reads or properties of the signing op.

| ID | What it means | Ops and functions (§2.1–§2.4) | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| F.01 | Handwritten signature image | `AddAnnot` (Stamp), `AddContentObject` | round-trip | E10c · E10c, E10j (phone sheet) · E10c |
| F.02 | Draw or type an e-signature | `AddAnnot` (Ink, or Stamp with typed text) | round-trip | E10c · E10c · E10c |
| F.03 | Certificate-based digital signatures | `Sign` | round-trip | E10d · E10i, E10j · E10i, E10j |
| F.04 | Verify signatures | `validate_signatures` | read | E10g · E10j · E10j |
| F.05 | Certify a document | `Sign` with `certify` | round-trip | E10f · E10j · E10j |
| F.06 | Trusted timestamps | `Sign` at B-T, `AddDocumentTimestamp` | round-trip | E10e · E10j · E10j |
| F.07 | Inspect signer identity and certificates | `signature_signer` | read | E10g · E10j · E10j |
| F.08 | Lock fields after signing | `Sign` with `lock` | round-trip | E10f · E10j · E10j |
| F.09 | Allowed changes after certification | the planner's DocMDP rules | round-trip | E10f, E10h · E10j · E10j |
| F.10 | Signature status and modification warnings | `diff_revisions`, `validate_signatures` | read | E10h · E10j · E10j |
| F.11 | Signing workflows via external services | `prepare_signature`, `finish_signature` | write (core, `pdf`) | E10d |

A row's column flips in the todo that completes it for that column, as in E9. F.11 has no screen: it is the API a service, a smart card or another person's machine plugs into, so its `web` and `desktop` columns are not targets. The manifest says so with `surfaces = ["core", "pdf"]` and `target = "write"`.

### 2.1 Visual signatures (F.01, F.02)

E10c adds no op. A signature image is placed with `AddAnnot`: a Stamp for an uploaded or typed signature, and an Ink annotation for strokes drawn with a pointer or pen, so the drawing stays editable as strokes. "Part of the page" uses `AddContentObject`, and the dialog offers it only when the file has no certification signature (G01). Placing is one undo step, class `Annotation` for the first two and `Content` for the third.

What E10c does add:

- **Capture.** A draw pad (strokes as pressure-free polylines), a type tab (three bundled styles), an upload tab (PNG and JPEG, white background removed) and a library tab.
- **The library.** Saved signatures live on the device and never in the document, through a new `Platform` method (`signatureLibrary`): IndexedDB on the web, the app data folder on the desktop. The phone shows the same library (G11).
- **Labelling.** Every surface that offers a picture says "This is not a digital signature". A copy test fails the build if "digital signature", "signed by" or "certified" appears in a string for the image path. The certificate path carries the opposite sentence, "Proves who signed and shows any change made after signing" ([design rule 1](../../design/paper/sign-protect-redact.md)).

### 2.2 The signing op (F.03, F.05, F.06, F.08)

`Sign` records a request to sign. It holds no key, no signature and no timestamp token, since those come from outside the document and arrive at save time through the two-phase API (§2.4).

| Op | Payload | Todo |
| --- | --- | --- |
| `Sign` | `{ target: SignTarget, params: SignParams }`, where `SignTarget` is `Field(FieldId)` or `NewField { page: PageId, rect: RectPt, name: String }`, and `SignParams` is `{ level: PadesLevel, subfilter: SubFilter, digest: DigestAlgorithm, reason, location, contact: Option<String>, claimed_time: Timestamp, appearance: SignAppearance, certify: Option<DocMdpLevel>, lock: Option<LockSpec> }` | E10d; `certify` and `lock` in E10f |
| `AddDocumentTimestamp` | `{ digest: DigestAlgorithm }` | E10e |

Targets use the typed IDs of ADR 0005 (`FieldId` and `PageId`, each wrapping a `LaminaPdfId`), so no payload holds an object number.

The payload types:

- `PadesLevel` is `BB` or `BT`; `SubFilter` is `CadesDetached` (default) or `Pkcs7Detached` (ADR 0020 §2); `DigestAlgorithm` is `Sha256`, `Sha384` or `Sha512`.
- `claimed_time` is an input, because an op takes no ambient values (ADR 0006 §1) and replaying a log must write the same bytes. The edge fills it from the clock when the user clicks Sign.
- `SignAppearance` is `Invisible` or `Visible { text: TextStyle, image: Option<BlobId>, show: Vec<AppearanceField> }`, drawn in the same op through E9's text builders. A visible signature built in the base WebAssembly module fails with `appearance-needs-text`, as fills do.
- `LockSpec` is E9f's, the `/Lock` action and field list. `Sign` writes it as the signature's FieldMDP transform (§12.8.2.4) and the planner enforces it from then on.

Both ops are class `Signature`, and neither needs a full rewrite (`requires_rewrite` false). Both are barriers (ADR 0006 §5): before a save they undo like any op, and the save that commits one drops the undo entries from before it. `Sign { target: NewField }` also creates the field, so a document with no signature field can be signed in one step, as ADR 0007 §4 describes.

The planner allows a revision that only adds a document timestamp or `/DSS` data at every DocMDP level, P1 included. The source is ISO 32000-2 §12.8.2.2.1 and its 2.0 errata ([research 04 §8.5](../../research/pdf/04-interactive-security.md#85-pades-baseline-levels-etsi-en-319-142-1-current-v121-2024-01)). E10e writes one function in `laminapdf-sign`, `is_ltv_only`, that decides this. E10f's `allowed_changes` (§5) and E10h's classifier both call it, so signing and validation cannot disagree.

### 2.3 Reading and checking (F.04, F.07, F.10)

Validation changes nothing, so it is a set of functions, not ops. They return the result model of ADR 0020 §8.

- `validate_signatures(doc, &ValidateOptions) -> Vec<SignatureReport>`: structure, integrity, cryptography, trust, coverage, modifications and time for each signature. `ValidateOptions` carries the trust anchors and any revocation data the edge fetched, never a URL to fetch.
- `signature_signer(doc, sig) -> SignerInfo`: subject, issuer, serial, key usage, validity and the certificate chain, for the certificate viewer.
- `signature_revision(doc, sig) -> Document`: the file as it stood when `sig` was made, opened read-only through the revision index. `signature_revision` feeds "Open signed version" and the compare view.
- `diff_revisions(doc, sig) -> Vec<RevisionChange>`: each object changed after `sig`, classified as allowed (`PermittedLtv`, `FormFill`, `Annotations`, a new signature) or `Other`, with a plain-words description and, for a known attack pattern, its name.

### 2.4 The two-phase API (F.11)

`prepare_signature(doc, &SignParams, target) -> PreparedSignature` assembles the signing revision in memory and returns the DER to sign. `finish_signature(prepared, signature_value, timestamp_token) -> Revision` builds the CMS and returns the bytes to append. A service, a card or a WebCrypto key sits between the two calls. `Sign` is the in-document form of the same request, and the save planner calls both functions itself when a `Signer` is attached. Placeholder size, retries and `SignError::PlaceholderTooSmall` are ADR 0020 §4's.

Each new error code gets a `docs/errors` page from the todo that adds it:

| Code | Todo | Meaning |
| --- | --- | --- |
| `sign-field-signed` | E10d | The target field already holds a signature |
| `sign-placeholder-too-small` | E10d | The CMS doesn't fit the `/Contents` placeholder; the error carries the size needed |
| `sign-key-unsupported` | E10d | The key's algorithm isn't one ADR 0020 §2 allows |
| `sign-certify-not-first` | E10f | A certification was asked for on a file that already has a signature |
| `timestamp-failed` | E10e | The TSA's answer failed a check (status, imprint, nonce, key usage or signature) |
| `signature-malformed` | E10g | A signature dictionary, `/ByteRange` or `/Contents` breaks a structural rule |
| `signature-algorithm-unsupported` | E10g | The signature uses an algorithm the validator doesn't accept |

## 3. Conformance the edits must keep

- **Bytes the signature covers never move.** The signing revision is appended after the base bytes, and `/ID[0]` stays (research 04 §9). A signature is never followed by a full rewrite without the override (ADR 0007 §3).
- **`/Contents` is unencrypted and ASCII hex.** The other strings in a signature dictionary are encrypted when the file is. Until E11b, an encrypted base is refused (`encrypted-base`), as for every op.
- **PDF/A and PDF/UA.** A visible signature widget gets an appearance stream and, in a tagged file, a `Form` element (E9's tagging); veraPDF reports nothing new on a signed `pdfa-form.pdf`.
- **Linearisation is dropped.** An appended revision breaks it, which viewers accept.

## 4. Crate and package changes

| Where | Change | Todo |
| --- | --- | --- |
| `laminapdf-sign` (new, core) | The signature dictionary and placeholder, digest over `/ByteRange`, CMS and CAdES attributes, `Signer` and a software signer, `prepare` and `finish` | E10d |
| `laminapdf-sign` | RFC 3161 request and response, the document timestamp, B-T | E10e |
| `laminapdf-sign` | The certification, FieldMDP and `/Lock` writers and the allowed-changes function the planner calls | E10f |
| `laminapdf-sign` | The validator, certificate chain building, OCSP and CRL parsers, the DSS writer, the fuzz target | E10g |
| `laminapdf-sign`, `laminapdf-cos` | The revision diff, its classifier and the attack corpus | E10h |
| `laminapdf-ops`, `laminapdf-save` | The `Sign` and `AddDocumentTimestamp` ops, the planner step that signs last, the rules of §5 | E10d, E10f |
| `laminapdf`, `@laminapdf/sdk`, `laminapdf-wasm` | The ops, the validation functions and the two-phase API, with serde samples in `fixtures/ops/v1/` | each todo for its own ops |
| `apps/tsa-proxy` (new Worker) | The `/tsa` route and the `/revocation` route of ADR 0020 §5 | E10e |
| `laminapdf-desktop-os`, `@laminapdf/sdk` worker | OS key stores and PKCS#11 on the desktop; WebCrypto and PKCS#12 on the web | E10i |
| `@laminapdf/ui` | The capture dialog and library, the sign and certify dialogs, the Signatures panel, the certificate viewer, the revision-diff banner, the phone sheet | E10c, E10j |

The crypto crates and their licence and advisory checks are ADR 0020 §7's. `laminapdf-sign` is a core crate: no network, no clock, no filesystem, no OS key store, and it builds for `wasm32`.

## 5. Saving and the save constraint

Signing makes the document `AppendOnly` from then on. `Sign` runs last in a save: pending rewrite-requiring ops write a full rewrite first, and the signature goes on top as an incremental revision (ADR 0007 §4). The table is the round's verdict on `Sign`; the planner already holds the rest.

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | `Sign` appends one revision; the file is `AppendOnly` afterwards | G02's "Signing saves the file." and, after, the "Signature valid" badge |
| `AppendOnly`: approval signatures | Signed, not certified | A further signature appends another revision. `certify` is refused (`sign-certify-not-first`), since a certification must be the first signature | G03's note "This file already has 1 signature, so it can't be certified. Sign it instead." |
| `AppendOnly` with a FieldMDP lock or `/Lock` | Some fields locked | Signing a field a lock covers is a violation (`Restriction::FieldMdp`); signing any other field is not | The conflict dialog names the lock and its signer |
| `AppendOnly` with DocMDP P1 | Certified, no changes | `Sign` is a violation (`Restriction::DocMdp`); `AddDocumentTimestamp` and DSS data are allowed | G02's danger notice, with **Save a signed copy…** |
| `AppendOnly` with DocMDP P2 or P3 | Certified, forms (and comments) | `Sign` into an existing or new signature field is allowed (ISO 32000-2 Table 257). A visual signature is an annotation, so P3 allows it and P2 does not | The first change a level forbids opens the constraint banner, as in E9 |
| Usage rights (UR3) | Extended by Adobe Reader rights | `Sign` needs `/Signature [/Modify]`; the placement ops follow E4's annotation rights | The banner when the rights don't cover the change |
| `RequiresRewrite` | A repaired file, or a pending redaction or sanitise | The rewrite first, then the signature on the rewritten output | A neutral strip saying why |
| `RequiresRewrite`: encrypted base | Password-protected | Refused (`encrypted-base`) until E11b | The warning strip |
| `Conflict` | Signatures plus a rewrite reason, or ops the constraint forbids | Save is refused; Apply anyway… invalidates the named signatures after a confirmation (ADR 0007 §3) | The danger strip naming each signature, with Save a copy… |

One rule needs the validator's help, so E10f and E10h build it together: the list of changes each certification level allows. The planner checks pending ops against that list before the user saves. The validator checks the same list after the file comes back from someone else. Both call one shared `allowed_changes` function in `laminapdf-sign`, so a change the editor lets through is never one the validator flags.

## 6. Differences between the shells

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Signature library | `signatureLibrary` (new) | IndexedDB, "stored in this browser only" | App data folder |
| Private keys | `signingKeys` (new) | WebCrypto non-extractable keys the user generates, and a PKCS#12 imported for the session | Also the OS key store (Keychain, certificate store and CNG) and PKCS#11 modules, smart cards and Secure Enclave keys |
| Timestamps | `timestampTransport` (new) | Through the optional TSA proxy, off until the user allows it; only a hash is sent | A direct call to the TSA |
| Trust anchors | `trustStore` (new) | The bundled EU Trusted Lists snapshot plus anchors the user adds | The operating system's store, the same snapshot and the user's anchors |
| Revocation | none | Off by default; a "Check revocation" button on each signature fetches through the proxy | On by default, a direct fetch |
| Phone | none: breakpoints | Image signatures, validation and the phone sheet (G11); certificate signing as a large-detent sheet when a key exists; certifying isn't offered ("Certify on a larger screen") | Not applicable |

The web reader sends nothing to a third party unless the user allows it, and the status bar's network indicator counts each request. A hash, never the file, is all that leaves the device (ADR 0011, ADR 0020 §5).

## 7. Todos

Each todo is one reviewable pull request under ~1,500 changed lines with tests, on the [todo template](../../todos/TEMPLATE.md). This plan confirms the eight implementation todos' scope and dependencies. It adds to their task lists the new `Platform` flags, error codes and functions it assigns them (§2 and §6), and it fixes the E10e todo's `Sign { timestamp }` to this plan's `params.level`. It records three checks to watch: two on todo size and one on validation order.

| ID | Todo | Depends on |
| --- | --- | --- |
| [E10c](../../todos/done/E10c-visual-signatures.md) | Capture, library and placement of image, drawn and typed signatures; the labelling copy; flips F.01 and F.02 in all four columns | E4d, E5c, E6b, E10b |
| [E10d](../../todos/ready/E10d-pades-signing-core.md) | `laminapdf-sign`, the `Sign` op, the planner's signing step, `Signer` and a software signer, the two-phase API; flips F.03 and F.11 `write` (core, `pdf`) | E9f, E10b |
| [E10e](../../todos/ready/E10e-timestamps-and-tsa-proxy.md) | RFC 3161, B-T, `AddDocumentTimestamp`, the TSA proxy Worker and its `/revocation` route; flips F.06 (core, `pdf`) | C4a, E10d |
| [E10f](../../todos/ready/E10f-certification-and-locking.md) | `certify` and `lock` on `Sign`, the allowed-changes function, the planner's rule tests; F.05 and F.08 round-trip, F.09 `write` | E10d |
| [E10g](../../todos/ready/E10g-signature-validation.md) | The validator, chain building, OCSP and CRL parsers, the DSS writer, the fuzz target, the certificate data API; F.04 and F.07 `read` (core, `pdf`) | E10d |
| [E10h](../../todos/ready/E10h-revision-diff-and-attacks.md) | `diff_revisions`, the classifier, the attack corpus; F.09 round-trip, F.10 `read` (core, `pdf`) | E10f, E10g, T2c |
| [E10i](../../todos/ready/E10i-platform-signers.md) | OS key stores, PKCS#11, WebCrypto and PKCS#12 adapters, the certificate picker; F.03 `write` (web, desktop) | C3a, D2c, E10d |
| [E10j](../../todos/ready/E10j-signature-ui.md) | G02–G05 and G11: the sign and certify dialogs, the Signatures panel, the certificate viewer, the warnings; the round's close | E10c, E10e, E10h, E10i |

E10c starts now. The cryptographic todos wait on E9f (and, through it, E9k, E9b and E9e), since E10d needs E9f's signature fields, and everything after E10d waits on E10d, so it sets the round's pace. E10e, E10f and E10g then run side by side, E10h follows E10f and E10g, and E10i can start as soon as E10d lands. E10j goes last.

Three checks to watch:

- **E10d is the round's largest.** It holds the crate, the CMS builder, the placeholder, the op, the planner step and the SDK surface. If the draft passes ~1,500 lines, split it. E10d keeps `prepare`, `finish` and `Sign`. A new todo takes the software signer and the SDK wrapper; E10e and E10i then depend on that todo, and the E10d row changes to match.
- **E10g and E10j are large too.** If E10g passes ~1,500 lines, the OCSP and CRL parsers and the DSS writer move to a todo of their own, since only revocation needs them. If E10j does, the sign and certify dialogs (G02, G03, G11) split from the panel, the certificate viewer and the warnings (G04, G05).
- **E10g's validator ships before E10h's diff.** Until E10h, a validation report says `modifications: NotChecked`, never `None`, so the badge can't claim "unchanged" before anything has checked it. ADR 0020 §8 lists `NotChecked` for this reason.

## 8. Fixtures

Lamina PDF's own scripts write every new fixture, licensed like the rest of Lamina PDF (`MIT OR Apache-2.0`), with provenance in `fixtures/README.md`.

- **Test keys and certificates** (`fixtures/signed/keys/`, written by `fixtures/signed/make.py`): a test root and intermediate CA, RSA and P-256 end-entity certificates, an Ed25519 one, an expired one and one without the signing key usage, each as a PKCS#12 with the password `test`. They sign nothing real and are labelled as test material in the README.
- **Signed outputs** (E10d onwards): `signed-b-b.pdf`, `signed-b-t.pdf` (timestamp from a test TSA key), `signed-cades-visible.pdf`, `signed-invisible.pdf`, `signed-pkcs7.pdf`, `signed-ecdsa.pdf`, `signed-ed25519.pdf`, `signed-twice.pdf`, and `signed-form-lock.pdf` (a signature with a FieldMDP lock).
- **Recorded responses** (`fixtures/signed/revocation/`): OCSP and CRL responses for the test CA, good, revoked and unknown, and a `TimeStampResp` for each hash, so no test reaches the network.
- **The attack corpus** (`fixtures/signed/attacks/`, E10h): one file per published attack, rebuilt by a script from the papers' descriptions with provenance beside each. Each file has a `.expected.json` naming the verdict, the finding and its pattern.

  | Attack | Files |
  | --- | --- |
  | Universal signature forgery | a missing, `null`, empty and non-hex `/Contents`; a missing, empty and shifted `/ByteRange` |
  | Incremental saving | page content redefined after the signed range, with and without a new xref |
  | Signature wrapping | a moved `/Contents` and a widened gap |
  | Shadow attacks | hide, replace and hide-and-replace through an allowed update |
  | Certification attacks | the evil annotation attack at P3, the sneaky signature attack at P2 |
- **The trust list snapshot** (`fixtures/signed/trust/`): a small, frozen copy of the EU Trusted Lists for tests, with its source URL, date and hash. The full snapshot ships with the app, and E10g records its size.
- **The pinned corpus** (nightly, never copied): signed files from public sources, such as the PDF Association's and pyHanko's test sets and the RUB `pdf-insecurity.org` samples where their licences allow, to check that validation agrees with pyHanko's on real files.

## 9. Tests

Each op gets the tests its todo lists ([ADR 0018](../../decisions/0018-test-strategy.md)). "Round-trip" means Lamina PDF, PDFium and qpdf (`--check` clean) each reopen the saved file and report the signature, and pyHanko and OpenSSL validate it. pdf.js doesn't validate signatures, so it only has to render the appearance.

| Op or function | Unit | Golden | Round-trip | Differential | Fuzz | e2e |
| --- | --- | --- | --- | --- | --- | --- |
| Visual signatures (E10c) | capture to polylines, Stamp and Ink payloads, copy test | the Stamp and Ink appearances on a fixture page | reopen shows the image and no signature objects | — | `ops_json` | Playwright: draw, type, upload and place; the library persists |
| `Sign` (E10d) | placeholder sizing and retry, ByteRange arithmetic, every algorithm, a rejected key | `signed-b-b.pdf` structure | pyHanko validates; `openssl cms -verify` on the extracted ranges; PDFium counts one signature | pyHanko's validation result against lamina's on every `signed-*.pdf` | `ops_json` | — |
| `prepare_signature`, `finish_signature` (E10d) | the same bytes as `Sign` with the same signer; a signer that answers late | — | a signature made across the two calls validates | — | — | — |
| `AddDocumentTimestamp`, B-T (E10e) | request and response checks, nonce and imprint mismatch, a TSA certificate without the extended key usage | `signed-b-t.pdf` | pyHanko reads the timestamp and validates it | the same | `tsa_response` | TSA proxy: allow-list, body limit, no body in logs |
| `certify`, `lock` (E10f) | each DocMDP level and each lock action | `certified-p1`, `-p2`, `-p3` rewritten by lamina | pyHanko accepts the certification and each lock | `would_violate` verdicts per op per level | `ops_json` | — |
| `validate_signatures` (E10g) | each structural rule, each trust and revocation state | a report per fixture | — | verdicts against pyHanko and, nightly, the EU DSS validator | `signature_parse`: signature dictionaries, CMS and `TSTInfo` | — |
| `diff_revisions` (E10h) | each change class, the LTV class at P1 | `.expected.json` per attack file | — | the attack corpus run through pyHanko and lamina | — | — |
| Platform signers (E10i) | the `Signer` contract per adapter | — | a signature made with each key store validates | — | — | WebdriverIO: a temporary Keychain on macOS, the certificate store on Windows, SoftHSM2 over PKCS#11 on Linux; Playwright: WebCrypto and PKCS#12 |
| Interface (E10j) | Vitest for the dialogs, panel state and certificate viewer | — | — | — | — | Playwright and WebdriverIO: sign, reopen, see the badge; open a tampered copy, see the banner and the diff; axe on each screen |

Across every op:

- apply then undo leaves the overlay empty and the next save byte-identical (ADR 0006 §3), with `Sign` the barrier that stops undo from crossing it;
- the same state saves to the same bytes on native and `wasm32`, given the same `claimed_time` and signer output;
- `would_violate` on every signed fixture gives §5's verdict, asserted per op and per level, because a misclassified op is a security bug;
- veraPDF reports nothing new on `pdfa-form.pdf` after signing.

The validator's tests treat any structural anomaly as a hard failure, and a test that reports such a file as valid fails the build. A signature passes only if:

- `/Contents` is present, non-empty and ASCII hex;
- `/ByteRange` is `[0 b c d]`, with `b` at the `<` of this signature's `/Contents`;
- the gap between the two ranges is exactly that hex string;
- `c+d` is the end of a genuine revision.

A new fuzz target's `fixtures/regressions/<target>` folder joins `laminapdf-model`'s regression guard list.

## 10. Decisions on the open questions

Items 1–7 and 11 are settled, by an ADR or by this plan's recommendation. Items 8–10 name who decides.

1. **Qualified electronic signatures.** Answered by [ADR 0020 §11](../../decisions/0020-signatures-and-crypto-backends.md#11-qualified-signatures-eidas-qes): the external signer, and eID cards and QSCD tokens through PKCS#11. No remote-signing client, no named providers.
2. **Which trust lists ship on the web?** Answered by ADR 0020 §9: a bundled snapshot of the EU Trusted Lists, refreshed by a task at each release, plus anchors the user adds. The Adobe Approved Trust List isn't bundled, so a `Trusted` result names its source and never claims Acrobat's green tick.
3. **Revocation on the web.** Answered by ADR 0020 §5: off by default, with a "Check revocation" button on each signature.
4. **The plan's location:** `docs/plans/editor/`, as E1–E9 use.
5. **Where a signature image goes by default.** A Stamp annotation, which a user can move, resize and delete. Drawn strokes stay Ink. "Part of the page" is opt-in and unavailable on a certified file (G01).
6. **Signing time.** `claimed_time` is an input to `Sign` and becomes `/M`. A CAdES signature carries no signing-time attribute ([research 04 §8.4](../../research/pdf/04-interactive-security.md#84-subfilters)), so the only trusted time is a timestamp token. The validator labels the two differently: "signed at (signer's clock)" and "signed at (timestamp)".
7. **Appearance of a visible signature.** A text block (name, date, reason, location) with an optional image, drawn from E9's builders. The signing revision includes the appearance, so changing it later is a modification the validator reports.
8. **Long-term validation.** B-LT and B-LTA signing flows are not in E10 (ADR 0020 §2). The DSS writer and the validator's use of embedded revocation data ship in E10g. **Decision for the owner, after E10j:** whether a B-LT signing todo joins a later round.
9. **PKCS#11 module discovery.** The desktop asks the user for the module's path once and remembers it, and offers common paths per operating system as suggestions. The desktop loads no module on its own. **Decided in E10i, round E10.**
10. **Hardware-backed tests.** CI cannot hold a real card or a Secure Enclave key. E10i tests the Keychain with a temporary keychain and generated keys, Windows with a throwaway store, and PKCS#11 with SoftHSM2; a manual Acrobat and card check is listed in its post-merge checks. **Decided in E10i, round E10.**
11. **Phone certificate signing.** Offered when a key exists on the device, through the same sheet as the desktop (G11); the web has no phone key store, so the phone web reader offers images, validation and imported PKCS#12 only. Certifying stays off the phone.
