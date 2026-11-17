# E7i — Struct-tree and MCID maintenance; PDF/A and PDF/UA re-checks on save

| Field | Value |
| --- | --- |
| ID | `E7i` |
| Status | `ongoing` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B8b](../done/B8b-conformance-profiles-and-verapdf.md), [E7d](../done/E7d-select-transform-existing.md), [E7e](../done/E7e-text-runs-and-inline-edit.md) |
| Dependency notes | [E7d](../done/E7d-select-transform-existing.md) and [E7e](../done/E7e-text-runs-and-inline-edit.md) (the hooks sit in `EditContent` and `EditText`; `SetTextStyle` and `ReplaceText` reach them through `EditText`), `B8` ([pdf-library §B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa): tagged PDF, PDF/UA, PDF/A) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7i-structure-and-conformance-maintenance` |
| PR title | `feat(content): keep tagging valid across content edits` |
| Size | M (200–600) |
| PR | [#711](https://github.com/laudebugs/laminapdf/pull/711) |

## Goal

Keep tagged documents valid across E6/E7 edits: update MCIDs, ParentTree, `/ActualText` and `/Alt` when runs change or are deleted; re-check PDF/A and PDF/UA claims on save and warn.

## Context

- **Design (Paper):** [A07 Structure and conformance warning](../../design/paper/add-edit-content.md#a07-structure-and-conformance-warning). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [03 §9 Tagged PDF](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149)
- [library B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa) profiles

## Scope

**In scope**

- MCID/ParentTree maintenance hooks in `EditContent`/`EditText`
- Conformance checks reuse B8 profiles
- Warnings in the save banner
- `RemoveConformanceClaims { pdfa, pdfua }` (class `Metadata`), the only op that may remove `pdfaid:` and `pdfuaid:`; A07's Save without the claim applies it ([E7 plan](../../plans/editor/e7-editing-existing-content.md) §3, §10)
- A07's strip, claim badges and Check before saving dialog (moved to [E7k](../ready/E7k-select-transform-ui.md); see As built)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `laminapdf-model` (struct tree), `laminapdf-content`, `laminapdf-ops`, `laminapdf`, `@laminapdf/sdk`, `@laminapdf/ui`
- **`Op` variants:** `RemoveConformanceClaims` (class `Metadata`)
- **`SaveConstraint`:** as E7
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none new; protects E7 rows' `round-trip` on tagged fixtures

## Tasks

- [x] Hooks
- [x] Checks
- [x] Tests

## Acceptance criteria

- [x] veraPDF structure checks pass after edits on PDF/UA fixtures
- [x] Deleting a tagged run removes its struct element or MCR
- [x] `pending_conformance_warnings` covers content edits (an added font not embedded, a figure without alternative text, broken tags)
- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): `RemoveConformanceClaims` is exposed in `laminapdf` and `@laminapdf/sdk` with docs, an example and a serde sample frozen in `fixtures/ops/v1/`
- [x] `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) asserts `RemoveConformanceClaims`'s verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2)

## As built

- **`laminapdf-content`** (`edit/marked.rs`, `edit/edits.rs`): `ContentOps::drawn_mcids` (the MCIDs whose marked content still paints something), `object_mcids` and `object_draws`. `delete` now takes the marked-content sequence that holds only the deleted object with it (ADR 0022 §4).
- **`laminapdf-ops`** (`content/structure.rs`): the hooks. `EditContent` and `EditText` compare `drawn_mcids` before and after the edit; each MCID that went has its `/ParentTree` slot cleared and leaves its element's `/K` (an integer or an MCR on that page), an element left empty leaves its parent (climbing, the root kept), and the page's `/StructParents` and parent-tree key go when no slot is left. Deleting a run whose text object then shows nothing deletes the object, so its marked content goes too. A rewritten run puts its new text where the old text was in its element's `/ActualText` and `/Alt`. `ReplaceImage` on an image inside a `Figure` (through the role map) removes the figure's `/Alt` and `/ActualText`. A gone MCID the parent tree doesn't map, or tagged content on a page without `/StructParents`, flags the page.
- **Warnings** (`metadata/content_conformance.rs`, compiled with `content-edit`, since only content edits raise them), each checked against the document as it is, so an undo or a fix takes it back: `figure-without-alt` (PDF/UA-1; a replaced figure still without `/Alt` or `/ActualText`), `content-tags-broken` (PDF/UA-1; a flagged page that still draws an MCID its slots don't map), and `pdfa-font-not-embedded` with a new `pages` field (PDF/A; a font object new since the file was opened, named by an edited page's own `/Resources`, without a font program; E5e's `all_embedded` reused). Two new codes with their pages in `docs/errors/`.
- **`RemoveConformanceClaims { pdfa, pdfua }`** (`metadata/claims.rs`, class `Metadata`): removes every `pdfaid:` or `pdfuaid:` property through the packet editor; neither set is `invalid-op`. Frozen sample `fixtures/ops/v1/remove-conformance-claims.json`; the save planner's metadata verdicts on the signed fixtures include it. Façade `laminapdf::RemoveConformanceClaims`; SDK `doc.removeConformanceClaims({ pdfa, pdfua })`, `Transaction.removeConformanceClaims`, the worker method.
- **Example and docs:** `edit_tagged_pdf` (Rust and TypeScript) edits `pdf_ua_report`'s output, plans, removes the claims only when the edits broke them, and saves; byte golden, render goldens and docs preview; `cargo xtask verapdf` (and so the nightly) validates it as PDF/UA-1. The [Edit a tagged PDF](https://docs.laminapdf.com/recipes/edit-tagged-pdf/) recipe is the guide, linked from Edit a PDF.
- **Tests:** `laminapdf-ops`' `tests/structure.rs` (delete an object, delete every object, delete a run, rewrite a run's `ActualText`, replace a figure's image and undo, a broken parent tree, the claims op and its frozen sample), `laminapdf-content`'s `edit/marked.rs`, `laminapdf-save`'s `tests/metadata.rs` verdicts.

**Deviations.**

- **A07's UI moved to [E7k](../ready/E7k-select-transform-ui.md).** The strip, claim badges and Check before saving dialog sit in the Edit mode for existing content and point at its Image inspector ("Add alternative text"), which E7k builds; no reader surface edits existing content yet. This PR ships the library and SDK calls they use (`plan().conformanceWarnings`, `removeConformanceClaims`).
- **No op adds a font yet**, so the added-font check is exercised by its reuse of E5e's checks, not by a content op; [E7f](../ready/E7f-font-handling-and-styling.md)'s font chain is the first to add one.
- **Untagged content a reorder moves out of its marked content** isn't detected: `content-tags-broken` looks at MCIDs and their slots, not at real content left outside every sequence.
- The fixtures of plan §8's "Tagging and claims" are built in the tests (a hand-written tagged page) and taken from `pdf_ua_report`'s golden, rather than a `tools/fixtures/content_edit.py` file.
- **Size: the base and text modules need their budgets raised (owner's decision, ADR 0016 §6).** Measured on the Mac against `main` at the claim: base 953,863 → 956,838 B gzip (+2,975: `RemoveConformanceClaims`, two warning codes' tables, the warnings' new fields), extract 1,387,193 → 1,399,825 (inside its 1,400,000), text 2,089,852 → 2,102,223 (+12,371: the hooks, which content editing compiles into the text and extract modules). The content warnings are compiled only with `content-edit`, so the base module doesn't carry them. CI measures about 1.25 KB less, so base (953,000) and text (2,099,000) still fail; the proposal is base 957,400 and text 2,102,800 (the Mac's figure plus about 500 B, as before).
- `tools/render/Cargo.lock` was stale on `main` (`laminapdf-build`'s new `laminapdf-content` dependency), which `cargo xtask docs-previews` needs; this PR updates it.

## Post-merge checks

None.

## Tests

- veraPDF nightly
- Unit

## Docs to update

- Guide: editing tagged PDFs
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
