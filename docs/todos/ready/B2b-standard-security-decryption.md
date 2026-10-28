# B2b — Standard security handler decryption (read only)

| Field | Value |
| --- | --- |
| ID | `B2b` |
| Status | `ready` |
| Plan | [pdf-library §7 B2](../../plans/pdf-library.md#b2-document-model), requirement [§4 F7](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2a](../done/B2a-limits-owned-document.md) |
| Blocks | [B3c](./B3c-save-planner-and-roundtrip.md), [B6b](./B6b-lamina-pdf-package.md), [B7a](./B7a-docs-site.md), [B9b](./B9b-text-extraction-core.md), [C4c](./C4c-remote-files-encryption-privacy.md), [E11b](./E11b-encrypt-on-write.md) |
| ADRs | Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (encrypted files open; password in `OpenOptions`), [0001](../../decisions/0001-core-language-and-licence.md) (no C crypto, permissive licences), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (new crypto crates through cargo-deny) |
| Branch | `feat/b2b-decryption` |
| PR title | `feat(core): standard security handler decryption (R2–R6, crypt filters)` |
| Size | L (≈ 800–1,200 lines incl. tests; fixture generation script separate) |
| PR | — |

## Goal

Let `Document::open` read files encrypted with the standard security handler when given the right password (or none, for the common empty-user-password case), so readers and the library can open password-protected PDFs. Today such files open but every string and stream is ciphertext (`is_encrypted()` only reports it). Writing encrypted files stays out of scope until full-editor E11; incremental updates on an encrypted base keep refusing (`EncryptedBase`).

## Context

- [Research 04 §7](../../research/pdf/04-interactive-security.md#7-encryption-76): what is encrypted and what is exempt (strings in `/Encrypt`, trailer `/ID`, strings inside streams, xref streams, XMP when `/EncryptMetadata false`, signature `/Contents`); crypt filters (`/V2`, `/AESV2`, `/AESV3`, `/Identity`, per-stream `/Crypt`); AES layout (16-byte IV ‖ CBC, PKCS#5).
- [04 §7.2](../../research/pdf/04-interactive-security.md#72-standard-security-handler-revisions): Algorithm 2 key derivation for R2–R4 (PAD, 50 MD5 rounds for R3+, `FF FF FF FF` when `/EncryptMetadata false` at R4), per-object keys (`sAlT` for AES), Algorithms 4–7 (user/owner checks), R6 Algorithm 2.A/2.B (SASLprep, SHA-256/384/512 loop, `/UE`/`/OE`, `/Perms` check).
- [04 §7.3](../../research/pdf/04-interactive-security.md#73-verified-example-pypdf-6170-aes-256-empty-user-password) verified pypdf AES-256 example; [§7.4](../../research/pdf/04-interactive-security.md#74-permission-bits-p) permission bits (read `/P` as 32-bit, only defined bits); [§7.5](../../research/pdf/04-interactive-security.md#75-user-vs-owner-passwords-why-owner-only-protection-isnt-security); [§7.7 PDFex](../../research/pdf/04-interactive-security.md#77-pdfex-müller-et-al-acm-ccs-2019) (mixed plaintext/ciphertext).
- ISO 32000-2 §7.6 (and ISO 32000-1 §7.6.3 for R2–R4).
- Code: [`lamina-cos/src/reader/mod.rs`](../../../crates/lamina-cos/src/reader/mod.rs) (`is_encrypted`, `resolve`, `decode_stream`), [`reader/objstm.rs`](../../../crates/lamina-cos/src/reader/objstm.rs), [`lamina-syntax/src/filters.rs`](../../../crates/lamina-syntax/src/filters.rs).

## Scope

**In scope**

- `lamina-syntax::crypt` module: RC4 (own small impl or a pure-Rust crate), AES-128/256-CBC decrypt (RustCrypto `aes` + `cbc`), MD5 and SHA-2 (RustCrypto `md-5`, `sha2`), SASLprep (`stringprep` crate or a minimal RFC 4013 table) — all pure Rust, licences checked with cargo-deny.
- Parse `/Encrypt` (`/Filter /Standard`, `/V` 1/2/4/5, `/R` 2/3/4/5/6, `/Length`, `/CF`, `/StmF`, `/StrF`, `/EFF`, `/O`, `/U`, `/OE`, `/UE`, `/Perms`, `/P`, `/EncryptMetadata`).
- Password authentication: user then owner (R2–R4: Algorithms 6/7; R5: single SHA-256; R6: Algorithm 2.B); empty password tried first; `/Perms` validated for R6 (mismatch → warning `perms-mismatch`, not an error).
- Decryption of strings and streams on resolve, per-object keys for R2–R4, the file key for R5/R6; `/Identity` filter; per-stream `/Crypt` filter with `/Name`; `/EncryptMetadata false` leaves the XMP metadata stream plaintext; exempt objects (the `/Encrypt` dict, xref streams, trailer `/ID`, signature `/Contents` of `/Type /Sig` dicts) are never decrypted; strings inside object streams are not decrypted twice.
- `OpenOptions::with_password(bytes or &str)`; `Document::permissions()` (typed `Permissions` from `/P`, masked per §7.4), `Document::encryption()` summary (V, R, cipher, key bits, `encrypt_metadata`), `Document::is_authenticated()` and which password matched (`User`/`Owner`).
- Errors: `Error::PasswordRequired` (non-empty user password and none given), `Error::WrongPassword`, `Error::UnsupportedSecurityHandler { filter }` (public-key handler, unknown `/V`/`/R`), with codes in `fixtures/codes.json`.
- Encrypted fixtures for every revision with a reproducible generator, in `fixtures/encrypted/` with their own facts file `fixtures/encrypted/expected.json` (the single source of truth for them). [B2c](./B2c-lamina-model-corpus-fuzz.md)'s corpus regeneration only touches `fixtures/corpus/`, so the two todos stay parallel and neither overwrites the other.

**Out of scope** (and which todo covers it)

- Writing encrypted files, changing or removing encryption, permission enforcement on write: full-editor E11.
- Public-key security handler (§7.6.5), ISO/TS 32003 AES-GCM and 32004 integrity: later (reported as `UnsupportedSecurityHandler`).
- Password UI: web C4, desktop D-rounds.
- Exposing passwords through WASM/TS: [B6b](./B6b-lamina-pdf-package.md).

## Tasks

- [ ] Load `/rust-best-practices`; read 04 §7 and ISO 32000-2 §7.6 Algorithms 1–2.B
- [ ] Add crypto deps to the workspace (`aes`, `cbc`, `md-5`, `sha2`, RC4 impl); run `cargo xtask deny`; verify they build for wasm32/iOS/Android (`cargo xtask portability`)
- [ ] `crates/lamina-syntax/src/crypt/` : `rc4.rs`, `aes.rs` (CBC decrypt, IV handling, padding check tolerant: bad padding → keep data, warning `aes-padding-invalid`), `key.rs` (Algorithm 2, 2.A, 2.B), `auth.rs` (Algorithms 4–7, R5, R6), `perms.rs`
- [ ] `crates/lamina-cos/src/reader/encrypt.rs`: parse `/Encrypt` (resolving an indirect `/Encrypt`), build a `SecurityHandler { file_key, stm_filter, str_filter, eff, encrypt_metadata, … }`
- [ ] Hook decryption into `resolve` (strings in the object, recursively, excluding exempt objects) and `decode_stream` (before the filter chain; `/Crypt` in the chain picks a named filter or `/Identity`)
- [ ] Object streams: decrypt the container once; members not decrypted again
- [ ] `OpenOptions::with_password`, `Document::permissions()`, `encryption()`, error variants and codes
- [ ] `fixtures/encrypted/` generator: `tools/xtask` subcommand `cargo xtask fixtures encrypted` (or a script under `fixtures/encrypted/make.sh`) that uses qpdf (`--encrypt user owner 40|128|256 --use-aes=y|n --cleartext-metadata`) and pypdf to produce from one plaintext source (with Info strings, an outline and text): R2 RC4-40, R3 RC4-128, R4 RC4-128, R4 AES-128, R4 AES-128 with `/EncryptMetadata false`, R5 AES-256 (pypdf), R6 AES-256; each with (a) empty user password and (b) user + owner passwords; one with an `/Identity` stream filter; record commands and tool versions in `fixtures/README.md`
- [ ] `fixtures/encrypted/expected.json` for the encrypted set, written by a password-aware facts script next to the generator (`fixtures/encrypted/facts.py`, pinned pypdf; passwords per file, page count, outline titles, Info strings, `qpdf --show-encryption` permissions) — facts from pypdf/qpdf, never lamina; nothing is written to `fixtures/corpus/expected.json`
- [ ] Fuzz seeds: whichever of B2b and B2c merges **second** adds `fixtures/encrypted/*.pdf` to the `open` fuzz target's seed script and makes the target try the empty password and each fixture's password from `fixtures/encrypted/expected.json`. If B2c is already merged when this PR opens, do it here; otherwise B2c does it
- [ ] Remove "Decryption arrives in Round B2" from ADR 0003's tolerance table (amend the row: encrypted file opens and decrypts given a password)

## Acceptance criteria

- [ ] Every encrypted fixture (R2, R3, R4 RC4, R4 AES-128, R5, R6; user, owner and empty passwords) opens and matches the plaintext original's page count, outline titles, Info strings and decoded content streams byte for byte
- [ ] Opening with the owner password and with the user password both succeed; `matched_password()` reports which
- [ ] A wrong password fails with `Error::WrongPassword` (code `wrong-password`); a missing one with `Error::PasswordRequired`
- [ ] `/EncryptMetadata false` fixture: XMP stream read as plaintext, everything else decrypted
- [ ] Known-answer unit tests for Algorithm 2 (R2–R4) and 2.B (R6) from the 04 §7.3 example and qpdf's test vectors
- [ ] `permissions()` matches `qpdf --show-encryption` for every fixture
- [ ] `incremental_update` on an encrypted base still fails with `EncryptedBase`
- [ ] No new `unsafe`; crypto crates pass cargo-deny and the portability check

## Post-merge checks

None.

## Tests

- Unit: `lamina-syntax` crypt known-answer tests (RC4, AES-CBC, MD5 rounds, 2.B loop termination).
- Integration: `crates/lamina-cos/tests/encryption.rs` over `fixtures/encrypted/` vs plaintext originals; wrong/missing password; `/Identity` and `/Crypt` per-stream; mixed plaintext objects (PDFex shape) behave per spec (plaintext strings in an encrypted doc are decrypted as garbage → documented, no panic).
- Facts: `fixtures/encrypted/expected.json` (page count, outline titles and Info from pypdf with the password); `crates/lamina-cos/tests/encryption.rs` reads it.
- Fuzz: encrypted fixtures seed the `open` target, added by whichever of B2b/[B2c](./B2c-lamina-model-corpus-fuzz.md) merges second (see Tasks).

## Docs to update

- [ ] [pdf-library §10 B2](../../plans/pdf-library.md#10-to-dos): tick the decryption and encrypted-fixtures to-dos
- [ ] ADR 0003 tolerance table row "Encrypted file"
- [ ] `fixtures/README.md` (provenance, generator commands, `fixtures/encrypted/expected.json` and its script), `crates/lamina-cos/README.md`

## Risks and open questions

- RC4/MD5 are broken primitives; we only *read* them. Make sure no API offers them for writing.
- R5 was never standardised; support read-only, document it.
- SASLprep crate choice (licence, size in WASM); if too heavy, implement the RFC 4013 subset needed and test against pypdf.
- AES padding errors in real files: tolerate (keep data, warn) rather than fail, consistent with ADR 0003; confirm with the corpus.
- WASM size: crypto adds to the base module; measure in [B6a](./B6a-lamina-wasm-and-adr-0016.md).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
