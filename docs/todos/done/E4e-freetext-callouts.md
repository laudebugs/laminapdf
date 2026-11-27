# E4e — FreeText text boxes and callouts with `/DA` and rich text

| Field | Value |
| --- | --- |
| ID | `E4e` |
| Status | `done` |
| Plan | [E4 plan §2.1](../../plans/editor/e4-markup-and-drawing-annotations.md#21-ops) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [B4b](./B4b-shaping-bidi-line-breaking.md), [B4c](./B4c-standard-14-and-simple-fonts.md), [E2p](./E2p-source-documents-and-copier.md), [E4a](./E4a-markup-and-drawing-annotations-feature-plan.md), [E4b](./E4b-annotation-model-and-overlay.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4e-freetext-callouts` |
| PR title | `feat(build): FreeText and callout annotations with generated appearances` |
| Size | L (400–800): `/DA`, the rich-text subset, layout into the appearance and callout geometry; editing in place is E4j |
| PR | [#678](https://github.com/laudebugs/laminapdf/pull/678) |

## Goal

Create and edit FreeText annotations (a text box, typewriter text, and a callout with `/CL` and `/IT /FreeTextCallout`), with the required `/DA`, `/RC` in the subset of [E4 plan §11](../../plans/editor/e4-markup-and-drawing-annotations.md#11-decisions-on-the-open-questions), and appearance streams laid out by `laminapdf-text`: wrapping, alignment (`/Q`), embedded subset fonts and a fallback for missing glyphs.

## Context

- **Design (Paper):** [C03 Free text and callouts](../../design/paper/comment-review.md#c03-free-text-and-callouts). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E4 plan §2.1, §2.2](../../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated) (the base module has no fonts) and [§11](../../plans/editor/e4-markup-and-drawing-annotations.md#11-decisions-on-the-open-questions) (the rich-text subset)
- [04 §2.2](../../research/pdf/04-interactive-security.md#22-the-annotation-zoo) (FreeText is the hard subtype)
- [library B4](../../plans/pdf-library.md#b4-fonts-and-text): shaping, line breaking, subsetting
- [03 §7 layout and shaping](../../research/pdf/03-text-and-fonts.md#7-text-layout--shaping--everything-pdf-does-not-do-for-you)

## Scope

**In scope**

- The `FreeText` spec and its patches; `FontRef` as a standard 14 name or a font's bytes by `BlobId` (E2p)
- `/DA` and `/DS` parse and write
- The rich-text subset: written for bold and italic, read wider; `rich-text-simplified` when an edit drops styles
- Layout through `laminapdf-text` (bidi, line breaking, fallback fonts) into the appearance; fonts embedded as subsets in the appearance's resources
- Callout geometry (`/CL`, the knee, the line ending) and `/RD`; text upright on rotated pages
- `appearance-needs-text` in the base WebAssembly module; the builder's fonts added to the extract module, which the viewer's worker opens documents in, measured against `packages/wasm/size-budgets.json`; if it doesn't fit, stop and ask the owner ([E4 plan §2.2](../../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated))
- `font-not-embedded` on PDF/A files for a standard 14 font

**Out of scope** (and which todo covers it)

- Editing in place, the font picker and the bold and italic controls → [E4j](../done/E4j-text-box-tools.md)
- System fonts on the desktop → [E6c](../ongoing/E6c-add-text.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.06` callouts and text boxes; `D.16` generate appearance streams (cross-cutting)
- **Crates and packages:** `laminapdf-build`, `laminapdf-text`, `laminapdf-model`, `laminapdf-ops`, `laminapdf`, `laminapdf-wasm`, `@laminapdf/sdk`
- **`Op` variants:** `AddAnnot` with `FreeText`; `UpdateAnnot` text patches, coalesced per editing session
- **`SaveConstraint`:** as [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.06` → `round-trip` (core, pdf); `D.16` → `write` (core, pdf)

## Tasks

- [x] `/DA` and `/DS` parser and writer
- [x] Rich-text reader and writer
- [x] Layout and the appearance generator
- [x] Callout geometry
- [x] Bindings and examples

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): the spec is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] `planner.would_violate(&op)` on the signed fixtures gives the verdicts of [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- [x] Round-trip: open → op → save → reopen in Lamina PDF, pdf.js and PDFium preserves the change and everything else; `qpdf --check` is clean
- [x] Text boxes in Latin, Arabic and CJK render alike in pdf.js and PDFium and extract correctly with pdftotext and pdf.js
- [x] Editing the pdf.js corpus's `annotation-freetext.pdf`, and an Acrobat-made box once one is supplied, keeps its look within the subset and regenerates `/AP`

## Post-merge checks

None.

## Tests

- Goldens in `fixtures/golden/annots/freetext.pdf` and render diffs
- Extraction tests (pdftotext, pdf.js)
- The `rich_text` fuzz target for the `/RC` reader

## Docs to update

- API docs: FreeText
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Viewers that regenerate FreeText appearances from `/RC` (Acrobat) may show differences; document them

**Outcome.** Redrawn by Lamina PDF, pdf.js's `annotation-freetext.pdf`, `rotated_freetexts.pdf` and `freetexts.pdf` keep their look for the boxes made by Acrobat, Firefox, Edge and Foxit. Two differ:

- Preview keeps a box's colour, font and size only in its own `AAPL:AKAnnotationObject` data, and writes `/DA (//Helvetica 12 Tf 0 g)`, so a redrawn Preview box is 12 pt black Helvetica.
- Okular writes `/IT /FreeTextTypeWriter` but wraps the text at the box. Lamina PDF follows Acrobat's typewriter tool, whose lines end only at line breaks, and grows the appearance to hold them; a redrawn Okular box is one line.

The extract module lays out standard 14 fonts only, with their AFM widths; a font given by its bytes, and fallback fonts, need the text module (`appearance-needs-text`), and the base module draws no text box (the owner's decision, measured against `packages/wasm/size-budgets.json`).

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
