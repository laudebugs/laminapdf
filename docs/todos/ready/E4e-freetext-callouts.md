# E4e — FreeText text boxes and callouts with `/DA` and rich text

| Field | Value |
| --- | --- |
| ID | `E4e` |
| Status | `ready` |
| Plan | [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [B4b](./B4b-shaping-bidi-line-breaking.md), [B4c](./B4c-standard-14-and-simple-fonts.md), [E4b](./E4b-annotation-model-and-overlay.md) |
| Blocks | [E4f](./E4f-annotation-tools-ui.md), [E8d](./E8d-field-appearances.md) |
| Dependency notes | [E4b](E4b-annotation-model-and-overlay.md), `B4` ([pdf-library §B4](../../plans/pdf-library.md#b4-fonts-and-text): fonts, shaping, subsetting) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4e-freetext-callouts` |
| PR title | `feat(build): FreeText and callout annotations with generated appearances` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Create and edit FreeText annotations (plain text box, typewriter, callout with `/CL` and `/IT /FreeTextCallout`), with required `/DA`, optional `/RC` per E4a, and appearance streams laid out with lamina-text (wrapping, alignment `/Q`, font embedding/subsetting, fallback for missing glyphs).

## Context

- [04 §2.2](../../research/pdf/04-interactive-security.md#22-the-annotation-zoo) (FreeText is the hard subtype)
- [library B4](../../plans/pdf-library.md#b4-fonts-and-text): shaping, line breaking, subsetting
- [03 §7 layout and shaping](../../research/pdf/03-text-and-fonts.md#7-text-layout--shaping--everything-pdf-does-not-do-for-you)

## Scope

**In scope**

- `DA` string parse/write
- Layout via lamina-text (bidi, line breaking) into the appearance stream; font resources embedded in `/AP` (subset) and referenced from `/DA` via AcroForm `/DR` where needed
- Callout line geometry and `/RD`
- Rich text writer for the E4a subset; reading `/RC` for display
- Editing in place in the overlay (contenteditable mapped to the annotation)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.06` callouts and text boxes; `D.16` generate appearance streams (cross-cutting)
- **Crates and packages:** `lamina-build`, `lamina-text`, `lamina-model`, `@lamina/viewer`
- **`Op` variants:** `AddAnnot` spec `FreeText`; `UpdateAnnot` text patch with coalescing
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Annotation` is allowed only under DocMDP P3 or when there is no certification signature (approval signatures alone allow annotations); otherwise a violation → warn and gate. Note the Evil Annotation Attack: annotations added after certification are shown as such in the signature panel (E10)
- **Web vs desktop:** identical; desktop may use system fonts passed as bytes (edge font discovery from E6c if present, otherwise bundled fonts)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.06` → `round-trip` (core, pdf); `D.16` → `write` (core, pdf)

## Tasks

- [ ] DA parser/writer
- [ ] Layout and appearance builder
- [ ] Callout geometry
- [ ] Rich text subset
- [ ] In-place editing

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Text boxes in Latin, Arabic and CJK render identically in pdf.js and PDFium and extract correctly
- [ ] Editing an Acrobat-made FreeText keeps its `/RC` and regenerates `/AP`

## Post-merge checks

None.

## Tests

- Goldens
- Render diffs
- Extraction tests (pdftotext, pdf.js)

## Docs to update

- API docs: FreeText
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Viewers that regenerate FreeText appearances from `/RC` (Acrobat) may show differences; document

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
