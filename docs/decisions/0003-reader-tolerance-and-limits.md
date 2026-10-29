# 0003 — Reader tolerance policy and `Limits` defaults

- Status: Accepted
- Date: 2026-10-28

## Context

A strict ISO 32000 parser rejects a large minority of the files that every mainstream viewer opens, so lamina must read tolerantly ([research ch. 01 §11.2](../research/pdf/01-file-structure.md#112-when-the-normal-path-fails-reconstruction)). But tolerance that is not written down drifts: the spike's first differential run disagreed on 12% of damaged files, and *every* disagreement was an unwritten policy choice (missing kids, missing header, which trailer wins, duplicate entries, integer overflow, truncated deflate, hybrid free entries). The same spike found that every number read from a file is an attacker-controlled allocation size ([ch. 07 §4](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned)). Repairs are also security-relevant: signature attacks rely on readers that silently repair ([ch. 04 §11.3](../research/pdf/04-interactive-security.md#113-guidelines-for-building-a-safe-parser-lamina)), and parser differentials let one file show different content in different viewers ([ch. 01 §13](../research/pdf/01-file-structure.md#13-security-relevant-parsing-pitfalls)).

The policy already exists in code, ported from the spike and fixed in review of PR #8: `crates/lamina-cos/src/lib.rs` (crate docs), `reader/mod.rs`, `reader/xref.rs`, `reader/repair.rs`, and `lamina-syntax`'s `parser.rs`, `lexer.rs` and `filters.rs`. Only two limits are configurable (`OpenOptions::max_decoded_size`, the constant `MAX_DEPTH`); the rest are private constants. qpdf added page-tree depth limits, a 25-filter cap and a global limits API in 2026 for the same reasons ([report: editing](../research/architecture/app-and-package-architecture.md#editing-is-an-op-log-over-a-lazy-object-graph-and-signatures-and-redaction-govern-saving)).

## Decision

### 1. Tolerance rules (what the reader does today; changing one needs an amendment to this ADR)

| Situation | Behaviour | Where |
| --- | --- | --- |
| Reference to a missing or free object, or whose generation differs from the xref entry's | Resolves to `null` (§7.3.10). Not cached under the stale generation | `Document::load`, `XrefEntry::matches_gen` |
| A `null` value | Same as absent (§7.3.7): an inheritable page attribute that is `null` keeps the inherited value; a `null` `/Contents` or array element contributes nothing | `Inherited::overlay`, `page_content` |
| `%PDF-` header | Optional. Searched in the first 1,024 bytes; absent → warning, parsing continues | `Document::open` (`OpenOptions`) |
| Stored xref | Used only if `startxref` parses, the `/Prev` chain loads, the trailer has `/Root`, **every** in-file entry points at a matching `num gen obj` header, and `/Root` resolves to a dictionary. Anything else triggers repair | `Document::open`, `entries_are_valid` |
| Repair | Scan all `num gen obj` headers. Per object number, the definition **last in the file** wins (an object-stream member counts at its container's offset, then its index); a parseable definition beats an unparsable one. All trailers and xref-stream dictionaries merge in file order; the newest `/Root` pointing at a dictionary wins, else the newest `/Type /Catalog`. `/Size` is recomputed | `reader/repair.rs` |
| xref chain | Newest section wins; within one classic table, a later subsection wins. `/Prev` loops are a load failure (→ repair) | `xref::load`, `parse_table` |
| Hybrid files (§7.5.8.4) | The classic table's in-use entries win; `/XRefStm` fills numbers the table omits or marks free; a broken `/XRefStm` is ignored | `xref::load` |
| xref-stream entry of unknown type | Treated as free (resolves to `null`) | `parse_stream_entries` |
| Object-stream index stale | Fall back to a search by number; absent → `null` | `load_compressed` |
| Wrong or unresolvable `/Length` | Stream extent recovered by scanning for `endstream` | `Parser::indirect_object` |
| Page tree | Iterative walk. Kids that are missing, unparsable or not dictionaries are skipped; a `/Kids` that is not an array is skipped; missing `/Pages` → 0 pages. A node without `/Type` is a tree node if it has `/Kids`. `/Count` is ignored (the walk is authoritative). A node reached twice is `Error::ReferenceCycle` | `Document::pages` |
| Integer too large for `i64` | Becomes a `Real` (§7.3.3); integers up to `i64::MAX` stay exact | `lexer::parse_number` |
| Truncated Flate data or bad Adler-32 | Partial output is kept if non-empty; no output → `Error::Decompress` | `filters::flate_decode` |
| Unsupported filter | The document opens; only decoding that stream fails (`Error::UnsupportedFilter`) | `filters::decode` |
| Encrypted file (standard security handler, R2–R6) | Opens and decrypts given its user or owner password (the empty user password is tried when none is given); strings and streams are decrypted as objects load. Exempt: the `/Encrypt` dictionary, xref streams, the trailer `/ID`, a signature's `/Contents`, object-stream members (their container is decrypted once), and the XMP metadata stream when `/EncryptMetadata` is false. AES data that isn't `IV ‖ whole blocks` is kept as stored and bad padding is kept unstripped (`aes-padding-invalid`); a plaintext object spliced into an encrypted file (PDFex) is decrypted like any other, i.e. to garbage. An R5/R6 `/Perms` that disagrees with `/P` → `perms-mismatch`. No password or a wrong one → `PasswordRequired` / `WrongPassword`; another handler (public key) or an unknown `/V`/`/R`/`/CFM` → `UnsupportedSecurityHandler` | `reader/encrypt.rs`, `Document::load_in_file`, `lamina_syntax::crypt` |

**Rejected** (errors, never repaired): empty input or no `n g obj` anywhere (`NotPdf`); no catalog even after repair (`MissingRoot`); an encrypted file without the right password (`PasswordRequired`, `WrongPassword`) or with a handler lamina can't decrypt (`UnsupportedSecurityHandler`); reference and page-tree cycles (`ReferenceCycle`); exceeding a **hard** limit (§3) outside a tolerant path. Soft limits, and hard limits hit inside a tolerant path, degrade with a warning instead (§3). A malformed object is an error *for the call that resolves it*, not for `open`; errors are not cached. **Writing is strict:** an incremental update refuses a repaired base (`RepairedBase`) or an encrypted one (`EncryptedBase`, until full-editor E11), never shrinks `/Size`, and appends a section of the same kind as the newest.

### 2. Warnings: stable, enumerated codes

Every repair or substitution the caller might care about is a `Warning`, never silent and never an error ([code organization §5](../guidelines/code-organization.md#5-error-handling-by-layer)). `Warning` stays `#[non_exhaustive]`; each variant has a `code()` returning a kebab-case string that is part of the public API in Rust, WASM and TypeScript ([code organization §4](../guidelines/code-organization.md#4-public-api-surface-and-stability)). Codes are never renamed or reused; retired codes stay reserved. Variants carry a byte offset or object number where one exists.

| Code | Raised when | Status |
| --- | --- | --- |
| `missing-header` | No `%PDF-x.y` in the first 1,024 bytes | exists |
| `xref-repaired` | The stored xref was rebuilt by scanning | exists |
| `xrefstm-ignored` | A hybrid file's `/XRefStm` could not be read | exists |
| `stream-length-recovered` | `/Length` was wrong or unresolvable; `endstream` was scanned for | exists |
| `stale-generation` | A reference's generation did not match its entry (→ `null`) | exists |
| `page-tree-node-skipped` | A kid or `/Kids` was missing, unparsable or the wrong type | exists |
| `page-count-mismatch` | A `/Count` differs from the walked page count | exists |
| `flate-truncated` | Flate output was kept despite truncation or a bad checksum | exists |
| `objstm-index-mismatch` | An object-stream index was stale and a search found the object | exists |
| `object-number-out-of-range` | An entry above `Limits::max_object_number` was dropped | exists |
| `aes-padding-invalid` | AES data was not `IV ‖ whole blocks` (kept as stored) or had invalid PKCS#5 padding (kept unstripped) | exists |
| `perms-mismatch` | An R5/R6 `/Perms` does not decrypt to the document's `/P` and `/EncryptMetadata` | exists |
| `warnings-truncated` | More than `Limits::max_warnings` were raised; the rest were dropped | exists |
| `invalid-box` | A page boundary box is not four numbers; the view reads it as absent | exists (B2c) |
| `invalid-rotate` | `/Rotate` is not a multiple of 90; the view reads 0 | exists (B2c) |
| `name-tree-node-skipped` | A name- or number-tree node, kid or key was malformed, reached twice or deeper than `max_tree_depth` | exists (B2c) |
| `outline-cycle` | An outline item was reached twice; that branch stops | exists (B2c) |
| `outline-item-skipped` | An outline item was missing, not a dictionary or deeper than `max_tree_depth`; it and its subtree are skipped | exists (B2c) |
| `invalid-date` | An Info date is not a valid PDF date; the raw string is kept | exists (B2c) |
| `invalid-page-label` | A `/PageLabels` entry (or part of one) is malformed; that part is dropped, a non-dictionary range falls back to decimal numbering | exists (B2c) |
| `invalid-entry` | An entry a view reads (`/Info`, `/Names`, `/Outlines`, `/Dests`, `/UserUnit`) is unreadable or of the wrong type; it reads as absent or its default | exists (B2c) |

Encryption adds three error codes (B2b), listed in `fixtures/codes.json` with the others: `password-required` (a non-empty user password and none given), `wrong-password` (neither the user nor the owner password) and `unsupported-security-handler` (not `/Standard`, an unknown `/V`/`/R`, or an unknown crypt filter method). All three have status "exists".

Warnings raised lazily (on resolve or decode) are recorded on the `Document` and deduplicated by (code, object). `Document::warnings()` returns what has been seen so far. Missing references (`null`) and integer-to-real promotion are documented behaviour, not warnings: they are common in valid files and too frequent to report.

**Strict mode.** `OpenOptions::default().with_strict(true)` turns every warning into `Error::Strict { code }`. Validators and signature verification (full-editor E10) use it, so a file that needs repair is reported, not silently mended.

### 3. `Limits`

One `Limits` struct, defined in `lamina-syntax` and re-exported by `lamina-cos`, `lamina-model` and `lamina`, replaces `OpenOptions::max_decoded_size` and the private constants in B2. It is `Copy` with **private fields**: callers read them through `const fn` getters (`limits.max_nesting_depth()`) and change them with `const fn with_*` builders on `Limits::default()`, so new limits can be added without breaking anyone. `OpenOptions` (also private fields and builders) holds the `Limits`, `strict` and (B2) the password; `Document::open(source, OpenOptions)` takes it.

Limits are of two kinds:

- **Hard limits** fail the call that hits them with `Error::LimitExceeded { limit: Limit, at: Option<u64> }`, which replaces today's `TooDeep` and `DecompressLimit`. When that call runs inside one of §1's tolerant paths, the path treats it like any other unreadable item and raises its usual warning: the page-tree walk skips the kid (`page-tree-node-skipped`; today `Document::pages` already skips every error except `ReferenceCycle`), the xref loader falls back to repair (`xref-repaired`; today any load error does), and repair skips an object stream it can't decode. A hard limit never changes an interpretation silently, and strict mode turns the fallback's warning into an error. Hit outside a tolerant path (resolving an object, decoding a stream, `open` itself), the error reaches the caller.
- **Soft limits** stop collecting and carry on, always with a warning: the item over the limit is dropped, and reading continues.

| Field | Kind | Default | On exceeding | Today |
| --- | --- | --- | --- | --- |
| `max_nesting_depth` (arrays/dictionaries) | hard | 256 | `LimitExceeded` | `lamina_syntax::MAX_DEPTH`, `TooDeep` |
| `max_stream_decoded` (bytes, one stream) | hard | 64 MiB | `LimitExceeded` | `DEFAULT_MAX_DECODED`, `DecompressLimit` |
| `max_decoded_total` (bytes decoded by one top-level call: `open`, one page's content, one save) | hard | 1 GiB | `LimitExceeded` | new |
| `max_filter_chain` (filters per stream) | hard | 8 | `LimitExceeded` | new (qpdf caps at 25) |
| `max_resolve_depth` (reference chains in flight) | hard | 32 | `LimitExceeded` | `MAX_RESOLVE_DEPTH`, reported as `ReferenceCycle` today |
| `max_page_tree_depth` | hard | 256 | `LimitExceeded`; the walk skips the subtree | new (the walk is iterative, so this bounds work, not stack) |
| `max_tree_depth` (name trees, number trees, outline nesting) | hard | 64 | `LimitExceeded`; views skip the subtree | new, with B2 views |
| `max_string_length` (bytes, one string) | hard | 16 MiB | `LimitExceeded` | new |
| `max_xref_sections` (`/Prev` hops) | soft | 4,096 | stored xref abandoned → repair, `xref-repaired` | `MAX_SECTIONS` (load failure → repair) |
| `max_object_number` | soft | 8,388,607 (ISO 32000-1 Annex C) | entry dropped, `object-number-out-of-range` | new |
| `max_warnings` | soft | 1,000 | later warnings dropped, `warnings-truncated` | new |
| `max_content_operators` (per content stream, with ADR 0023) | hard | 10,000,000 | `LimitExceeded` | reserved |
| `max_xobject_depth` (Form XObjects, patterns, Type 3, with ADR 0023) | hard | 32 | `LimitExceeded` | reserved |
| `max_image_pixels` (when lamina decodes images) | hard | 2^28 | `LimitExceeded` | reserved |

`max_decoded_total` is per call, not per document lifetime, so a long editing session doesn't run out of budget, while a bomb that multiplies through many small streams or a filter chain still stops. Loops over counts read from the file stay bounded by the bytes actually present, which is why no fuel counter is needed for the COS layer. `Limits::unlimited()` exists for trusted input (lamina's own output in tests). `MAX_FILTER_PARAM_DEPTH` (3) stays an internal constant: it bounds a fixed structure, not input size.

### 4. Enforcement

- Each rule in §1 has a named test in `crates/lamina-cos/tests/policy.rs` or `reader_tolerance.rs` (most exist; B2 adds one per new warning code).
- Each hard limit has a pair of tests: at the limit succeeds, one past fails with `LimitExceeded` (as `parse_object_accepts_nesting_at_the_limit` / `…_rejects_nesting_past_the_limit` do now), plus a test that a tolerant path hitting it degrades with its warning. Each soft limit has a test that the item is dropped and its warning raised.
- Warning and error codes are listed once in `fixtures/codes.json`; a Rust test and (from B6) a TS test check that both expose exactly those codes.
- The differential suite ([ADR 0018](0018-test-strategy.md)) compares page count and repaired/failed outcome with pdf.js, PDFium, qpdf and pypdf. A disagreement is triaged as a lamina bug, a deliberate policy difference (listed with the rule from §1 in the suite's exceptions file), or another reader's bug.
- Fuzz targets ([ADR 0018](0018-test-strategy.md)) assert no panic and no hang under default `Limits`.

## Consequences

- Behaviour on damaged files is predictable and reviewable; a policy change is an ADR amendment plus a changed test, not an accident.
- Callers can see exactly what was repaired, and validators can refuse it with strict mode.
- The `OpenOptions` and error enum changes are breaking, but nothing is published yet.
- Lazily recorded warnings mean `warnings()` can grow after `open`; the UI must treat the list as live.
- Defaults are guesses informed by Annex C, qpdf and the research. The nightly corpus run will show if any default rejects legitimate files; raising one is a normal PR that updates this table.

## Alternatives considered

- **Strict by default** (pypdf `strict=True`). Rejects files users expect to open.
- **Report every `null` reference as a warning.** Accurate but noisy; valid files have them.
- **A per-document decompression budget.** Simpler, but a long session viewing many pages would exhaust it.
- **A fuel counter for all parsing.** Unnecessary while every loop is bounded by input bytes; it may come back with content-stream interpretation (ADR 0023).
- **Limits as global statics** (qpdf's global API). Not per document, and awkward in a shared WASM instance.
