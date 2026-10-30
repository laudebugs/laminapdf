# B2b — Standard security handler decryption (read only)

| Field | Value |
| --- | --- |
| ID | `B2b` |
| Status | `done` |
| Plan | [pdf-library §7 B2](../../plans/pdf-library.md#b2-document-model), requirement [§4 F7](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2a](./B2a-limits-owned-document.md) |
| Blocks | [B3c](./B3c-save-planner-and-roundtrip.md), [B6b](./B6b-lamina-pdf-package.md), [B7a](../ready/B7a-docs-site.md), [B9b](./B9b-text-extraction-core.md), [C4c](../ready/C4c-remote-files-encryption-privacy.md), [E11b](../ready/E11b-encrypt-on-write.md) |
| ADRs | Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (encrypted files open; password in `OpenOptions`), [0001](../../decisions/0001-core-language-and-licence.md) (no C crypto, permissive licences), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (new crypto crates through cargo-deny) |
| Branch | `feat/b2b-decryption` |
| PR title | `feat(core): standard security handler decryption (R2–R6, crypt filters)` |
| Size | L (≈ 800–1,200 lines incl. tests; fixture generation script separate) |
| PR | [#22](https://github.com/laudebugs/lamina/pull/22) |

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
- `OpenOptions::with_password(bytes or &str)`; `Document::permissions()` (typed `Permissions` from `/P`, masked per §7.4), `Document::encryption()` summary (V, R, cipher, key bits, `encrypt_metadata`), `Document::is_authenticated()` and which password matched (`User`/`Owner`) (done as `matched_password()`; `is_authenticated()` was dropped in review as redundant: an opened encrypted document is always authenticated, and `is_encrypted()` now means exactly that).
- Errors: `Error::PasswordRequired` (non-empty user password and none given), `Error::WrongPassword`, `Error::UnsupportedSecurityHandler { filter }` (public-key handler, unknown `/V`/`/R`), with codes in `fixtures/codes.json`.
- Encrypted fixtures for every revision with a reproducible generator, in `fixtures/encrypted/` with their own facts file `fixtures/encrypted/expected.json` (the single source of truth for them). [B2c](../done/B2c-lamina-model-corpus-fuzz.md)'s corpus regeneration only touches `fixtures/corpus/`, so the two todos stay parallel and neither overwrites the other.

**Out of scope** (and which todo covers it)

- Writing encrypted files, changing or removing encryption, permission enforcement on write: full-editor E11.
- Public-key security handler (§7.6.5), ISO/TS 32003 AES-GCM and 32004 integrity: later (reported as `UnsupportedSecurityHandler`).
- Password UI: web C4, desktop D-rounds.
- Exposing passwords through WASM/TS: [B6b](../done/B6b-lamina-pdf-package.md).

## Tasks

- [x] Load `/rust-best-practices`; read 04 §7 and ISO 32000-2 §7.6 Algorithms 1–2.B
- [x] Add crypto deps to the workspace (`aes`, `cbc`, `md-5`, `sha2`, RC4 impl; done without `cbc`: the CBC chaining is a few lines over `aes`, and RC4 is our own; `stringprep` for SASLprep; `cpufeatures → libc` allowed by name in `cargo xtask portability`); run `cargo xtask deny`; verify they build for wasm32/iOS/Android (`cargo xtask portability`)
- [x] `crates/lamina-syntax/src/crypt/` : `rc4.rs`, `aes.rs` (CBC decrypt, IV handling, padding check tolerant: bad padding → keep data, warning `aes-padding-invalid`), `key.rs` (Algorithm 2, 2.A, 2.B), `auth.rs` (Algorithms 4–7, R5, R6), `perms.rs`
- [x] `crates/lamina-cos/src/reader/encrypt.rs`: parse `/Encrypt` (resolving an indirect `/Encrypt`), build a `SecurityHandler { file_key, stm_filter, str_filter, eff, encrypt_metadata, … }`
- [x] Hook decryption into `resolve` (strings in the object, recursively, excluding exempt objects) and `decode_stream` (before the filter chain; `/Crypt` in the chain picks a named filter or `/Identity`)
- [x] Object streams: decrypt the container once; members not decrypted again
- [x] `OpenOptions::with_password`, `Document::permissions()`, `encryption()`, error variants and codes
- [x] `fixtures/encrypted/` generator (done as `fixtures/encrypted/make.py`: Python, because it also writes the plaintext source and the two crypt-filter files qpdf can't make; `--allow-weak-crypto` is a general qpdf option, so it goes before `--encrypt`): `tools/xtask` subcommand `cargo xtask fixtures encrypted` (or a script under `fixtures/encrypted/make.sh`) that produces, from one plaintext source (with Info strings, an outline and text), each case with (a) an empty user password and (b) user + owner passwords. Pin qpdf ≥ 11.7 (11.0+ refuses RC4 output without `--allow-weak-crypto`; flags checked against the [qpdf CLI docs](https://qpdf.readthedocs.io/en/stable/cli.html)) and use the long form `qpdf --encrypt --user-password=U --owner-password=O --bits=N [options] -- in.pdf out.pdf`:
  - R2 RC4-40: `--bits=40 --allow-weak-crypto`
  - R3 RC4-128: `--bits=128 --use-aes=n --allow-weak-crypto`
  - R4 RC4-128: `--bits=128 --use-aes=n --force-V4 --allow-weak-crypto` (without `--force-V4` this comes out as R3)
  - R4 AES-128: `--bits=128 --use-aes=y`
  - R4 AES-128 with `/EncryptMetadata false`: `--bits=128 --use-aes=y --cleartext-metadata`
  - R5 AES-256 (the deprecated Acrobat IX handler): `--bits=256 --force-R5`
  - R6 AES-256: `--bits=256`
  - one file with an `/Identity` stream filter (`/StmF /Identity`), written with pypdf or patched from the R4 AES-128 output, since qpdf has no flag for it
  - the generator checks each output's `/V` and `/R` with `qpdf --show-encryption` and fails if a case comes out at the wrong revision; record the commands and tool versions in `fixtures/README.md`
- [x] `fixtures/encrypted/expected.json` for the encrypted set, written by a password-aware facts script next to the generator (`fixtures/encrypted/facts.py`, pinned pypdf; passwords per file, page count, outline titles, Info strings, `qpdf --show-encryption` permissions) — facts from pypdf/qpdf, never lamina; nothing is written to `fixtures/corpus/expected.json`
- [x] Fuzz seeds (this todo owns them, whichever of B2b and [B2c](../done/B2c-lamina-model-corpus-fuzz.md) merges first): the generator also writes the encrypted fixtures to `fuzz/seeds/open/encrypted/` and their passwords, one per line, to `fuzz/seeds/open/passwords.txt`, creating `fuzz/seeds/open/` if it doesn't exist yet. B2c's `open` target seeds from everything under `fuzz/seeds/open/` and tries the empty password and each line of `passwords.txt`, so it picks these up whether it merges before or after this PR. If B2c has already created an empty `passwords.txt`, fill it in rather than replacing it
- [x] New codes: add the warnings `aes-padding-invalid` and `perms-mismatch` and the errors `password-required`, `wrong-password` and `unsupported-security-handler` to `fixtures/codes.json` and to ADR 0003 §2's code table (status "exists"), so `crates/lamina-cos/tests/codes.rs` keeps the enums and the list in step
- [x] Remove "Decryption arrives in Round B2" from ADR 0003's tolerance table (amend the row: encrypted file opens and decrypts given a password)

## Acceptance criteria

- [x] Every encrypted fixture (R2, R3, R4 RC4, R4 AES-128, R5, R6; user, owner and empty passwords) opens and matches the plaintext original's page count, outline titles, Info strings and decoded content streams byte for byte
- [x] `qpdf --show-encryption` on each fixture reports the intended `/V` and `/R` (in particular, the R3 and R4 RC4-128 files differ), checked by the generator
- [x] Opening with the owner password and with the user password both succeed; `matched_password()` reports which
- [x] A wrong password fails with `Error::WrongPassword` (code `wrong-password`); a missing one with `Error::PasswordRequired`
- [x] `/EncryptMetadata false` fixture: XMP stream read as plaintext, everything else decrypted
- [x] Known-answer unit tests for Algorithm 2 (R2–R4) and 2.B (R6) from the 04 §7.3 example and qpdf's test vectors (done as: vectors computed independently with pypdf 6.21.3 for Algorithms 2–7, 2.A/2.B and `/Perms`, FIPS-197 / SP 800-38A for AES, published RC4 vectors; the 04 §7.3 pypdf file is `corpus/enc.pdf`, whose Title now decrypts; qpdf supplies the fixtures rather than unit vectors)
- [x] `permissions()` matches `qpdf --show-encryption` for every fixture
- [x] `incremental_update` on an encrypted base still fails with `EncryptedBase`
- [x] No new `unsafe`; crypto crates pass cargo-deny and the portability check

## Post-merge checks

None.

## Tests

- Unit: `lamina-syntax` crypt known-answer tests (RC4, AES-CBC, MD5 rounds, 2.B loop termination).
- Integration: `crates/lamina-cos/tests/encryption.rs` over `fixtures/encrypted/` vs plaintext originals; wrong/missing password; `/Identity` and `/Crypt` per-stream; mixed plaintext objects (PDFex shape) behave per spec (plaintext strings in an encrypted doc are decrypted as garbage → documented, no panic).
- Facts: `fixtures/encrypted/expected.json` (page count, outline titles and Info from pypdf with the password); `crates/lamina-cos/tests/encryption.rs` reads it.
- Fuzz: this todo writes the encrypted seeds and `passwords.txt` to `fuzz/seeds/open/` (creating the directory if [B2c](../done/B2c-lamina-model-corpus-fuzz.md) hasn't merged yet); B2c's `open` target reads that directory (see Tasks).

## Docs to update

- [x] [pdf-library §10 B2](../../plans/pdf-library.md#10-to-dos): tick the decryption and encrypted-fixtures to-dos
- [x] ADR 0003: tolerance table row "Encrypted file"; §2 code table gains `aes-padding-invalid` and `perms-mismatch`; the codes [B2a](../done/B2a-limits-owned-document.md) shipped but left marked `B2` in the status column become "exists"
- [x] `fixtures/README.md` (provenance, generator commands, `fixtures/encrypted/expected.json` and its script), `crates/lamina-cos/README.md`

## Risks and open questions

- RC4/MD5 are broken primitives; we only *read* them. Make sure no API offers them for writing.
- R5 was never standardised; support read-only, document it.
- SASLprep crate choice (licence, size in WASM); if too heavy, implement the RFC 4013 subset needed and test against pypdf.
- AES padding errors in real files: tolerate (keep data, warn) rather than fail, consistent with ADR 0003; confirm with the corpus.
- WASM size: crypto adds to the base module; measure in [B6a](../done/B6a-lamina-wasm-and-adr-0016.md).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
