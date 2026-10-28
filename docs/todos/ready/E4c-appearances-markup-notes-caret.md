# E4c — Appearances and creation: text markup, notes and caret

| Field | Value |
| --- | --- |
| ID | `E4c` |
| Status | `ready` |
| Plan | [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [C5b](./C5b-find-in-document.md), [E4b](./E4b-annotation-model-and-overlay.md) |
| Blocks | [E4f](./E4f-annotation-tools-ui.md) |
| Dependency notes | [E4b](E4b-annotation-model-and-overlay.md), `C5` ([web-app §C5](../../plans/web-app.md#c5-text-selection-and-search): text layer and search) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4c-appearances-markup-notes-caret` |
| PR title | `feat(build): appearance streams for text markup, notes and caret annotations` |
| Size | M (200–600) |
| PR | — |

## Goal

Create highlight/underline/strike-through/squiggly annotations from glyph quads, sticky notes (Text + Popup with standard icons), and text-correction caret annotations grouped with StrikeOut (`/IRT` + `/RT /Group`), each with a generated appearance stream.

## Context

- [04 §2.2 The annotation zoo](../../research/pdf/04-interactive-security.md#22-the-annotation-zoo)
- [04 §2.4 Why appearance streams matter](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter): lamina always writes `/AP` for what it creates ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 3)
- [web C5](../../plans/web-app.md#c5-text-selection-and-search): quads from selection
- ISO 32000-2 §12.5.6.10 (text markup), §12.5.6.4 (text), §12.5.6.11 (caret)

## Scope

**In scope**

- Appearance builders: highlight (Multiply blend via ExtGState `/BM`), underline, strike-out, squiggly; Text icons (Note, Comment, Key, Help, Paragraph, NewParagraph, Insert); Caret
- QuadPoints ordering per spec and as viewers expect (pdf.js/PDFium order)
- Replacement-text group (Caret + StrikeOut)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.01` highlight / underline / strike through; `D.02` sticky notes and text comments; `D.03` text-correction annotations
- **Crates and packages:** `lamina-build` (appearances), `lamina-model`
- **`Op` variants:** `AddAnnot` with specs `TextMarkup`, `Note`, `Caret`, `ReplaceText` group
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Annotation` is allowed only under DocMDP P3 or when there is no certification signature (approval signatures alone allow annotations); otherwise a violation → warn and gate. Note the Evil Annotation Attack: annotations added after certification are shown as such in the signature panel (E10)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.01` → `round-trip` (core, pdf); `D.02` → `round-trip` (core, pdf); `D.03` → `round-trip` (core, pdf)

## Tasks

- [ ] Builders and golden appearance streams
- [ ] Specs and validation
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Appearances render the same in pdf.js and PDFium (diff below threshold) and in Acrobat-generated reference renders
- [ ] Highlight keeps text legible (Multiply) in both renderers

## Tests

- Golden appearance streams in `fixtures/golden/annots/`
- Render diffs pdf.js vs PDFium
- Round-trip

## Docs to update

- API docs: markup annotations
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
