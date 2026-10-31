# E3f — Attachments panel, associated files (`/AF`) and portfolios (read)

| Field | Value |
| --- | --- |
| ID | `E3f` |
| Status | `ready` |
| Plan | [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B8b](./B8b-conformance-profiles-and-verapdf.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md), [E3b](./E3b-bookmarks-and-destinations.md), [X3a](../done/X3a-nightly-lane.md) |
| Blocks | [E5c](./E5c-stamps-and-attachments.md), [E11d](./E11d-sanitiser.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3f-attachments-and-associated-files` |
| PR title | `feat(ops): embedded files, associated files and portfolio reading` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Attach, extract and delete embedded files (EmbeddedFiles name tree), attach associated files with `/AFRelationship` to the catalog, pages or annotations (PDF 2.0 / PDF/A-3, e.g. Factur-X), and read portfolios (`/Collection`) with a file list.

## Context

- **Design (Paper):** [O14 Attachments panel](../../design/paper/pages-navigation-extraction.md#o14-attachments-panel). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §6 Embedded files, associated files, portfolios](../../research/pdf/04-interactive-security.md#6-embedded-files-associated-files-portfolios)
- ISO 32000-2 §7.11 (file specifications), §14.13 (associated files), §12.3.5 (collections)

## Scope

**In scope**

- Ops: `AttachFile { name, blob, mime, description, af: Option<AfTarget>, relationship }`, `DeleteAttachment`, `UpdateAttachment`
- `extract_attachment(id) -> bytes` and listing incl. FileAttachment annotations (read)
- PDF/A-3 helper: MIME `/Subtype`, `/Params` (size, dates, checksum), `/AFRelationship`
- Portfolio view (collection schema, files) read-only
- Attachments panel: add (picker/drop), save out, delete, show relationship

**Out of scope** (and which todo covers it)

- FileAttachment annotations (create) → [E5c](E5c-stamps-and-attachments.md)
- Removing all attachments as sanitisation → [E11d](E11d-sanitiser.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.05` attach / extract / delete embedded files; `J.06` portfolios / collections; `H.09` extract attachments; `M.09` associated files and structured metadata
- **Crates and packages:** `lamina-model` (file specs, name-tree writer from [E2b](E2b-page-structure-ops.md)), `lamina-ops`, `@laminapdf/ui`
- **`Op` variants:** `AttachFile`, `UpdateAttachment`, `DeleteAttachment` (class `Document`)
- **`SaveConstraint`:** `Free` → incremental; signed: adding or changing attachments is class `Document`, a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → warn and gate (override names the signatures). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock`. Deleting an attachment leaves it in earlier revisions: notice offers full rewrite
- **Web vs desktop:** identical; desktop saves extracted files to disk via dialog, web downloads
- **Server features:** none (attachments never leave the device)
- **Capability-matrix rows to flip:** `J.05` → `round-trip` (core, pdf, web, desktop); `J.06` → `read` (core, pdf, web, desktop); `H.09` → `write` (core, pdf, web, desktop); `M.09` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] File spec writer with `/UF` and `/F`
- [ ] Embedded file stream with `/Params` and checksum
- [ ] AF arrays on targets
- [ ] Collection reader
- [ ] Panel
- [ ] Factur-X example

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] veraPDF passes PDF/A-3b on a fixture after attaching an XML with `/AFRelationship /Alternative`
- [ ] Attachments extracted equal the originals byte-for-byte (SHA-256)
- [ ] pdf.js lists attachments added by lamina

## Post-merge checks

None.

## Tests

- Unit and round-trip
- veraPDF nightly
- e2e on both shells

## Docs to update

- API docs: attachments
- Guide: Factur-X/ZUGFeRD
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Large attachments in WASM memory: stream from `ByteSource` and blob store; enforce `Limits`
- Attached executables: show a warning on extract (never open them)

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
