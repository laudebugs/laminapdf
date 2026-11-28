# E11: security and sanitisation

**Status:** Draft

E11 lets a person protect a PDF with passwords and permissions, remove that protection when they hold the right to, and strip hidden information before a file leaves their hands. Reading encrypted files already works. Nothing writes an encrypted file, and no op removes metadata, scripts or attachments. The owner is asked to approve four things: the eight new ops (§2), the save rules (§5), the AES-256-only decision (§10, item 1) and the split of the round into six todos (§7).

The round has one hard rule: a protected or sanitised file must not leak what it claims to hide. PDFex (research 04 §7.7) showed that a file encrypted in part leaks through its plaintext parts, and the sanitising tools people use leave metadata, earlier revisions and attachments behind. So the tests scan the output bytes, and the sanitiser is finished only when `laminapdf reveal` finds nothing in the categories it removed.

- **Round:** [full-editor §E11](../full-editor.md#e11-security-and-sanitisation), phase 6, milestone M9 Advanced
- **Feature-plan todo:** [E11a](../../todos/done/E11a-security-and-sanitisation-feature-plan.md)
- **Follows:** [ADR 0005](../../decisions/0005-document-model.md) (overlay and `LaminaPdfId`s), [0006](../../decisions/0006-op-log-and-undo.md) (ops, barriers, undo), [0007](../../decisions/0007-save-planner.md) (the planner; §4 fixes the order redact, rewrite, sign), [0010](../../decisions/0010-platform-adapter.md) (capability flags), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (privacy), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0020](../../decisions/0020-signatures-and-crypto-backends.md) (RustCrypto, key stores), [0021](../../decisions/0021-pdf-javascript-policy.md) (scripts and XFA), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (error codes)
- **Builds on:** [B2b](../../todos/done/B2b-standard-security-decryption.md) (decryption), [T2c](../../todos/done/T2c-reveal-command.md) (`laminapdf reveal`, the category names), [E3f](../../todos/done/E3f-attachments-and-associated-files.md) (attachments), [E5b](../../todos/done/E5b-comment-threads-and-review.md) (comments), [E7c](../../todos/done/E7c-content-ir.md) (the content IR), [E10](e10-signatures.md) (signatures, for E11e and for §5)
- **Design:** [G06 Protect](../../design/paper/sign-protect-redact.md#g06-protect), [G07 Sanitise](../../design/paper/sign-protect-redact.md#g07-sanitise)

## 1. Where things stand

Reading is complete. Writing, removing and sanitising do not exist.

- **Reading.** `laminapdf-syntax`'s `crypt` module reads R2 to R6 (RC4 40 and 128, AESV2, AESV3) through `authenticate` and `decrypt`. `laminapdf-cos` exposes `Document::is_encrypted`, `encryption()`, `permissions()` and `matched_password()`, and `OpenOptions::with_password`. The SDK takes `password` on open and retries an NFKC-normalised form after `wrong-password`. The UI has `PasswordDialog`.
- **No writer.** Core cannot encrypt: it has no encrypt function and no random source, and the workspace `Cargo.toml` lists no `cbc` or RNG crate. `aes`, `sha2` and `md-5` are workspace dependencies already, and the CBC chaining is hand-written in `crypt/aes.rs`.
- **The planner refuses.** An encrypted base adds `RewriteReason::EncryptedBase`, and every mode returns `encrypted-base` unless `SaveOptions::with_remove_encryption(true)` is set, which forces a plaintext full rewrite. Nothing re-encrypts. `Restriction` has no permission variant.
- **Ops.** `Op::requires_rewrite` is `false` for every variant. Metadata, XMP and attachment ops exist (`RemoveMetadata`, `RemoveXmpProperty`, `ReplaceXmpPacket`, `DeleteAttachment`, `DeleteAnnot`). No op touches scripts, actions, layers or encryption. `ModificationClass::Document` already names "encryption" in its doc comment.
- **The sanitiser's yardstick.** `laminapdf reveal` reports seven categories: `history`, `metadata`, `attachments`, `active-content`, `annotations`, `forms` and `hidden-content`. `laminapdf::redaction` and `revisions` hold `hidden_text` and the redaction warnings.
- **Fixtures.** `fixtures/encrypted/` holds R2 to R6 files made by qpdf, with the passwords in `passwords.json`. Hidden-content files exist in pieces (`attachments/`, `layers.pdf`, `links.pdf`, `metadata/`), and nothing combines them.
- **The matrix.** G.01 to G.05, G.08 to G.13 already carry `round = "E11"`, and G.11 targets `read`. This plan adds no rows and changes two targets: G.12 to `read` and G.13 to `write` (§2).

## 2. The operations

Eleven manifest rows, eight new ops. The strip ops are small and independent so the UI can offer them one at a time, and `Sanitise` is the preset that runs them together.

| ID | What it means | Ops and functions | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| G.01 | Document-open password | `SetEncryption` (user password) | round-trip | E11b · E11f · E11f |
| G.02 | Encrypt a PDF | `SetEncryption` | round-trip | E11b · E11f · E11f |
| G.03 | Permission passwords | `SetEncryption` (owner password) | round-trip | E11b · E11f · E11f |
| G.04 | Restrict print, copy, edit | `SetEncryption` (`permissions`) | round-trip | E11b · E11f · E11f |
| G.05 | Remove security with authorisation | `RemoveEncryption` | round-trip | E11c · E11c · E11c |
| G.08 | Remove hidden metadata and comments | `StripMetadata`, `StripAnnotations` | round-trip | E11d · E11g · E11g |
| G.09 | Sanitise embedded content | `StripActiveContent`, `Sanitise` | round-trip | E11d · E11g · E11g |
| G.10 | Remove attachments and scripts | `StripAttachments`, `StripActiveContent` | round-trip | E11d · E11g · E11g |
| G.11 | Inspect encryption | `encryption_info()` (a read) | read | E11b · E11f · E11f |
| G.12 | Certificate-based security | `SetEncryption` with recipients (the write half, behind `experimental_pubsec`) | read | E11e · E11e · E11e |
| G.13 | Remove hidden layers and information | `StripHiddenContent` | write | E11d · E11g · E11g |

A row's column flips in the todo that completes it. This plan changes two targets in the manifest, to match what the todos already say: G.13 becomes `write`, as E11d says, and G.12 becomes `read`, since the write half is experimental and ships off by default. The write half is tested all the same, and the README says "experimental".

Every op below is a barrier that requires a rewrite and has class `Document` (ADR 0006 §1). None takes an ambient input: a password is a `Secret` the caller passes, the entropy comes through `SaveOptions` (§2.1), and no op names a page or object, since each acts on the whole file.

| Op | Payload | Todo |
| --- | --- | --- |
| `SetEncryption` | `user`, `owner` (`Secret`s), `permissions`, `encrypt_metadata`, optional `recipients` | E11b, E11e |
| `RemoveEncryption` | none: authorisation is read from the open document's `matched_password()` and `/P` (§2.2) | E11c |
| `StripMetadata`, `StripAnnotations`, `StripAttachments`, `StripActiveContent`, `StripHiddenContent` | none: each acts on the whole file | E11d |
| `Sanitise` | `categories` (reveal's names); expands to the strip ops it names, so it adds no class of its own | E11d |

### 2.1 Encryption (G.01 to G.04, G.11)

`SetEncryption { user, owner, permissions, encrypt_metadata }` writes the standard security handler at R6: `/V 5`, `/R 6`, crypt filter `/StdCF` with `AESV3`, `/U`, `/UE`, `/O`, `/OE`, `/Perms`, `/EncryptMetadata`. The op is a barrier that requires a rewrite, class `Document`.

- **Passwords are `Secret`s.** A `Secret` is a byte string that zeroises on drop, prints as `<secret>` in `Debug`, and serialises as JSON `null` (a placeholder string such as `"<secret>"` could be someone's password). An op log that holds one cannot replay an encrypted save: the deserialiser reads the `null` back as a missing secret, and applying the op answers `secret-missing`. So the editor treats `SetEncryption` as unpersistable, the recovery copy and crash reports drop it with a notice, and the `fixtures/ops/v1/` sample holds `null`. Passwords stay in the worker (G06).
- **Entropy is injected.** Core has no ambient randomness. `SaveOptions` takes 32 random bytes (`with_entropy_seed`, `entropySeed` in the SDK) and `SeededEntropy` expands them with SHA-256 in counter mode into the salts, the file key and every IV, so the options stay `Copy` and a fixed seed makes a save byte-reproducible. The SDK draws the seed from `crypto.getRandomValues` on every save, native shells pass `getrandom`'s. A save that encrypts without a seed fails with `encryption-no-entropy`.
- **Empty passwords.** The owner password is the one G06 calls the permissions password. An empty user password opens without a prompt (the reader already does this) and an empty owner password is replaced by a random one the user is told to keep, since R6 needs a distinct owner entry. Neither password set is the error `encryption-no-password`. The save reports a made owner password once (`SaveReport::generated_owner_password`), as a `Secret`.
- **Permissions.** `permissions` maps the dialog's controls to the `/P` bits ([04 §7.4](../../research/pdf/04-interactive-security.md#74-permission-bits-p)):

  | G06 control | Bits (ISO 32000-2 Table 22) |
  | --- | --- |
  | Printing: High quality | 3 and 12 |
  | Printing: Low resolution only | 3 |
  | Printing: Not allowed | none |
  | Copy text and images | 5 |
  | Fill in forms and sign | 9 |
  | Add comments | 6 |
  | Insert, rotate or delete pages | 11 |
  | Edit contents (no control in G06): off whenever a permissions password is set, and E11b records it | 4 |
  | Screen readers | 10, always set |

  Bit 10 is always set, as G06's note says, and a test fails if it isn't: a protected file must stay readable to assistive technology. PDF 2.0 deprecates the bit, and research 04 §7.4 says it did not verify how readers treat it, so E11b checks pdf.js and PDFium against a file with it on and leaves it set.
- **PDFex.** The writer encrypts every string and stream except those the standard exempts (the `/Encrypt` dictionary, `/ID`, a signature's `/Contents`, xref streams, and the metadata stream when `/EncryptMetadata` is false with an `Identity` crypt filter). A writer that cannot encrypt an object returns `encryption-incomplete` and writes nothing, and a test scans the output for a sentinel plaintext. `/Perms` is computed and checked on read-back.
- **Inspecting.** `encryption_info()` returns the handler, revision, version, cipher, key length, what `/EncryptMetadata` says, the permission bits and which password opened the file, built from `cos`'s `Encryption`, `permissions()` and `matched_password()`. The function never returns key material.

### 2.2 Removing and changing security (G.05)

`RemoveEncryption` removes `/Encrypt` and writes a full rewrite (no earlier revision survives). The save planner's existing `with_remove_encryption(true)` becomes this op's implementation. Changing passwords or permissions is `SetEncryption` on an encrypted base, so the dialog's **Change passwords or limits…** needs no third op.

Authorisation is a check the core makes before the op applies, never the UI's:

- the file opened with the **owner** password (`matched_password() == Owner`), or
- it opened with a **user** password that grants every permission bit (`/P` all set), which is the case for a file that is protected only to be opened.

A file whose empty user password opened it silently, with restrictions, is neither. The dialog asks for the owner password, authenticates it against `/O`, and on failure shows G06's "That password doesn't match" message. The error code is `owner-password-required`. Research 04 §7.5 explains why owner-only protection isn't security: someone with the user password can already read, copy and print, and the plan does not pretend otherwise.

### 2.3 Sanitising (G.08, G.09, G.10, G.13)

Five strip ops, each requiring a rewrite, class `Document`, each returning the items it removed. `categories` use `laminapdf reveal`'s names, so the report and the reveal listing speak one language.

| Op | Category | Removes |
| --- | --- | --- |
| `StripMetadata` | `metadata` | `/Info`, XMP, `/PieceInfo`, page and document thumbnails, `/Metadata` streams on objects; keeps nothing the conformance claims need unless the user also runs `RemoveConformanceClaims` (the report says so) |
| `StripAnnotations` | `annotations` | Comments and markup (every annotation except links, widgets and popups) with their replies; the preset leaves it unticked ("kept in this preset", G07) |
| `StripAttachments` | `attachments` | `/EmbeddedFiles`, `/AF`, File Attachment annotations, portfolio collections |
| `StripActiveContent` | `active-content` | `/Names /JavaScript`, `/OpenAction` and `/AA` scripts, Launch, SubmitForm, ImportData, GoToR and URI actions to external targets, and XFA |
| `StripHiddenContent` | `hidden-content` | Content in optional-content groups that are off, content wholly outside the page's CropBox, and the OCG dictionaries left empty |

`Sanitise { categories }` is the dialog's op. It expands to the matching strip ops in the log (so undo takes them together), and `history` adds nothing to the log because the rewrite itself drops earlier revisions, the unreferenced objects and the free list. Because `history` always runs, a sanitised file is always a single revision, as G07 says. Three cases need more than deletion:

- **XFA.** `StripActiveContent` removes `/AcroForm /XFA` whole (the stream or the packet array and every packet it names) and clears `NeedsRendering`, as the E11d todo specifies. The AcroForm fields stay as the static fallback. A dynamic XFA form shows a warning in the dialog first (G07).
- **Formatting scripts.** A field's `/AA` format and keystroke scripts go with the rest, and the report names the fields that lose their formatting (ADR 0021 §3).
- **Hidden content needs the IR.** An OCG-off content run can be dropped only by editing the content stream, so `StripHiddenContent` depends on E7c's content IR. Off-page content is cut at the CropBox. Neither flattens a page to an image: text stays text, which is what the Hacker News research (R8) asked for, and a test extracts text before and after.
- **The report.** `SanitiseReport` is JSON, schema `laminapdf.sanitise/1`, with one section per category, each removed item named by the object number `laminapdf reveal` lists, a "kept" section for unticked categories and a post-removal scan: `reveal` run on the result must list nothing in the removed categories. The scan runs inside the op, and a non-empty result is `sanitise-incomplete`, so a bug in a strip op shows up as a failed save. The report holds object numbers and counts, not document text.

### 2.4 Certificate-based security (G.12)

`SetEncryption` takes optional `recipients` (certificates and each one's permissions) and writes `/Adobe.PubSec` with `adbe.pkcs7.s5`, one CMS `EnvelopedData` per recipient group. It is gated by `experimental_pubsec` in `SaveOptions`, off in the UI unless the person turns on Experimental features. Reading needs a private key through E10i's key stores, so E11e depends on E10g and E10i and ships last. The algorithm limits per platform are in the E11e todo and stay there.

## 3. Conformance the edits must keep

- **PDF/A forbids encryption.** `SetEncryption` on a file that claims PDF/A is refused with `encryption-pdfa-conflict`; the dialog offers **Remove the PDF/A claim and protect** (`RemoveConformanceClaims` first). PDF/UA is unaffected as long as bit 10 stays set; E11b confirms that against veraPDF.
- **Order in a save.** Strip ops and redaction first, then the rewrite with encryption, then any signature on top (ADR 0007 §4). A signature on an encrypted file keeps `/Contents` unencrypted (E10 plan §3).
- **A rewrite keeps the file's protection.** Since E11b, a rewrite of an encrypted base (for example a sanitise) re-encrypts every object under the file key the open already derived, with the same `/Encrypt` dictionary, and needs no password again, and an incremental save encrypts what it appends. Only `SetEncryption` and `RemoveEncryption` change it. The planner adds `RewriteReason::EncryptedBase` only when the options remove the encryption, and `encrypted-base` is left to the spike-era test writers.
- **`/ID`.** The first element stays on every save, as E10 needs. R6 derives no key from `/ID` (only R2 to R4 do), so a rewrite that changes the encryption keeps the first element too; the second still follows the content, as for any save (decided in E11b).
- **Linearisation is dropped,** as for every rewrite.
- **Timestamps and long-term validation** on an encrypted file follow E10's rules, and E11 adds none.

## 4. Crate and package changes

| Where | Change | Todo |
| --- | --- | --- |
| `laminapdf-syntax` | `crypt::new_handler` (the R6 hash, `/U /UE /O /OE /Perms`), `crypt::encrypt` (AES-256, and AES-128 and RC4 to keep an older base's protection), the `Entropy` trait with `SeededEntropy`, the `/P` builder; `cbc` is not added, the hand-written chaining gains an encrypt direction. `laminapdf-cos` gains `Encryptor`, the object encryptor | E11b |
| `laminapdf-ops` | `SetEncryption`, `RemoveEncryption`, `Secret`, the `requires_rewrite` arms | E11b, E11c |
| `laminapdf-save` | The planner keeps encryption on an encrypted base, writes it on `SetEncryption`, adds `Restriction::Permissions` for the advisory check, and runs the post-sanitise scan | E11b, E11c, E11d |
| `laminapdf-ops`, `laminapdf` | The five strip ops, `Sanitise`, `SanitiseReport`, serde samples in `fixtures/ops/v1/` | E11d |
| `laminapdf`, `@laminapdf/sdk`, `laminapdf-wasm` | The ops, `encryption_info()`, the entropy hand-off, the report type | each todo for its own ops |
| `laminapdf-sign` (E10) | The CMS `EnvelopedData` writer and the `Decryptor` trait | E11e |
| `@laminapdf/ui` | The Protect dialog with its Certificates tab, the remove-protection dialog, the status-bar security item, the encryption inspector, the permission-based tool disabling | E11c, E11f, E11e |
| `@laminapdf/ui` | The Sanitise dialog and report panel | E11g |

`laminapdf-syntax` stays a core crate: no network, no clock, no filesystem, and `wasm32`-clean. The new crypto uses crates already in `Cargo.toml` (`aes`, `sha2`, `md-5`); ADR 0020 §7 forbids hand-written primitives, and the CBC chaining that exists already is a mode, not a cipher. If E11b finds the chaining better replaced by the `cbc` crate, it adds that crate with the licence and advisory checks and says so in its PR.

## 5. Saving and the save constraint

All eight new ops require a rewrite. On an unsigned file that is `RequiresRewrite` and nothing more. A signature turns it into a `Conflict`, because a rewrite changes the bytes the signature covers (ADR 0007 §3).

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` or `RequiresRewrite`, unsigned, unencrypted | The usual case | One full rewrite, encrypted when `SetEncryption` is pending, with no earlier revision | "Encrypting… 62%" (G06); the status bar's security item after |
| Encrypted base, no encryption op | Any rewrite or incremental save | Incremental: new objects encrypted under the existing key. Rewrite: every object re-encrypted under it, same `/Encrypt` dictionary | Nothing, except a file opened without the owner password that forbids the change (below) |
| Encrypted base, `RemoveEncryption` | Authorised (§2.2) | Plaintext full rewrite, `/ID` refreshed | G06-remove-security's rewrite note |
| Encrypted base, `SetEncryption` | Change of passwords or limits | Full rewrite under the new handler | G06 pre-filled |
| `AppendOnly`: DocMDP P1, P2 or P3, a FieldMDP lock or `/Lock`, alike | Signed, certified or locked | A pending rewrite-requiring op makes it a `Conflict` | The danger strip naming each signature with **Save a protected copy…** (keeps the signed original) and **Remove signatures and protect** (G06, G07) |
| `Conflict` | Signatures plus these ops | Save is refused. The override invalidates the named signatures after a confirmation (ADR 0007 §3). **Save a copy** writes the new file and leaves the original | G06's and G07's signature notices |
| Usage rights (UR3) | Extended by Adobe Reader rights | A rewrite removes the rights signature, so it follows the signed case | The usage-rights banner, as in E9 |
| Permissions forbid the change | The file opened with a user password whose `/P` forbids the edit | The ops apply; Save writes. The permission bits are advisory (§10, item 2); the interface disables the tools with the reason, and opening with the owner password lifts them | A reason on each disabled tool: "This file's permissions don't allow adding comments. Open it with the permissions password to change that." |

One rule the planner needs from this round: a `SetEncryption` whose `permissions` forbid an action does not stop later ops. It records `Restriction::Permissions` so the UI can warn that the saved file restricts what the next person may do, and it never blocks the person who just set it.

## 6. Differences between the shells

The shells behave the same. Keys and passwords never leave the device, and E11 adds no server feature.

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Entropy | none: the host passes 32 bytes (`entropySeed`) | `crypto.getRandomValues` in the worker, drawn on every save by the SDK | `getrandom` |
| Passwords | none | Held in the worker only, zeroised after the save; not in the recovery copy, crash report or log | The same, in the engine process |
| Recipient certificates | `certificateStore` (new, E11e) | Imported `.cer` and `.p7b` files only | Also the Keychain, Windows certificate store and NSS (G06) |
| Private keys for opening | `signingKeys` (E10) | WebCrypto non-extractable keys and an opt-in PKCS#12 | The OS key stores and PKCS#11 |
| Phone | none: breakpoints | Protect and Remove become large-detent sheets; the Certificates tab is not offered | Not applicable |

The web reader's network indicator never counts a request for E11, because none is made.

## 7. Todos

Each todo is one reviewable pull request under about 1,500 changed lines, on the [todo template](../../todos/TEMPLATE.md). E11b and E11d each held a dialog and a core change that together would overflow, so this plan moves the dialogs into new E11f and E11g and confirms E11c and E11e as they stand.

| Todo | Scope | Depends on | Size |
| --- | --- | --- | --- |
| [E11b](../../todos/done/E11b-encrypt-on-write.md) | `crypt::encrypt`, `Entropy`, `SetEncryption` (no recipients), the planner's keep-encryption rule, `encryption_info()`, the PDFex checks, the permission-based disabling in Comment mode, the SDK surface. The Protect dialog and inspector panel move to E11f | B2b, E11a | L |
| [E11c](../../todos/ready/E11c-remove-security.md) | `RemoveEncryption`, the authorisation check, the remove-protection and change flows, the status-bar security item (needs only the info E11b returns) | E11b | S |
| [E11d](../../todos/ready/E11d-sanitiser.md) | The five strip ops, `Sanitise`, `SanitiseReport`, the post-removal scan, the SDK surface. The dialog and panel move to E11g. **New dependency on E11b**, so a sanitise of an encrypted file re-encrypts instead of refusing | E3f, E5b, E7c, E11a, E11b, T2c | L |
| [E11e](../../todos/ready/E11e-public-key-security.md) | Public-key read and the flagged write, the `Decryptor` trait, the Certificates tab | E10g, E10i, E11b, E11f | M |
| [E11f](../../todos/ready/E11f-protect-dialog-and-inspector.md) (new) | The Protect dialog (Password tab), its signed-file notice and **Save a protected copy…**, the encryption inspector panel, the document-open password flow's copy, e2e on web and desktop | E11b | M |
| [E11g](../../todos/ready/E11g-sanitise-dialog-and-report.md) (new) | The Sanitise dialog with presets and counts, the report panel, Export report and Copy as text, e2e on web and desktop | E11d | M |

E11c's design has the remove dialog and E11f's has the protect one, and both end up in the same file, so E11c and E11f take turns: E11c creates `security/` in `@laminapdf/ui` with the remove dialog, and E11f adds beside it. They can run in either order. The order that matters is E11b first, then E11c, E11d and E11f in parallel, then E11g, then E11e last because it waits on E10.

**Watch items.** Two checks for the E11b and E11d authors: E11b's size depends on how much of the permission-based tool disabling is already in E4f's session block (if it only needs a bit table, it stays), and E11d's `StripHiddenContent` is the riskiest op because it edits content streams (if it grows past the PR's budget, it splits out as E11h and the other four strip ops ship without it).

## 8. Fixtures

Lamina PDF's own scripts write every new fixture, licensed like the rest (`MIT OR Apache-2.0`), with provenance in `fixtures/README.md`.

- **Encrypted outputs** (`fixtures/encrypted/written/`, E11b): files the writer produces, kept as goldens with the `Entropy` test source so the bytes are stable: `r6-user.pdf`, `r6-owner-permissions.pdf` (each control of §2.1), `r6-no-metadata-encryption.pdf`. A signed one (`r6-signed.pdf`) waits for E10d, which writes the first signature. The existing qpdf-made files in `fixtures/encrypted/` stay the reader's inputs.
- **Sentinel plaintext:** every source page carries the string `LAMINA-SENTINEL-7f3a` in its content, an annotation, a form value, the Info dictionary and an attachment, so the leak scan has something to find.
- **Hidden-everything** (`fixtures/sanitise/hidden-everything.pdf`, E11d): metadata (Info, XMP, `/PieceInfo`, a thumbnail), a comment thread, an attachment and an `/AF` entry, a document script, an `/OpenAction` script, a Launch action, a dynamic XFA packet, an off OCG with content, off-page content, and three revisions. One category per file beside it (`only-metadata.pdf`, `only-scripts.pdf`, and so on), each with an `.expected.json` listing the object numbers `reveal` finds.
- **`sanitise-clean.pdf`:** nothing hidden, for the "Nothing hidden to remove" state.
- **Signed and encrypted:** the existing `fixtures/signed/signed-encrypted.pdf`, plus `certified-p1.pdf` for the conflict paths.
- **Public-key files** (`fixtures/encrypted/pubsec/`, E11e): written by pyHanko's public-key handler with the test keys E10d adds under `fixtures/signed/keys/`, one for RSA PKCS#1 v1.5, one RSA-OAEP, one ECDH.

## 9. Tests

Each op gets the tests its todo lists ([ADR 0018](../../decisions/0018-test-strategy.md)). "Round-trip" for encryption means qpdf (`--check` and `--show-encryption`), pypdf, pdf.js and PDFium open the saved file with the right password and show the same text and page count as the source, and each refuses the wrong one.

| Op or function | Unit | Golden | Round-trip | Differential | Fuzz | e2e |
| --- | --- | --- | --- | --- | --- | --- |
| `SetEncryption` (E11b) | each `/P` mapping and bit 10, `/Perms` check, `Secret` redaction, the empty-owner rule, the PDFex refusal | `written/*.pdf` with a fixed entropy seed | the four readers above; a leak scan finds no sentinel | qpdf and pypdf decrypt to the same plaintext as lamina's reader | the `/Encrypt` dictionary parser with R6 variants, and `ops_json` | Playwright: protect, save, reopen with each password |
| Keep encryption (E11b) | an incremental save under the existing key, a rewrite under it | a re-saved `r6-user.pdf` | qpdf `--check`, the same text | — | `save_roundtrip` | — |
| `encryption_info` (E11b) | each handler and cipher in `fixtures/encrypted/`, no key bytes in the output | JSON per fixture | — | against `qpdf --show-encryption` | — | the inspector shows the facts |
| `RemoveEncryption` (E11c) | owner, full-rights user, restricted user, wrong password | the output has no `/Encrypt` | PDFium and pdf.js open it with no password | — | — | Playwright: remove, with and without the permissions password |
| Strip ops, one per category: `StripMetadata`, `StripAnnotations`, `StripAttachments`, `StripActiveContent`, `StripHiddenContent` (E11d) | each on its single-category file and on `hidden-everything.pdf` | the report JSON per file | `reveal` on the output lists nothing in the removed categories; text extraction before and after is equal where the page isn't hidden; qpdf `--check`, pdf.js and PDFium open it | pdf.js renders each page like the source, apart from the hidden layer | `ops_json`; the sanitiser on mutated `hidden-everything.pdf` (no panic, no leaked removed object) | E11g covers the dialog; the ops have no screen of their own |
| `Sanitise` (E11d) | the preset; idempotent (a second run reports nothing); XFA removal keeps the AcroForm fields | `SanitiseReport` | the scan inside the op fails on a deliberately broken strip | — | — | — |
| Sign over these (E11b, E11d) | the `Conflict` row of §5 for each of the eight ops at P1, P2 and P3 | — | the override invalidates the signature; **Save a copy** keeps the original's | — | — | — |
| Interface (E11c, E11f, E11g) | Vitest for the dialogs, counts, the permission reasons and the report panel | — | — | — | — | Playwright and WebdriverIO: protect, sanitise, open the report; axe on each screen |
| Public key (E11e) | each recipient algorithm per platform | one `pubsec` file per algorithm | pyHanko and lamina read each other's files | — | CMS `EnvelopedData` parser | WebdriverIO: a temporary Keychain, SoftHSM2 |

Across every op:

- apply then undo leaves the overlay empty and the next save byte-identical (ADR 0006 §3), except that `SetEncryption` and `RemoveEncryption` are barriers;
- the same state and the same entropy seed save to the same bytes on native and `wasm32`;
- a password never appears in an error message, a toast, a log line, a crash report or a serialised op, and a test greps each for the fixture passwords;
- veraPDF still reports nothing new on a sanitised `pdfa-form.pdf`, except that `StripMetadata` removes the claim and says so.

New fuzz targets add `fixtures/regressions/<target>` folders that join `laminapdf-model`'s regression guard list.

## 10. Decisions on the open questions

1. **Do we ever write R4 or AES-128?** No. E11 writes AES-256 R6 only, with no legacy opt-in in the UI or the SDK. The fixtures show qpdf and pypdf reading R6; E11b's round-trip test shows pdf.js and PDFium do too, and the plan rests on that test rather than on a claim about older readers. R4 derives its key with MD5 and ISO 32000-2 deprecates it, so a legacy writer would add a weaker mode and a test matrix. [04 §13](../../research/pdf/04-interactive-security.md#13-implications-for-laminas-design) allowed an explicit opt-in, and this plan declines it. **Decision for the owner if a user needs R4:** a later todo adds an SDK-only option.
2. **Honour `/P` by default?** Yes, in the interface only. The editor disables the tools the permission bits forbid and names the reason, and opening the file with the permissions password (the owner password) lifts it. The core library does not enforce `/P`: an SDK that refused to edit would be a lock the standard says it isn't, and anyone with the user password can already read and copy. The doc and the dialog say these limits are advisory, as G06 does.
3. **Who may remove security?** The check of §2.2, made in the core. The user password counts only when it grants every permission.
4. **Where do passwords live?** In the worker, as `Secret`s, never in an op log that persists (§2.1).
5. **Do rewrites keep protection?** Yes (§3), so sanitising an encrypted file does not silently strip its encryption.
6. **The plan's location:** `docs/plans/editor/`, as E1 to E10 use.
7. **A sanitiser that fails its own scan.** The save fails with `sanitise-incomplete` and nothing is written (§2.3). A user who is told "15 items removed" has to be right.
8. **Public-key writing.** Behind `experimental_pubsec` until the owner decides, after E11e, whether it ships on. **Decision for the owner, after E11e.**
