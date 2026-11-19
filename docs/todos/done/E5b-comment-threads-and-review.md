# E5b — Comment threads, replies and review state

| Field | Value |
| --- | --- |
| ID | `E5b` |
| Status | `done` |
| Plan | [E5 plan §2.1](../../plans/editor/e5-review-stamps-interchange.md#21-ops) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4b](./E4b-annotation-model-and-overlay.md), [E5a](./E5a-review-stamps-interchange-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5b-comment-threads-and-review` |
| PR title | `feat(ops): comment threads, replies and review state` |
| Size | M (200–400): the thread model, two ops and a helper; the panel is E5g |
| PR | [#618](https://github.com/laudebugs/laminapdf/pull/618) |

## Goal

The thread model and `comment_threads`, the `AddReply` and `SetReviewState` ops, and `delete_annotations`, so the Comments panel ([E5g](../ready/E5g-comments-panel.md)) and interchange ([E5d](../done/E5d-xfdf-fdf.md)) have one place to read and write a review.

## Context

- [E5 plan §2.1 and §2.2](../../plans/editor/e5-review-stamps-interchange.md#22-reading-comments)
- [04 §2.3 Markup annotations, replies, review state](../../research/pdf/04-interactive-security.md#23-markup-annotations-replies-review-state-12562)
- ISO 32000-2 §12.5.6.2 and §12.5.6.4

## Scope

**In scope**

- `laminapdf-model`: the thread model (replies in order, the latest state per model, broken threads flagged, cycles followed once), states read as names or strings, and in the façade (feature `extract`) the quoted text from the text under a markup annotation's quads, since the model can't extract text
- `comment_threads(pages)` in the façade, WASM and SDK (`doc.annotations.threads`)
- `AddReply` and `SetReviewState`: Text annotations with `/IRT`, the parent's `/Rect`, an appearance that draws nothing and no hiding flag, checked against pdf.js, PDFium and Acrobat-made reference files requested in [issue #524](https://github.com/laudebugs/laminapdf/issues/524)
- `delete_annotations(ids)`: one transaction of E4b's `DeleteAnnot`, after dropping IDs another selected ID's cascade already removes
- `fixtures/annotations/review.pdf`

**Out of scope** (and which todo covers it)

- The panel → [E5g](../ready/E5g-comments-panel.md)
- XFDF and FDF → [E5d](../done/E5d-xfdf-fdf.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.10` replies to comments; `D.11` status and review properties; `D.12` filter and search comments; `D.15` delete annotations
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf`, `laminapdf-wasm`, `@laminapdf/sdk`
- **`Op` variants:** `AddReply`, `SetReviewState` (class `Annotation`)
- **`SaveConstraint`:** as [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.10`, `D.11` → `round-trip` (core, pdf); `D.12` → `read` (core, pdf)

## Tasks

- [x] The thread model and its reader
- [x] `comment_threads` through the bindings
- [x] The two ops and `delete_annotations`
- [x] Fixtures

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new op and function is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdicts of [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint), because a misclassified op is a security bug
- [x] Round-trip: open → op → save → reopen in Lamina PDF, pdf.js and PDFium preserves the change and everything else; `qpdf --check` is clean
- [x] Deleting a comment together with its reply deletes both in one step
- [x] Every thread of `review.pdf` reads back, the broken ones flagged and none dropped
- [x] pdf.js reports each new reply's `inReplyTo` and each state, and neither pdf.js nor PDFium draws an icon for them

## As built

- **The thread model** is `laminapdf_model::Document::comment_threads` (`comments.rs`). It puts every markup annotation of every page in exactly one `CommentThread`: the comment, then its group, replies and states as `ThreadMember`s with their pages, in document order. `Annot::state` reads `/StateModel` and `/State` as names or strings, a missing model taken from the state's name and a missing state from the model's default. `CommentThread::latest_state` goes by `/CreationDate` (else `/M`), then by position. A reply to an annotation no page lists is `BrokenThread::MissingParent`, and a reply to a listed annotation that isn't markup (a Popup) starts a thread of its own, unbroken. A ring of `/IRT`s starts at its first annotation and is `BrokenThread::Cycle`. `AnnotSubtype::is_markup` is the one list of markup subtypes.
- **The ops' payloads differ from the plan's first draft**, which the plan now records: neither carries the new annotation's ID, which `Applied::created_annotations` reports as for `AddAnnot`; `SetReviewState` takes one `state` whose model follows from it; `AddReply` has no rich text, which waits for E4e's rich-text model. A state's `/Contents` says what was set, as Acrobat writes it ("Accepted set by Ben"), for screen readers and readers without review support. Both refuse a parent that isn't markup (`invalid-op`).
- **`delete_annotations`** is `laminapdf_ops::delete_annotations` (the ops) and the façade's `Document::delete_annotations` (one transaction). An ID is left out when another selected ID's deletion takes it but not the other way round, or when an ID already kept takes it, so both members of a group or a ring give one delete.
- **Quoted text** is every glyph whose centre is inside a text markup's quad bounds, read with the extractor (feature `extract`), lines joined by a space.
- **The SDK** follows its flat API rather than the plan's `doc.annotations.threads`: `doc.commentThreads(pages)` and `doc.deleteAnnotations(ids)`, with the `CommentThread` and `CommentPart` types (`Comment` is a DOM type). Both are in the extract and text modules, beside `pageAnnotations`.
- **pdf.js and PDFium.** `tools/fixtures/pdfjs_replies.mjs` records pdf.js reading each reply's `inReplyTo` and each state, and drawing as many operators for the answered example as for the marked-up one. The `review_comments` example's render goldens are `mark_up`'s images, so `cargo xtask render-golden` checks that PDFium and hayro draw nothing for a reply or a state. The Acrobat reference files of issue #524 never arrived; the owner confirmed testing without them.
- **The capability matrix** shows D.10 and D.11 at round-trip and D.12 at read in the core column. The `pdf` column takes no evidence yet (TypeScript tags count only in the reader's end-to-end tests), so the SDK's tests (`packages/sdk/test/comments.test.ts`) aren't counted there.
- **Fixtures:** `annotations/review.pdf`; the frozen samples `ops/v1/add-reply.json` and `set-review-state.json`, which the `ops_json` fuzz target takes as seeds; the `annots` fuzz entry replies to and sets states on what it reads; the round-trip suite's mark cases add a reply and a state.
- **The example** is `review_comments` / `review-comments`, answering `mark_up`'s review, on the docs site's Answer a review recipe.

## Post-merge checks

None.

## Tests

- Unit tests over `review.pdf` and the corpus
- The round-trip suite
- `ops_json` gains the ops

## Docs to update

- API docs: comment threads and review
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- How Acrobat shows replies decides their flags; record the reference files' behaviour in the API docs

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
