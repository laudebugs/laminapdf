# E4b — Annotation model, the annotation ops' core and the appearance trait

| Field | Value |
| --- | --- |
| ID | `E4b` |
| Status | `done` |
| Plan | [E4 plan §2](../../plans/editor/e4-markup-and-drawing-annotations.md#2-the-operations) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E1d](./E1d-edit-session-undo-ui.md), [E2j](./E2j-page-tree-and-tree-writers.md), [E4a](./E4a-markup-and-drawing-annotations-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4b-annotation-model-and-overlay` |
| PR title | `feat(model): typed annotations and the annotation ops' core` |
| Size | L (400–800): the views and the read API, three ops without specs, the trait, bindings and a fuzz target; the specs and the viewer are other todos |
| PR | [#526](https://github.com/laudebugs/laminapdf/pull/526) |

## Goal

Typed annotation views in `laminapdf-model`, `page_annotations` in the façade and the SDK, and the core every annotation op of the round shares: `AddAnnot` (its common part), `UpdateAnnot` and `DeleteAnnot`, with the `Appearances` trait that lets the ops call the builder's generator ([E4 plan §2.1–§2.3](../../plans/editor/e4-markup-and-drawing-annotations.md#21-ops)).

## Context

- [E4 plan §2](../../plans/editor/e4-markup-and-drawing-annotations.md#2-the-operations), [§3](../../plans/editor/e4-markup-and-drawing-annotations.md#3-conformance-the-edits-must-keep) and [§6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- [04 §2.1 The common dictionary](../../research/pdf/04-interactive-security.md#21-the-common-dictionary) and [§2.3 Markup annotations](../../research/pdf/04-interactive-security.md#23-markup-annotations-replies-review-state-12562)
- [ADR 0005](../../decisions/0005-document-model.md) §5: `/NM` is a hint, never the ID
- [E3c](../done/E3c-links.md): the link ops, which share this core

## Scope

**In scope**

- `laminapdf-model`: `Annot` (an `AnnotSubtype` enum that keeps unknown subtypes), the markup entries (`/T`, `/Contents`, `/CreationDate`, `/Subj`, `/IRT`, `/RT`, `/CA`, `/RC`), `Popup` pairing both ways, each subtype's geometry with quads kept in the order stored, `AnnotFlags`, and `annot-name-duplicate`
- The façade's `page_annotations(index)` (`PageAnnotation`, in page space), WASM's `pageAnnotations` (extract and text modules) and the SDK's `doc.pageAnnotations(index)`
- `laminapdf-ops`: `UpdateAnnot` (text entries, `/M`, flags, and `translate` for any subtype but Widget, with its Popup) and `DeleteAnnot` (with its Popup, its replies and its whole group, and its structure-tree reference), `Applied::also_deleted` (and `Edited::also_deleted`, `alsoDeleted` across the boundary), `unknown-annotation`, `annotation-locked`, `annotation-not-editable`
- `fixtures/annotations/` and `tools/fixtures/annotations.py` ([E4 plan §9](../../plans/editor/e4-markup-and-drawing-annotations.md#9-fixtures)); `fixtures/ops/v1/update-annot.json` and `delete-annot.json`
- The `annots` fuzz target and its seeds (not yet in `cargo xtask fuzz` or the nightly lane, which only the owner can change: [issue #491](https://github.com/laudebugs/laminapdf/issues/491))

**Moved to [E4c](../ready/E4c-appearances-markup-notes-caret.md)**, which adds the first ops that make annotations, so each of these has something to act on:

- `AddAnnot`'s common part and the `Appearances` trait (no spec exists until E4c, and the trait has no implementation before the first generator)
- Tagged files' new `Annot` elements, `/Tabs /S`, `annot-without-contents` and `annot-not-tagged`, with E3c's pyHanko check
- `pdfa-annot-flags`, and usage rights checked per op (with E2i's usage-rights file)
- The runnable Rust and TypeScript examples: a recipe that highlights text and leaves a note; this PR has a doc example on `page_annotations`

**Out of scope** (and which todo covers it)

- Each subtype's spec and appearance → [E4c](../ready/E4c-appearances-markup-notes-caret.md), [E4d](../ready/E4d-appearances-ink-and-shapes.md), [E4e](../ready/E4e-freetext-callouts.md)
- The viewer's annotation layer → [E4i](../ready/E4i-viewer-annotation-layer.md)

**Editor facts**

- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf`, `laminapdf-wasm`, `@laminapdf/sdk`
- **`Op` variants:** `UpdateAnnot`, `DeleteAnnot` (class `Annotation`)
- **`SaveConstraint`:** as [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint); in tagged certified-P3 files, the pyHanko check decides whether annotations are tagged
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** none (the subtypes flip in E4c–E4e)

## Tasks

- [x] Views, unknown-key passthrough and the `/NM` index
- [x] `page_annotations` in the façade, WASM and SDK
- [x] `UpdateAnnot` and `DeleteAnnot` with undo
- The `Appearances` trait and `AddAnnot`: moved to [E4c](../ready/E4c-appearances-markup-notes-caret.md)
- [x] Fixtures and the fuzz target

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable doc example (the recipe pair moved to E4c, with the first ops that make annotations), tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [x] Unknown annotation keys survive an update and a translate (round-trip test)
- [x] `page_annotations` agrees with pdf.js's `getAnnotations` on every page of `fixtures/annotations/` (`pdfjs.json`); the pinned corpus's `annotation-*.pdf` files join in [E4g](../ready/E4g-appearance-parity-suite.md)'s corpus test
- [x] `DeleteAnnot` on `threads.pdf` removes the reply chain through its cycle, and deleting either member of a group removes both
- Usage rights per op: moved to [E4c](../ready/E4c-appearances-markup-notes-caret.md), with `AddAnnot` and E2i's usage-rights file
- veraPDF on `golden/examples/pdf_ua_report.pdf` after an annotation is added: moved to E4c, which adds annotations

## Post-merge checks

None.

## Tests

- Unit tests for the views over `fixtures/annotations/` and the corpus
- `fixtures/ops/v1/` samples and the round-trip suite's annotation cases
- The `annots` fuzz target

## Docs to update

- API docs: annotations (reading, moving, deleting)
- `fixtures/README.md`: `fixtures/annotations/`
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
