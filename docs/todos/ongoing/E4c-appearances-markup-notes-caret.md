# E4c — Appearances and creation: text markup, notes and caret

| Field | Value |
| --- | --- |
| ID | `E4c` |
| Status | `ongoing` |
| Plan | [E4 plan §2.1](../../plans/editor/e4-markup-and-drawing-annotations.md#21-ops) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [C5b](../done/C5b-find-in-document.md), [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md), [E4b](../done/E4b-annotation-model-and-overlay.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4c-appearances-markup-notes-caret` |
| PR title | `feat(build): text markup, notes and caret annotations with their appearances` |
| Size | M (200–400): three specs and their generators, `replace_text`, bindings |
| PR | [#539](https://github.com/laudebugs/laminapdf/pull/539) |

## Goal

Highlight, underline, strike-through and squiggly annotations from glyph quads, sticky notes (Text with a Popup and the standard icons), and text corrections (a Caret grouped with a StrikeOut), each with a generated appearance stream.

## Context

- [E4 plan §2.1](../../plans/editor/e4-markup-and-drawing-annotations.md#21-ops) (the specs and the rules every op keeps) and [§11](../../plans/editor/e4-markup-and-drawing-annotations.md#11-decisions-on-the-open-questions) (Popups only for notes)
- [04 §2.2 The annotation zoo](../../research/pdf/04-interactive-security.md#22-the-annotation-zoo) and [§2.4 Why appearance streams matter](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter)
- [web C5](../../plans/web-app.md#c5-text-selection-and-search): quads from the selection
- ISO 32000-2 §12.5.6.10 (text markup), §12.5.6.4 (text), §12.5.6.11 (caret)

## Scope

**In scope**

- From [E4b](../done/E4b-annotation-model-and-overlay.md): `AddAnnot`'s common part (`AnnotCommon`, `/P`, `/F` with Print, `/NM`, `/M`) and the `Appearances` trait (`Document::with_appearances`; `appearance-unavailable` without one) with the builder's implementation ([E4 plan §2.2](../../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated)); in tagged files the new `Annot` element through E2j's number-tree writer, `/Tabs /S`, `annot-without-contents` and `annot-not-tagged`, with E3c's pyHanko check on the certified-P3 fixture if E3c hasn't run it; `pdfa-annot-flags`; usage rights checked per op (`/Create`, `/Modify`, `/Delete`) once E2i adds the usage-rights file; and the recipe that highlights text and leaves a note, in Rust and TypeScript
- Specs `TextMarkup`, `Note` and `Caret`; the generators: highlight (Multiply through an ExtGState in the appearance), underline, strike-out, squiggly, the seven note icons (Note, Comment, Key, Help, Paragraph, NewParagraph, Insert), the caret
- Quads written in the order pdf.js, PDFium and Acrobat expect, read in either order
- The façade's `replace_text` (a StrikeOut and a Caret grouped in one transaction); the group's primary member and where the replacement text lives checked against an Acrobat-made reference file, or the pdf.js corpus's `annotation-caret-ink.pdf` until one arrives
- Opacity in one place: before freezing the goldens, measure whether pdf.js and PDFium apply `/CA` on top of an appearance, and write it as [E4 plan §2.1](../../plans/editor/e4-markup-and-drawing-annotations.md#21-ops) says
- On PDF/A files, appearances in the output intent's device space; `pdfa-transparency` on PDF/A-1
- On tagged files, `/Contents` defaults to the quoted text ([E4 plan §3](../../plans/editor/e4-markup-and-drawing-annotations.md#3-conformance-the-edits-must-keep))

**Out of scope** (and which todo covers it)

- The tools → [E4f](../ready/E4f-annotation-tools-ui.md)
- Replies and review state → [E5b](../ready/E5b-comment-threads-and-review.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.01` highlight / underline / strike through; `D.02` sticky notes and text comments; `D.03` text-correction annotations
- **Crates and packages:** `laminapdf-build` (generators), `laminapdf-ops` (specs), `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** `AddAnnot` with `TextMarkup`, `Note`, `Caret`; `UpdateAnnot` patches for each
- **`SaveConstraint`:** as [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.01`, `D.02`, `D.03` → `round-trip` (core, pdf)

## Tasks

- [x] Generators and golden appearance streams
- [x] Specs, patches and validation
- [x] `replace_text`
- [x] Bindings and examples

**As built.**

- **IDs.** `AddAnnot` makes its own ID, reported in `Applied::created_annotations`, as E3c's `AddLink` does, in place of the plan's `id` in the payload. It takes `marked`, the MCIDs under the mark, which the façade's `mark_text` reads from the glyphs. Geometry uses the model's `UserSpacePoint` and `UserSpaceRect`, a quad four points, with E3c's serde adapters.
- **Colour.** The spec's colour is `AnnotColor`, since `laminapdf-build` already exports a `Color` and a public name may live in one crate only.
- **The hook.** `Op::apply_to` takes the document's `Appearances`, which `Tx` borrows from `Document::with_appearances`; the façade sets `laminapdf-build`'s `AnnotationAppearances` on every document when its `build` feature is on, so all three WebAssembly modules draw appearances. Each appearance is a Form XObject in user space whose `/BBox` is the `/Rect`, with no `/Matrix`.
- **Opacity.** Measured on 2026-11-13 with pdf.js 6.4.299 and PDFium (pypdfium2 5.14.0): neither applies an annotation's `/CA` on top of its appearance. So opacity lives in the appearance's ExtGState, and `/CA` is also written before PDF 2.0, for readers that draw from the keys.
- **Text corrections.** The pdf.js corpus's `annotation-caret-ink.pdf` has a lone caret ("Insert Text") holding its text in `/Contents`, not a replacement group, so the group's order follows the standard and Acrobat's described behaviour: the caret is the primary and holds the replacement, and the strike-out names it with `/IRT` and `/RT /Group`. pdf.js shows a group member's `/Contents` from its primary (ISO 32000-2 §12.5.6.2), so the strike-out's own description reads as the replacement there; the round-trip suite compares contents by that rule. The Acrobat reference file of issue #521 can still confirm the order.
- **Tagging.** In a tagged file the `Annot` element goes right after the element that owns the first marked MCID, else after the element that owns the page's last marked content, else last in the document element. On a file that claims PDF/UA-1 without a structure tree, the mark is made and `annot-not-tagged` warns (for new annotations only), where `AddLink` refuses.
- **Usage rights per op** wait for E2i's usage-rights fixture; E2i's todo now carries the check.
- **Capabilities.** `D.01`, `D.02` and `D.03` are `round-trip` in the core column. The `pdf` column has no evidence mechanism yet (only the reader's end-to-end tests claim TypeScript evidence), so it stays empty, as for every other row.
- **Rotated pages** are covered by the façade's tests at `/Rotate` 0, 90, 180 and 270 over generated pages, in place of a `fixtures/annotations/rotated.pdf`: text marks read back on their page-space boxes, and a note's Popup opens right of its icon as the page is shown (left of it when the page has no room), on the page. The icon keeps its upper-left corner at the point through NoRotate.
- **Render parity.** `annots/markup.pdf` and the example were rendered in pdf.js 6.4.299, PDFium and poppler and compared by eye (alike), and hayro and PDFium render them in the render goldens; E4g's suite adds the measured threshold. pdf.js and PDFium read every mark in the round trip.

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new spec is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every spec's op reports `Annotation` and `requires_rewrite` false; `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) gives the verdicts of [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- [ ] Round-trip: open → op → save → reopen in Lamina PDF, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, structure tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Each appearance renders in pdf.js and PDFium within E4g's threshold, and a highlight keeps the text under it legible in both
- [ ] veraPDF reports nothing new on `golden/examples/pdf_ua_report.pdf` after a highlight with its default `/Contents`

## Post-merge checks

None.

## Tests

- Golden appearance streams in `fixtures/golden/annots/markup.pdf`
- Render diffs pdf.js vs PDFium
- The round-trip suite

## Docs to update

- API docs: markup annotations and notes
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Acrobat's text-correction group may differ from the standard's description; the reference file decides, and the difference is recorded in the API docs

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
