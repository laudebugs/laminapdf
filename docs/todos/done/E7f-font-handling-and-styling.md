# E7f — Font handling: re-subset, substitution, font/size/colour/style, alignment and spacing

| Field | Value |
| --- | --- |
| ID | `E7f` |
| Status | `done` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6c](./E6c-add-text.md), [E7e](./E7e-text-runs-and-inline-edit.md) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7f-font-handling-and-styling` |
| PR title | `feat(text): font substitution and text styling for edited runs` |
| Size | L (600–1,500) |
| PR | [#844](https://github.com/laudebugs/laminapdf/pull/844) |

## Goal

When the original subset lacks glyphs, add a new subset of the same font (if the full font is available) or substitute (bundled/system font, metrics-matched); change font family/size/colour/style, bold/italic/underline/super/subscript, alignment and line spacing for edited lines.

## Context

- **Design (Paper):** [A04 Edit existing text inline](../../design/paper/add-edit-content.md#a04-edit-existing-text-inline). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [03 §6 Font embedding and subsetting](../../research/pdf/03-text-and-fonts.md#6-font-embedding--subsetting-from-code)
- [03 §7 Layout](../../research/pdf/03-text-and-fonts.md#7-text-layout--shaping--everything-pdf-does-not-do-for-you)

## Scope

**In scope**

- Font resolution chain
- Style op: `SetTextStyle { runs, font, size, colour, bold, italic, underline, baseline, align, line_spacing }`, each field optional ([E7 plan](../../plans/editor/e7-editing-existing-content.md) §2.1)
- Line-level layout for alignment/spacing
- Synthetic bold/italic only as fallback
- The same chain for a page drawn in a comment summary: E5f lists a page whose fonts the file doesn't embed without its drawing (`summary-page-not-drawn`), since PDF/UA-1 needs embedded fonts; with an embedded, metrics-matched substitute the summary can draw it, and its warning names the substitutes
- Never substitute silently: when the original subset lacks a glyph the edit needs and no full copy of the font is available, the op's report names the missing characters and the substitute font, and the editor shows that before the edit is committed ([Hacker News research](../../research/hacker-news/pdf-users-on-hacker-news.md#four-things-the-evidence-argues-against): users mock editors that promise Word-style editing and quietly change fonts)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.02` edit existing text; `A.04` change font family/size/colour/style; `A.05` bold, italic, underline, super/subscript; `A.06` alignment and line spacing
- **Crates and packages:** `laminapdf-text`, `laminapdf-content`
- **`Op` variants:** `SetTextStyle` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked unless the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock` (the signature panel, E10h, reports the later content change)
- **Web vs desktop:** desktop substitution may use system fonts; web bundled/user fonts
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.02`, `A.04`, `A.05` → `round-trip` (core, pdf); `A.06` → `write` (core, pdf); web and desktop in [E7l](../ready/E7l-inline-text-editor.md)

## Tasks

- [x] Resolution chain
- [x] Style op
- [x] Layout
- [x] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [x] Editing text that needs new glyphs succeeds on fixtures with subset fonts and the result extracts correctly
- [x] An edit that needs a substitute font reports the missing characters and the substitute's name, in the op report, before it is committed; undo restores the original run. The editor's notice is [E7l](../ready/E7l-inline-text-editor.md)'s: it reads `planEditText`.
- [x] On a PDF/A fixture, an edit that adds a font raises no `pdfa-font-not-embedded` when the font is embedded, and raises it with the page when a test makes the added font unembedded (E7i's added-font check, end to end)

## Notes from the work

- **The chain is opt-in.** `EditText` takes `fonts`, the fonts a character the run's font lacks may come from, in order: a full copy of the run's own font (its bytes, from `addBlob`) adds a new subset, any other font substitutes. An op that names none fails with `missing-glyph` as before, so a font never changes silently. `plan_edit_text` (façade, WASM, SDK `content.planEditText`, viewer `planEditText`) says which characters and which font before the edit is committed; `Applied::text_edits` and `Edited.textEdits` report the same afterwards, with the fidelity (ADR 0022 §7) and `grew_by`, how many points wider the line became (the "line grows beyond its original width" warning).
- **Wire spelling.** `SetTextStyle` takes `font: AnnotFont` (a standard family or a blob ID, as `AddText` does) and `color`, not the plan's `FontRef` and `colour`; the plan's table says so.
- **Bold and italic pick a face.** With `font` a standard family, or a run in one of the standard 14 fonts, they pick the family's face; a font given by its bytes is one face, so the caller names the bold face. Synthetic bold and italic, which the scope allows only as a fallback, are [#845](https://github.com/laudebugs/laminapdf/issues/845); until then a run with no face to take gets `invalid-op`.
- **Underline** is a path drawn after the text object. Lamina PDF can't tell it from other lines, so `underline: false` is refused.
- **Alignment and spacing** move whole lines of the run's text object with a `Td` and its inverse, so what follows keeps its place: a line aligns to the widest extent of the object's lines, and spacing is a multiple of the glyphs' size, measured from the first of the runs' lines. Text drawn mirrored (a text matrix that flips it) is refused with `content-not-editable` (`rotated-run`), as is a line that starts, or text that follows a shifted line, without a line move of its own, since a `Td` restarts from the line's start. `justify` is refused: it needs word spacing per line, which this PR leaves to E7l.
- **Size.** About 3,200 lines, over the L band: the content-crate rewrite, the font chain, the style op, and the WASM, SDK and viewer surfaces are one change, and splitting them would leave a façade method with nothing behind it.
- **Widths.** The report and the underline's length come from what the rewrite measures (`Rewritten`, in text space, scaled to points by the run's old length), so `Tc`, `Tw` and `Tz` count. `Effect` and `Edited` keep `Eq` (`TextEditReport` implements it by hand, `grew_by` never being NaN).
- **Open boxes.** Library parity is ticked once CI shows the SDK and viewer packages typecheck and pass (they were not run locally); the pdf.js, PDFium and qpdf round-trip box is ticked once the full lane's round-trip legs pass, which now run a restyle with font, bold, size, colour, underline and superscript, and an `editText` with `fonts`; the editor's notice for a substitute font is E7l's.
- **Not in this PR.** Synthetic bold and italic ([#845](https://github.com/laudebugs/laminapdf/issues/845)). The comment summary drawing a page with substitutes ([#846](https://github.com/laudebugs/laminapdf/issues/846)); desktop system-font lookup for the full-font step ([#847](https://github.com/laudebugs/laminapdf/issues/847), in the desktop engine); the WASM size budgets, which need a WebAssembly build to measure ([#848](https://github.com/laudebugs/laminapdf/issues/848)); the editor's notice and Paragraph inspector, which are [E7l](../ready/E7l-inline-text-editor.md)'s.

## Post-merge checks

None.

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

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
