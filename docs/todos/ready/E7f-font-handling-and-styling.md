# E7f — Font handling: re-subset, substitution, font/size/colour/style, alignment and spacing

| Field | Value |
| --- | --- |
| ID | `E7f` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6c](./E6c-add-text.md), [E7e](./E7e-text-runs-and-inline-edit.md) |
| Blocks | [E7g](./E7g-search-and-replace.md) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7f-font-handling-and-styling` |
| PR title | `feat(text): font substitution and text styling for edited runs` |
| Size | L (600–1,500) |
| PR | — |

## Goal

When the original subset lacks glyphs, add a new subset of the same font (if the full font is available) or substitute (bundled/system font, metrics-matched); change font family/size/colour/style, bold/italic/underline/super/subscript, alignment and line spacing for edited lines.

## Context

- [03 §6 Font embedding and subsetting](../../research/pdf/03-text-and-fonts.md#6-font-embedding--subsetting-from-code)
- [03 §7 Layout](../../research/pdf/03-text-and-fonts.md#7-text-layout--shaping--everything-pdf-does-not-do-for-you)

## Scope

**In scope**

- Font resolution chain
- Style ops: `SetTextStyle { runs, style }`
- Line-level layout for alignment/spacing
- Synthetic bold/italic only as fallback

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.02` edit existing text; `A.04` change font family/size/colour/style; `A.05` bold, italic, underline, super/subscript; `A.06` alignment and line spacing
- **Crates and packages:** `lamina-text`, `lamina-content`
- **`Op` variants:** `SetTextStyle` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` violates every DocMDP level and approval signatures' integrity → blocked unless the user confirms invalidating the named signatures
- **Web vs desktop:** desktop substitution may use system fonts; web bundled/user fonts
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.02` → `round-trip` (core, pdf, web, desktop); `A.04` → `round-trip` (core, pdf, web, desktop); `A.05` → `round-trip` (core, pdf, web, desktop); `A.06` → `write` (core, pdf, web, desktop)

## Tasks

- [ ] Resolution chain
- [ ] Style op
- [ ] Layout
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Editing text that needs new glyphs succeeds on fixtures with subset fonts and the result extracts correctly

## Tests

- Edit corpus
- Extraction
- Render diffs

## Docs to update

- User help
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Metric mismatch after substitution overflows lines; warn when the edited line grows beyond its original width

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
