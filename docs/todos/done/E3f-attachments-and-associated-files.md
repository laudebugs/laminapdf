# E3f — Attachments panel, associated files (`/AF`) and portfolios (read)

| Field | Value |
| --- | --- |
| ID | `E3f` |
| Status | `done` |
| Plan | [E3 plan §2.1](../../plans/editor/e3-navigation-attachments-inspection.md#21-ops) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B8b](./B8b-conformance-profiles-and-verapdf.md), [E2i](./E2i-signed-gating-and-close.md), [E2j](./E2j-page-tree-and-tree-writers.md), [E2p](./E2p-source-documents-and-copier.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md), [X3a](./X3a-nightly-lane.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3f-attachments-and-associated-files` |
| PR title | `feat(ops): embedded files, associated files and portfolio reading` |
| Size | L (400–800): three ops, file specifications at every level, the portfolio view and the Attachments tab |
| PR | [#706](https://github.com/laudebugs/laminapdf/pull/706) |

## Goal

Attach, extract and delete embedded files (EmbeddedFiles name tree), attach associated files with `/AFRelationship` to the catalog, pages or annotations (PDF 2.0 / PDF/A-3, e.g. Factur-X), and read portfolios (`/Collection`) with a file list.

## Context

- **Design (Paper):** [O14 Attachments panel](../../design/paper/pages-navigation-extraction.md#o14-attachments-panel). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §6 Embedded files, associated files, portfolios](../../research/pdf/04-interactive-security.md#6-embedded-files-associated-files-portfolios)
- ISO 32000-2 §7.11 (file specifications), §14.13 (associated files), §12.3.5 (collections)

## Scope

**In scope**

- Ops: `AttachFile { name, file: BlobId, mime, description, created, modified, attach_to, relationship }`, `UpdateAttachment`, `DeleteAttachment` ([E3 plan §2.1](../../plans/editor/e3-navigation-attachments-inspection.md#21-ops)); the bytes live in [E2p](../done/E2p-source-documents-and-copier.md)'s blob store, and the dates come in the op
- PDF/A warnings `pdfa-attachment-forbidden` (PDF/A-1, PDF/A-2) and `pdfa-attachment-incomplete` (PDF/A-3 without `/AF` or a modification date); `/Version` is never raised
- `DeleteAttachment` reports `AttachmentStillHeld` when a FileAttachment annotation keeps the file
- UR3 `/EF` rights mapped to the attachment ops in `would_violate`, by op ([E3 plan §6](../../plans/editor/e3-navigation-attachments-inspection.md#6-saving-and-the-save-constraint))
- `doc.attachments()` and `attachment_bytes(id)` (size checked against `Limits`, checksum compared), listing FileAttachment annotations too (read)
- PDF/A-3 helper: MIME `/Subtype`, `/Params` (size, dates, checksum), `/AFRelationship`
- Portfolio view (collection schema, files) read-only
- Attachments panel: add (picker/drop), save out, delete, show relationship

**Out of scope** (and which todo covers it)

- FileAttachment annotations (create) → [E5c](../ready/E5c-stamps-and-attachments.md)
- Removing all attachments as sanitisation → [E11d](../ready/E11d-sanitiser.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.05` attach / extract / delete embedded files; `J.06` portfolios / collections; `H.09` extract attachments; `M.09` associated files and structured metadata
- **Crates and packages:** `laminapdf-model` (file specs, name-tree writer from [E2j](../done/E2j-page-tree-and-tree-writers.md)), `laminapdf-ops`, `@laminapdf/ui`
- **`Op` variants:** `AttachFile`, `UpdateAttachment`, `DeleteAttachment` (class `Document`)
- **`SaveConstraint`:** `Free` → incremental; signed: adding or changing attachments is class `Document`, a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → warn and gate (override names the signatures). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock`. Deleting an attachment leaves it in earlier revisions: notice offers full rewrite
- **Web vs desktop:** identical; desktop saves extracted files to disk via dialog, web downloads
- **Server features:** none (attachments never leave the device)
- **Capability-matrix rows to flip:** `J.05` → `round-trip` (core, pdf, web); `J.06` → `read` (core, pdf, web); `H.09` → `read` (core, pdf, web); `M.09` → `round-trip` (core, pdf, web)

## Tasks

- [x] File spec writer with `/UF` and `/F`
- [x] Embedded file stream with `/Params` and checksum
- [x] AF arrays on targets
- [x] Collection reader
- [x] Panel
- [x] Factur-X example

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [x] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [x] veraPDF passes PDF/A-3b on a fixture after attaching an XML with `/AFRelationship /Alternative`
- [x] Attachments extracted equal the originals byte-for-byte (SHA-256)
- [x] pdf.js lists attachments added by lamina

## Post-merge checks

None.

## Tests

- Unit and round-trip
- veraPDF nightly
- Playwright: attach, save out and compare the bytes

## Docs to update

- API docs: attachments
- Guide: Factur-X/ZUGFeRD
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Large attachments in WASM memory: stream from `ByteSource` and blob store; enforce `Limits`
- Attached executables: show a warning on extract (never open them)

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **Model** (`laminapdf-model/src/attachments/`): `Document::attachments()` lists each file specification once with its holders (`AttachmentHolder`: the `/EmbeddedFiles` tree, the catalog's, pages' and annotations' `/AF`, a `FileAttachment`'s `/FS`); `attachment_bytes(id)` checks the stated size and the stored length against `Limits::max_stream_decoded` before decoding and compares the MD5 `/CheckSum` (`Checksum::Matches`, `Mismatch`, `Absent`); `collection()` reads `/Collection`'s schema (ordered by `/O`), sort, view, initial file and each file's `/CI` values. A direct file specification gets a path ID (`AttachmentId`, kind `e`). `AfRelationship` moved here from `laminapdf-build` (re-exported there), with `EncryptedPayload`, `FormData` and `Schema` added; the façade's `Attachment` stays the builder's, so the model's file is `AttachedFile`. One writer, `write_embedded_file`, writes the stream and the specification for the builder and the ops, so the builder's attachments now carry `/CheckSum` too (the `pdf_a3_invoice_with_xml` reference PDF changed). `set_names_entry` and `set_at_path` join `set_catalog_entry`; the named destinations' tree write uses `set_names_entry` now.
- **Ops:** `AttachFile` (the tree key gets ` (2)`, ` (3)`, … when the name is taken), `UpdateAttachment`, `DeleteAttachment`; class `Document`, no rewrite, `Op::embedded_file_right()` (`EmbeddedFileRight`). `unknown-attachment` is the one new error; `attachment-still-held` is a façade edit warning (`Applied::attachments_held`), `pdfa-attachment-forbidden` and `pdfa-attachment-incomplete` are plan-time conformance warnings over `PendingChanges::attached`, so an undone attachment warns of nothing. `pdfa-attachment-incomplete` also counts a missing MIME type, which ISO 19005-3 §6.8 asks for beside `/AF` and `/ModDate`.
- **Signed files and UR3:** `would_violate` judges an attachment op by the usage-rights signature's `/EF` rights instead of class `Document`; at save time, pending `Document` changes that are all attachment ops are judged the same way (`PendingChanges::document_besides_attachments`), so the verdict before an edit and the save agree. The target counts too, strictly (the lead's decision): attaching to a page changes the page, which no usage right covers, so it is judged as any other `Document` change; attaching to an annotation also needs `/Annots /Modify`. No signed fixture grants `/EF` (pyHanko can't sign usage rights), so the `/EF` cases are hand-built files in `crates/laminapdf-save/tests/attachments.rs`.
- **Review fixes:** `attachment-too-large` refuses a file over `max_stream_decoded`, which could never be read back out, and `attachment_limit()` / `attachmentLimit()` give the limit the reader's messages name (it was a hard-coded 512 MB). Tree keys compare as decoded text, so a UTF-16BE key and the same name in PDFDocEncoding clash, and a key listed twice keeps both files. Direct annotations hold files too, and deleting a file rewrites their `/AF` in the page's `/Annots`. A changed attachment is checked against PDF/A-3 again. The disclosure's embedded files come from the same walk as the listing, widened to `/AF` on every object and specifications nothing holds; direct annotations' `/AF`, which the old scan missed, now count. The viewer's worker still gives attached bytes to its text-capable copy: that copy applies every edit the model applies, the `attachFile` op included.
- **Library:** façade `attachments()`, `attachment_bytes()`, `collection()`, `attach_file()`; `add_blob()` left the `content-edit` feature. WASM: `attachments()`, `attachmentBytes()`, `collection()` and `addBlob()` in every module. SDK: `attachments()`, `attachmentBytes()`, `collection()`, `attachFileOp()`, `attachFile()`, `updateAttachment()`, `deleteAttachment()` and the transaction's update and delete. The plan grouped the TypeScript edits as `doc.attachments`; they follow the SDK's flat methods instead, as the XMP edits do.
- **Factur-X:** the `attach_factur_x` example (Rust and TypeScript write the same bytes) attaches `factur-x.xml` as the document's `Alternative` and records the four `fx:` properties, which E3e's extension schemas declare; `cargo xtask verapdf` passes it as PDF/A-3b (run locally: 16 validations over 14 files pass). The recipe is "Make an invoice Factur-X"; the "Attachments" guide covers the rest.
- **Round trip:** `attachFile` is a proving op on every fixture (J.05, M.09), and the attachment fixtures are edited (attached to a page, renamed, deleted while an annotation holds it). pdf.js lists lamina's attachments: `getAttachments()` gives the keys and `getAttachmentContent(key)` the bytes, whose SHA-256 must match; PDFium compares names and SHA-256 too. `cargo xtask roundtrip` passed locally (qpdf, pdf.js 6.4.299, pypdfium2 5.14.0 on 5,793 outputs).
- **Web reader:** the rail's Attachments tab, as the [area document](../../design/paper/pages-navigation-extraction.md#o14-attachments-panel) records, with its differences from O14: the details sit under the list rather than in the docked inspector; Belongs to stays text in Organise, since no op changes what a file is associated with (Add file… associates new files with the document); the history note shows after a deletion. The viewer's worker keeps attached bytes as blobs, and its edit log replays them after a restart, as it does source documents.
- **Sizes:** the reader's startup script measures 287,705 bytes (the tab, the engine's list and the client's calls; about 540 over its waiver), so the waiver rises to 288,200. The WebAssembly modules grow by about 37 KB gzipped each over what CI measured for `main` (base 952,238 → 991,224; extract 1,386,607 → 1,424,491; text 2,072,609 → 2,110,708): the three ops, the listing, the portfolio reader, their serde code and DTOs, and the usage-rights check of a deletion's pages and annotations, in every module since attachments are part of the document. The budgets rise to 992,800, 1,426,000 (glue 6,000) and 2,112,300, the SDK's bundle to 27,100 and its worker file to 25,200, recorded in ADR 0016 §6. With `main` merged (E5f's comment summary), the text module measures 2,127,571, the SDK's JS 28,565 and its worker file 26,686, so those budgets rise to 2,129,000, 28,700 and 26,800.
- **Fixtures:** `fixtures/attachments/make.py` writes `associated.pdf` (a tree, a page `/AF`, a `FileAttachment` annotation, a wrong checksum) and `portfolio.pdf` (a `/Collection` with a schema, a sort and items).

