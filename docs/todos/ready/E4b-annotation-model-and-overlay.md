# E4b — Annotation model, the annotation ops' core and the appearance trait

| Field | Value |
| --- | --- |
| ID | `E4b` |
| Status | `ready` |
| Plan | [E4 plan §2](../../plans/editor/e4-markup-and-drawing-annotations.md#2-the-operations) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E1d](../done/E1d-edit-session-undo-ui.md), [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4b-annotation-model-and-overlay` |
| PR title | `feat(model): typed annotations and the annotation ops' core` |
| Size | L (400–800): the views and the read API, three ops without specs, the trait, bindings and a fuzz target; the specs and the viewer are other todos |
| PR | — |

## Goal

Typed annotation views in `laminapdf-model`, `page_annotations` in the façade and the SDK, and the core every annotation op of the round shares: `AddAnnot` (its common part), `UpdateAnnot` and `DeleteAnnot`, with the `Appearances` trait that lets the ops call the builder's generator ([E4 plan §2.1–§2.3](../../plans/editor/e4-markup-and-drawing-annotations.md#21-ops)).

## Context

- [E4 plan §2](../../plans/editor/e4-markup-and-drawing-annotations.md#2-the-operations), [§3](../../plans/editor/e4-markup-and-drawing-annotations.md#3-conformance-the-edits-must-keep) and [§6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- [04 §2.1 The common dictionary](../../research/pdf/04-interactive-security.md#21-the-common-dictionary) and [§2.3 Markup annotations](../../research/pdf/04-interactive-security.md#23-markup-annotations-replies-review-state-12562)
- [ADR 0005](../../decisions/0005-document-model.md) §5: `/NM` is a hint, never the ID
- [E3c](E3c-links.md): the link ops, which share this core

## Scope

**In scope**

- `laminapdf-model`: `Annotation` (subtype enum with unknown passthrough), `MarkupAnnotation` (`/T`, `/Contents`, `/CreationDate`, `/Subj`, `/IRT`, `/RT`, `/CA`, `/RC`), `Popup` pairing, geometry readers that accept quads in either order, the `/NM` index with `annot-name-duplicate`
- The façade's `page_annotations(index)` and the SDK's `doc.annotations.list(page)`: the record of [E4 plan §2.3](../../plans/editor/e4-markup-and-drawing-annotations.md#23-reading-annotations), in page space
- `laminapdf-ops`: `AddAnnot`'s common part (`AnnotCommon`, `/P`, `/F` with Print, `/NM`, `/M`, the structure element in tagged files), `UpdateAnnot` (common properties and `translate` for any subtype but Widget and Link), `DeleteAnnot` (with its Popup, replies and group, `also_deleted`), `annot-locked`
- The `Appearances` trait in `laminapdf-ops` and its implementation skeleton in `laminapdf-build`, wired through the façade ([E4 plan §2.2](../../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated)); `appearance-needs-text`
- If [E3c](E3c-links.md) merged first, `AddLink`, `UpdateLink` and `DeleteLink` rebuilt over this core, with a test that both paths write the same bytes; otherwise E3c does it
- `fixtures/annotations/` and `tools/fixtures/annotations.py` ([E4 plan §9](../../plans/editor/e4-markup-and-drawing-annotations.md#9-fixtures))
- The `annots` fuzz target, its regressions folder in the model's guard list

**Out of scope** (and which todo covers it)

- Each subtype's spec and appearance → [E4c](E4c-appearances-markup-notes-caret.md), [E4d](E4d-appearances-ink-and-shapes.md), [E4e](E4e-freetext-callouts.md)
- The viewer's annotation layer → [E4i](E4i-viewer-annotation-layer.md)

**Editor facts**

- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf-build` (the trait's implementation), `laminapdf`, `laminapdf-wasm`, `@laminapdf/sdk`
- **`Op` variants:** `AddAnnot` (core), `UpdateAnnot`, `DeleteAnnot` (class `Annotation`)
- **`SaveConstraint`:** as [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint); in tagged certified-P3 files, E3c's pyHanko answer decides whether annotations are tagged
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** none (the subtypes flip in E4c–E4e)

## Tasks

- [ ] Views, unknown-key passthrough and the `/NM` index
- [ ] `page_annotations` in the façade, WASM and SDK
- [ ] The three ops with undo and coalescing keys
- [ ] The `Appearances` trait and its wiring
- [ ] Fixtures and the fuzz target

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Unknown annotation keys survive an update and a translate (round-trip test)
- [ ] `page_annotations` agrees with pdf.js's `getAnnotations` on subtype and count over the pdf.js corpus's `annotation-*.pdf` files
- [ ] `DeleteAnnot` on `threads.pdf` removes the reply chain through its cycle and reports it

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

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
