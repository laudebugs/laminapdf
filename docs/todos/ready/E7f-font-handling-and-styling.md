# E7f — Font handling: re-subset, substitution, font/size/colour/style, alignment and spacing

| Field | Value |
| --- | --- |
| ID | `E7f` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6c](./E6c-add-text.md), [E7e](./E7e-text-runs-and-inline-edit.md) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7f-font-handling-and-styling` |
| PR title | `feat(text): font substitution and text styling for edited runs` |
| Size | L (600–1,500) |
| PR | — |

## Goal

When the original subset lacks glyphs, add a new subset of the same font (if the full font is available) or substitute (bundled/system font, metrics-matched); change font family/size/colour/style, bold/italic/underline/super/subscript, alignment and line spacing for edited lines.

## Context

- **Design (Paper):** [A04 Edit existing text inline](../../design/paper/add-edit-content.md#a04-edit-existing-text-inline). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [03 §6 Font embedding and subsetting](../../research/pdf/03-text-and-fonts.md#6-font-embedding--subsetting-from-code)
- [03 §7 Layout](../../research/pdf/03-text-and-fonts.md#7-text-layout--shaping--everything-pdf-does-not-do-for-you)

## Scope

**In scope**

- Font resolution chain
- Style ops: `SetTextStyle { runs, style }`
- Line-level layout for alignment/spacing
- Synthetic bold/italic only as fallback
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
- **Capability-matrix rows to flip:** `A.02`, `A.04`, `A.05` → `round-trip` (core, pdf); `A.06` → `write` (core, pdf); web and desktop in [E7l](./E7l-inline-text-editor.md)

## Tasks

- [ ] Resolution chain
- [ ] Style op
- [ ] Layout
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Editing text that needs new glyphs succeeds on fixtures with subset fonts and the result extracts correctly
- [ ] An edit that needs a substitute font reports the missing characters and the substitute's name, in the op report and in the editor, before it is committed; undo restores the original run

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
