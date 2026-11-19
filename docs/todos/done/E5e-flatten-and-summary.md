# E5e — Flatten annotations into page content

| Field | Value |
| --- | --- |
| ID | `E5e` |
| Status | `done` |
| Plan | [E5 plan §2.1](../../plans/editor/e5-review-stamps-interchange.md#21-ops) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E2e](./E2e-content-append-api.md), [E4c](./E4c-appearances-markup-notes-caret.md), [E4d](./E4d-appearances-ink-and-shapes.md), [E5a](./E5a-review-stamps-interchange-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5e-flatten` |
| PR title | `feat(ops): flatten annotations into page content` |
| Size | M (200–400): one op over the content append API, and the appearance functions moved to the model |
| PR | — |

## Goal

`FlattenAnnots`: each chosen annotation's normal appearance drawn into its page through the content append API, then the annotation removed, as [E5 plan §2.1](../../plans/editor/e5-review-stamps-interchange.md#21-ops) sets out.

## Context

- [E5 plan §2.1, §3 and §5](../../plans/editor/e5-review-stamps-interchange.md#21-ops)
- [04 §2.5 Flattening](../../research/pdf/04-interactive-security.md#25-flattening); ISO 32000-2 §12.5.5
- [E2e](../done/E2e-content-append-api.md): the content append API

## Scope

**In scope**

- `FlattenAnnots { annots, intent }` (class `Content`): `/AS` resolved, `wrap_existing`, `add_xobject`, `q <A> cm /FmN Do Q`, the annotation, its Popup and its replies removed
- Skipped: hidden for the intent, optional content that is off; `flatten-no-appearance` when no appearance can be made; `flatten-widget` for widgets
- Tagged files: the appearance drawn as an artifact and its `Annot` element removed; `flatten-untagged` on PDF/UA-1 files
- `still_in_earlier_revisions` set
- The appearance and matrix functions moved from `laminapdf-extract` to `laminapdf-model`, both crates using one copy
- `fixtures/annotations/flatten.pdf`

**Out of scope** (and which todo covers it)

- The comment summary → [E5f](../done/E5f-comment-summary.md)
- The dialog → [E5i](../ready/E5i-interchange-flatten-summary-ui.md)
- Form fields → [E8g](../ongoing/E8g-form-data-and-flatten.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.14` flatten annotations into content
- **Crates and packages:** `laminapdf-ops`, `laminapdf-model`, `laminapdf-content`, `laminapdf-extract`, `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** `FlattenAnnots` (class `Content`)
- **`SaveConstraint`:** as [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint): a violation under any certification
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.14` → `round-trip` (core, pdf)

## Tasks

- [x] The op
- [x] The move of the appearance functions
- [x] Fixtures and the corpus test
- [x] Bindings and examples

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new op and function is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdicts of [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint), because a misclassified op is a security bug
- [x] Round-trip: open → op → save → reopen in Lamina PDF, pdf.js and PDFium preserves the change and everything else; `qpdf --check` is clean
- [x] PDFium's render of each flattened page of `flatten.pdf` matches its render before flattening within E4g's threshold
- [x] The pdf.js corpus's annotated files flatten and render the same, or are listed with their reason

## As built

- **The op** is `FlattenAnnots { annots, intent }` in `laminapdf-ops`' `annot/flatten.rs`, class `Content`, no full rewrite, no usage right of its own (a UR3 file refuses class `Content` already). It draws each annotation, and each member of its Popup, reply and group cascade that has an appearance, in `/Annots` order, then removes the chosen ones through `DeleteAnnot`, so the structure tree loses their `Annot` elements and the parent tree their keys. An ID an earlier one's removal took is skipped. The drawing is `[/Artifact BMC] [/OC /OcN BDC] q [/GsN gs] <A> cm /FmN Do Q [EMC] [EMC]`: the artifact when the file is tagged (`/MarkInfo /Marked` or a structure tree); the annotation's `/OC`, registered under the page's `/Properties` (`PageContentWriter::add_properties`), so the drawing keeps following its layer; `/CA` (stroking), `/ca` (non-stroking, PDF 2.0, else `/CA`) and `/BM` in an ExtGState, as §12.5.2 applies them to the appearance. A `NoRotate` annotation on a turned page is drawn turned back about its `/Rect`'s upper-left corner (§12.5.3); `NoZoom` has no meaning once the drawing is page content. A stream written in place, or one without `/Subtype /Form`, is drawn from a Form XObject copy.
- **Left in place, not removed:** an annotation hidden for the intent (`Hidden`; `NoView` on screen; no `Print` flag printed), and one whose `/OC` (or whose appearance's `/OC`) is off in the default configuration (`Document::optional_content_off` in `laminapdf-model`: `/BaseState`, `/ON`, `/OFF`, a membership's `/OCGs` and `/P`; `/VE` and usage states aren't evaluated, and what can't be read counts as on; `disclosure/forms.rs` keeps its own reading, since it lists the names of layers off rather than judging content). These are not reported. One without an appearance gets the generator's first when its kind is one Lamina PDF draws; when its kind isn't, or the generator can't draw it (none, or a text box it can't lay out), it stays and is listed in `Applied::not_flattened`, which the façade reports as the `flatten-no-appearance` edit warning (`facadeWarnings`, kind `unsupported`). Any other error drawing it (bad geometry, an unknown blob) fails the op. A cascade member without an appearance (a reply) is removed without being drawn.
- **Refusals:** a widget is `flatten-widget` (new `opErrors` code). An annotation whose cascade holds a `Locked` annotation (itself, its Popup, a reply, a group member) is refused with `annotation-locked`, naming the locked one, before anything is drawn. `flatten_candidates` (which the façade's `All` and `Pages` scopes use) leaves such annotations out, with widgets, links, Popups and replies (anything with `/IRT`).
- **Conformance on save:** `flatten-untagged` is a `ConformanceWarning` (`opWarnings`) that `pending_conformance_warnings` raises on a file claiming PDF/UA-1 for the annotations in `PendingChanges::flattened` that no page lists any more, so undoing a flattening takes the warning back. On a file claiming PDF/A, a flattened appearance whose own `/Resources` name a font without a font program (Type 3 aside) raises `pdfa-font-not-embedded` the same way (plan §3); fonts of forms nested inside the appearance aren't walked.
- **The move:** `laminapdf-model`'s `appearance.rs` has `Document::normal_appearance` (`/AP /N`, the entry `/AS` names for a state dictionary), `Document::annotation_rect`, `Document::appearance_placement` and `AppearancePlacement { matrix, fit }`, plus `UserSpaceRect::transformed`. `laminapdf-content`'s `transformed_box` is deprecated in its favour (`docs/renames.json`) and no longer used inside the workspace. The placement keeps the form's `/Matrix` and the fit `A` apart: `Do` applies the form's matrix itself, so the flattener sets only `A`, while the extractor, which interprets the stream directly, uses `to_user_space()`. The extractor keeps its own visibility rules for covers.
- **Façade and bindings:** `Document::flatten_annotations(&FlattenScope, FlattenIntent)` and `flatten_op`; `FlattenScope::{All, Pages, Annotations}`; WASM `flattenAnnotations(scope, intent)` in the extract and text modules (`FlattenScopeInit`); the SDK's `doc.flattenAnnotations(scope, intent)` with `FlattenScope` and `FlattenIntent`, following the flat API of `deleteAnnotations` rather than the plan's `doc.annotations.flatten`; the plan's §4 now says so for the round's other functions. The op itself crosses through `apply` in every module. New public items say `Added in 0.6.0.` / `@since 0.6.0`, the next minor release after 0.5.0.
- **Renders, and how the PDFium criterion was narrowed:** E4g's threshold doesn't exist yet, so the comparisons use the render goldens' SSIM ≥ 0.99 (ADR 0004), and E4g's todo now asks to recheck them. The criterion asks for each flattened page of `flatten.pdf` to match in PDFium; `laminapdf-render-pdfium`'s `tests/flatten.rs` (ignored, needs PDFium) compares 9 of its 11 flattenable annotations, on screen and printed. It leaves out two: the translucent square, since PDFium draws an annotation's appearance without its `/CA`, which flattening applies as ISO 32000-2 says; and the square without an appearance, which PDFium and Lamina PDF each draw their own way. Its corpus test flattens the 29 `annotation-*.pdf` files of the pdf.js corpus (`cargo xtask corpus fetch --source pdfjs`); 26 match, and the three whose shapes have no appearance are listed with that reason. CI's render-golden run fetches no corpus, so without it the test prints why and passes. `laminapdf-render`'s `tests/flatten.rs` checks the annotations hayro draws as the standard says (it ignores `/CA`, `NoView` and `/OC` and draws nothing without an appearance).
- **Capability D.14** is round-trip in the core column (the round-trip suite's flatten cases, which `cargo xtask roundtrip` checked clean in qpdf, pdf.js 6.4.299 and PDFium on 2026-11-16) and the pdf column (`packages/sdk/test/comments.test.ts`).
- **Fixtures and tests:** `fixtures/annotations/flatten.pdf` and `flatten-cases.pdf` (membership dictionaries, a blend mode, a form without `/Subtype`, locks, both opacities, a reply without an appearance, a `NoRotate` square on a turned page; `tools/fixtures/annotations.py`); `ops/v1/flatten-annots.json`; `laminapdf-ops`' `tests/flatten.rs`; `laminapdf-save`'s `tests/flatten.rs` (planner verdicts, both save modes, `qpdf --check`, undo and redo bytes) and the round-trip suite's flatten cases (`tests/common/roundtrip_flatten.rs`); `laminapdf`'s `tests/flatten.rs`; the SDK's `comments.test.ts`. The example is `flatten_comments` / `flatten-comments`, flattening `mark_up`'s review for printing, on the docs site's Flatten comments recipe.

## Post-merge checks

None.

## Tests

- Unit tests per flag, intent and subtype
- `save_roundtrip` gains the op
- The flatten corpus test

## Docs to update

- API docs: flattening
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
