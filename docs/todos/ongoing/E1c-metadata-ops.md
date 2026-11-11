# E1c — Metadata ops: Info and XMP in sync (J.01–J.03, J.10)

| Field | Value |
| --- | --- |
| ID | `E1c` |
| Status | `ongoing` |
| Plan | [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B3c](../done/B3c-save-planner-and-roundtrip.md), [B6b](../done/B6b-lamina-pdf-package.md), [B8b](../done/B8b-conformance-profiles-and-verapdf.md), [E1b](../done/E1b-xmp-packet-model.md), [X3a](../done/X3a-nightly-lane.md) |
| Dependency notes | [E1b](../done/E1b-xmp-packet-model.md), `B3` ([pdf-library §B3](../../plans/pdf-library.md#b3-ops-and-save): ops and save planner), `B6` ([pdf-library §B6](../../plans/pdf-library.md#b6-wasm-and-laminapdfsdk): `laminapdf-wasm` and `@laminapdf/sdk`) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1c-metadata-ops` |
| PR title | `feat(ops): metadata ops writing Info and XMP together` |
| Size | L (600–1,500) |
| PR | [#411](https://github.com/laudebugs/laminapdf/pull/411) |

## Goal

The first user-visible write ops: set title/author/subject/keywords, dates, producer/creator, and remove selected metadata, writing the Info dictionary and the XMP packet together, with save-time `xmp:MetadataDate` and `xmpMM:InstanceID`, PDF 2.0 rules, and PDF/A-aware checks that warn before a save breaks a claimed conformance level.

## Context

- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation): ships list and save rules for E1
- [04 §5 Metadata](../../research/pdf/04-interactive-security.md#5-metadata): Info ↔ XMP sync rules
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1: no ambient inputs (dates are in the payload); §3 undo records
- [ADR 0007](../../decisions/0007-save-planner.md) §2: DocMDP P1 allows no changes, P2 allows form filling, page templates and signing, P3 adds annotations, so class `Metadata` is a violation at every certification level; approval signatures without a certification only make the save `AppendOnly` (incremental), and FieldMDP/`/Lock` restricts only the locked fields
- [library B3](../../plans/pdf-library.md#b3-ops-and-save): the proving op `SetInfo` this PR generalises
- ISO 32000-2 §14.3.3 (Info, deprecated keys in 2.0), ISO 19005 (PDF/A metadata rules: Info/XMP equivalence)

## Scope

**In scope**

- `Op::SetMetadata(SetMetadata { patch: MetadataPatch, at: Option<PdfDate> })` added **alongside** `SetInfo` (ADR 0006 §6: adding an op keeps `v`; removing or renaming one would bump it). `SetInfo` stays in the `Op` enum with its v1 wire name (`setInfo`) and meaning, implemented on top of the new Info/XMP writer so it also keeps XMP in sync; it is marked deprecated in rustdoc/TSDoc and the UI never emits it
- `Op::RemoveMetadata(RemoveMetadata { keys: Vec<MetadataKey> })` (J.10)
- Save-time metadata as a classified op, never an unclassified write: when the shell passes `SaveOptions { metadata_date, instance_id }` and the overlay is non-empty, the planner adds a synthetic `Op::UpdateSaveMetadata { metadata_date, instance_id }` (class `Metadata`, `requires_rewrite = false`) to the pending changes and runs it through `ModificationClass`/`would_violate` like any other op (no clocks in core). It sets `xmp:MetadataDate`, `xmp:ModifyDate`/Info `ModDate` and `xmpMM:InstanceID`. If it would add a violation or change `constraint.kind()` (e.g. a form fill or a signature on a DocMDP P2/P3-certified file, where `Metadata` is not allowed), the planner leaves it out, writes nothing to Info or `/Metadata`, and reports `SaveReport::metadata_not_updated` so the shell can say why. It is not recorded in the undo history
- PDF 2.0: XMP authoritative; Info keeps only `CreationDate`/`ModDate` unless the file already has others (kept, flagged)
- PDF/A claim check: keep `pdfaid:`; validate Info/XMP equivalence after the op; `ConformanceWarning` returned by the planner before save
- Bindings in `laminapdf-wasm` / `@laminapdf/sdk`: `doc.setMetadata()`, `doc.removeMetadata()`, `doc.metadata()`

**Out of scope** (and which todo covers it)

- UI for editing metadata → [E1f](../ready/E1f-metadata-ui-capability-page.md)
- Custom/extension schema editing → [E3e](../ready/E3e-xmp-editor.md)
- Stripping all hidden metadata as sanitisation (G.08) → [E11d](../ready/E11d-sanitiser.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.01` title, author, subject, keywords; `J.02` creation/modification dates; `J.03` producer and creator; `J.10` remove selected metadata
- **Crates and packages:** `laminapdf-ops`, `laminapdf-model` (`xmp` from E1b), `laminapdf-save` (save hook), `laminapdf-wasm`, `packages/sdk`
- **`Op` variants:** `SetMetadata`, `RemoveMetadata`, and the planner-generated `UpdateSaveMetadata` (all class `Metadata`, `requires_rewrite = false`)
- **`SaveConstraint`:** `Free` → incremental. Certified files: class `Metadata` is a DocMDP violation at every level (P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → `Conflict`, gated: the UI names the certification signature and the change is saved only through the explicit invalidation override (ADR 0007 §3). Approval signatures only (no certification): `AppendOnly`, saved as an incremental update that leaves their `/ByteRange` intact; FieldMDP/`/Lock` does not apply because no field changes. `UpdateSaveMetadata` follows the same rule and is dropped rather than turning an allowed save into a `Conflict` (see In scope). `RemoveMetadata` does not make old values disappear from earlier revisions: the op result says so and offers a full rewrite (which on signed files is the override path)
- **Web vs desktop:** identical (core); UI in E1f
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `J.01` → `round-trip` (core, pdf); `J.02` → `round-trip` (core, pdf); `J.03` → `round-trip` (core, pdf); `J.10` → `round-trip` (core, pdf)

## Tasks

- [x] Define `MetadataPatch` (title, author(s), subject, keywords, creator, producer, creation/modification dates, `Option<Option<_>>` for clear) and `MetadataKey`
- [x] Implement `apply` for both ops through `CosMut`: Info dict (keep key order), XMP via E1b; create Info and/or XMP when missing
- [x] Undo records per property; coalescing key for typing into one field (`transact_coalescing`)
- [x] `SaveOptions { metadata_date, instance_id }` supplied by shells; derive `InstanceID` deterministically from the content hash when the shell passes none, so tests stay deterministic
- [x] `UpdateSaveMetadata`: generated by `SavePlanner` before classification, classified as `Metadata`, kept only when `would_violate` is empty and `constraint.kind()` is unchanged; `SaveReport::metadata_not_updated` otherwise
- [x] Conformance module: detect PDF/A (`pdfaid:part`) and PDF/UA claims; after applying, check Info/XMP equivalence and required properties; produce `ConformanceWarning` codes
- [x] Expose `planner.conformance_warnings()` to the UI (E1e/E1f)
- [x] Differential (moved from [E1b](../done/E1b-xmp-packet-model.md), which couldn't run `exiftool`): the XMP values `laminapdf_model::xmp::Packet` reads against `exiftool -j` over the corpus, in the nightly lane, beside the pypdf metadata differential; `exiftool` pinned in `doctor.rs`
- [x] Pass `xmp::Written::canonical` on: a save that rewrote the packet whole (UTF-16 or `end="r"`) says so in its report
- [x] Bindings and TS types; examples `examples/edit-metadata.rs` and `packages/sdk/examples/edit-metadata.ts`
- [x] Create the shared signed-fixture set `fixtures/signed/` (approval signature, certification P1, P2, P3, two signatures, FieldMDP lock), generated by a pyHanko script in `fixtures/scripts/` with provenance in `fixtures/README.md`; every later op's classification test reuses it
- [x] Keep `SetInfo` as is on the wire (`setInfo`, same fields and meaning); mark it deprecated in docs; route its `apply` through the new writer. Never rewrite or delete the frozen `fixtures/ops/v1/` samples: add new `setMetadata`/`removeMetadata` samples next to them. (If a later change ever removes `SetInfo`, that PR bumps `v` and adds `migrate_v1_to_v2` mapping `setInfo` → `setMetadata`, per ADR 0006 §6)

**As built.** The stamp runs only when the shell passes a date or an ID, so default saves are unchanged; the planner-side rules are in the [E1 plan §2](../../plans/editor/e1-editing-foundation.md#2-the-operations). The exiftool differential (`cargo xtask differential --live --xmp`) has run only over hand-written samples: exiftool can't run on the machine this was built on, so its first nightly run is its real test. The usage-rights case is a hand-built file (pyHanko doesn't sign UR3), and the P2 form-fill case is a planner unit test, since no form-filling op exists yet. The capability matrix's `pdf` column stays unflipped: the SDK's tests name the rows they prove (`J.01 (pdf, round-trip): …`), but `cargo xtask capabilities` refuses `[cap …]` tags until it can run TypeScript tests, which needs the WebAssembly built in the full lane's capability job.

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug. Expected for `SetMetadata`, `RemoveMetadata` and `UpdateSaveMetadata`: violation on P1, P2 and P3; none on the approval-only and FieldMDP-lock (approval) fixtures, where the save is `AppendOnly`
- [ ] Save-time metadata never escapes classification: a title edit on an unsigned file updates `MetadataDate`, `ModDate` and `InstanceID`; a form fill (stub `FormFill`-class test op) on the P2 fixture saves `AppendOnly`, the appended revision touches neither Info nor `/Metadata`, and `metadata_not_updated` is set; a save on the approval-only fixture updates them incrementally
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Info and XMP agree after every op on every metadata fixture (test compares both views)
- [ ] veraPDF (run locally as an external CLI; output summarised in the PR) still passes on edited PDF/A-1b/2b/3b fixtures after a title change; a change that would break the claim (e.g. removing the XMP packet) yields a `ConformanceWarning` before save
- [ ] A PDF 2.0 fixture gets no new deprecated Info keys
- [ ] The existing frozen `fixtures/ops/v1/` `setInfo` samples are byte-unchanged in the diff and still deserialise and apply (now also updating XMP); a v1 log mixing `setInfo` and `setMetadata` applies
- [ ] Determinism: saving the same state twice gives identical bytes on native and wasm32 (ADR 0007 §6)

## Post-merge checks

- [ ] The veraPDF check on the edited PDF/A fixtures is green in the first scheduled nightly run on `main` (the PR runs it locally; link the run in a follow-up PR or the done todo)

## Tests

- Unit and property tests in `crates/laminapdf-ops/tests/metadata.rs`
- Round-trip suite in `crates/laminapdf/tests/roundtrip_metadata.rs` with readers: lamina, pdf.js (Node harness), PDFium, qpdf `--check`; pypdf metadata differential
- veraPDF on PDF/A fixtures in the nightly lane (external tool, never linked)
- Signed fixtures (approval, certification P1/P2/P3) for gating
- Vitest in `packages/sdk` for the TS API

## Docs to update

- `@laminapdf/sdk` and `lamina` API docs pages for metadata
- `fixtures/README.md` for new PDF/A and signed fixtures
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Ambiguous mapping of multiple authors (`dc:creator` Seq vs Info `/Author` string): document the joining rule (`; `)
- Some files have conflicting Info and XMP values; which wins on read? Recommendation: XMP for PDF 2.0 and PDF/A, newer `ModDate` otherwise — decide in E1a

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
