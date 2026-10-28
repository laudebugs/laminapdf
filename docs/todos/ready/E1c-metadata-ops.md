# E1c — Metadata ops: Info and XMP in sync (J.01–J.03, J.10)

| Field | Value |
| --- | --- |
| ID | `E1c` |
| Status | `ready` |
| Plan | [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B3c](./B3c-save-planner-and-roundtrip.md), [B6b](./B6b-lamina-pdf-package.md), [E1b](./E1b-xmp-packet-model.md) |
| Blocks | [E1f](./E1f-metadata-ui-capability-page.md), [E2b](./E2b-page-structure-ops.md), [E3e](./E3e-xmp-editor.md) |
| Dependency notes | [E1b](E1b-xmp-packet-model.md), `B3` ([pdf-library §B3](../../plans/pdf-library.md#b3-ops-and-save): ops and save planner), `B6` ([pdf-library §B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf): `lamina-wasm` and `@lamina/pdf`) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1c-metadata-ops` |
| PR title | `feat(ops): metadata ops writing Info and XMP together` |
| Size | L (600–1,500) |
| PR | — |

## Goal

The first user-visible write ops: set title/author/subject/keywords, dates, producer/creator, and remove selected metadata, writing the Info dictionary and the XMP packet together, with save-time `xmp:MetadataDate` and `xmpMM:InstanceID`, PDF 2.0 rules, and PDF/A-aware checks that warn before a save breaks a claimed conformance level.

## Context

- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation): ships list and save rules for E1
- [04 §5 Metadata](../../research/pdf/04-interactive-security.md#5-metadata): Info ↔ XMP sync rules
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1: no ambient inputs (dates are in the payload); §3 undo records
- [ADR 0007](../../decisions/0007-save-planner.md) §2: `Metadata` class is allowed by DocMDP only as an incremental update
- [library B3](../../plans/pdf-library.md#b3-ops-and-save): the proving op `SetInfo` this PR generalises
- ISO 32000-2 §14.3.3 (Info, deprecated keys in 2.0), ISO 19005 (PDF/A metadata rules: Info/XMP equivalence)

## Scope

**In scope**

- `Op::SetMetadata(SetMetadata { patch: MetadataPatch, at: Option<PdfDate> })` replacing/extending `SetInfo`
- `Op::RemoveMetadata(RemoveMetadata { keys: Vec<MetadataKey> })` (J.10)
- Save hook in `lamina-save`: when the overlay touched metadata or any object, update `xmp:MetadataDate`, `xmp:ModifyDate`/`ModDate` and `xmpMM:InstanceID` from values passed in `SaveOptions` (no clocks in core)
- PDF 2.0: XMP authoritative; Info keeps only `CreationDate`/`ModDate` unless the file already has others (kept, flagged)
- PDF/A claim check: keep `pdfaid:`; validate Info/XMP equivalence after the op; `ConformanceWarning` returned by the planner before save
- Bindings in `lamina-wasm` / `@lamina/pdf`: `doc.setMetadata()`, `doc.removeMetadata()`, `doc.metadata()`

**Out of scope** (and which todo covers it)

- UI for editing metadata → [E1f](E1f-metadata-ui-capability-page.md)
- Custom/extension schema editing → [E3e](E3e-xmp-editor.md)
- Stripping all hidden metadata as sanitisation (G.08) → [E11d](E11d-sanitiser.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.01` title, author, subject, keywords; `J.02` creation/modification dates; `J.03` producer and creator; `J.10` remove selected metadata
- **Crates and packages:** `lamina-ops`, `lamina-model` (`xmp` from E1b), `lamina-save` (save hook), `lamina-wasm`, `packages/pdf`
- **`Op` variants:** `SetMetadata`, `RemoveMetadata` (class `Metadata`, `requires_rewrite = false`)
- **`SaveConstraint`:** `Free` → incremental. `AppendOnly`: metadata changes are allowed only as an incremental update; DocMDP P1 flags them (violation) and the UI asks before invalidating; FieldMDP unaffected. `RemoveMetadata` does not make old values disappear from earlier revisions: the op result says so and offers a full rewrite (which on signed files is the override path)
- **Web vs desktop:** identical (core); UI in E1f
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `J.01` → `round-trip` (core, pdf); `J.02` → `round-trip` (core, pdf); `J.03` → `round-trip` (core, pdf); `J.10` → `round-trip` (core, pdf)

## Tasks

- [ ] Define `MetadataPatch` (title, author(s), subject, keywords, creator, producer, creation/modification dates, `Option<Option<_>>` for clear) and `MetadataKey`
- [ ] Implement `apply` for both ops through `CosMut`: Info dict (keep key order), XMP via E1b; create Info and/or XMP when missing
- [ ] Undo records per property; coalescing key for typing into one field (`transact_coalescing`)
- [ ] `SaveOptions { metadata_date, instance_id }` supplied by shells; derive `InstanceID` deterministically from the content hash when the shell passes none, so tests stay deterministic
- [ ] Conformance module: detect PDF/A (`pdfaid:part`) and PDF/UA claims; after applying, check Info/XMP equivalence and required properties; produce `ConformanceWarning` codes
- [ ] Expose `planner.conformance_warnings()` to the UI (E1e/E1f)
- [ ] Bindings and TS types; examples `examples/edit-metadata.rs` and `packages/pdf/examples/edit-metadata.ts`
- [ ] Create the shared signed-fixture set `fixtures/signed/` (approval signature, certification P1, P2, P3, two signatures, FieldMDP lock), generated by a pyHanko script in `fixtures/scripts/` with provenance in `fixtures/README.md`; every later op's classification test reuses it
- [ ] Remove `SetInfo` or keep it as a deprecated alias that serialises as `setMetadata` (decide in review; update `fixtures/ops/v1/`)

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Info and XMP agree after every op on every metadata fixture (test compares both views)
- [ ] veraPDF still passes on edited PDF/A-1b/2b/3b fixtures after a title change; a change that would break the claim (e.g. removing the XMP packet) yields a `ConformanceWarning` before save
- [ ] A PDF 2.0 fixture gets no new deprecated Info keys
- [ ] Determinism: saving the same state twice gives identical bytes on native and wasm32 (ADR 0007 §6)

## Tests

- Unit and property tests in `crates/lamina-ops/tests/metadata.rs`
- Round-trip suite in `crates/lamina/tests/roundtrip_metadata.rs` with readers: lamina, pdf.js (Node harness), PDFium, qpdf `--check`; pypdf metadata differential
- veraPDF on PDF/A fixtures in the nightly lane (external tool, never linked)
- Signed fixtures (approval, certification P1/P2/P3) for gating
- Vitest in `packages/pdf` for the TS API

## Docs to update

- `@lamina/pdf` and `lamina` API docs pages for metadata
- `fixtures/README.md` for new PDF/A and signed fixtures
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Ambiguous mapping of multiple authors (`dc:creator` Seq vs Info `/Author` string): document the joining rule (`; `)
- Some files have conflicting Info and XMP values; which wins on read? Recommendation: XMP for PDF 2.0 and PDF/A, newer `ModDate` otherwise — decide in E1a

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
