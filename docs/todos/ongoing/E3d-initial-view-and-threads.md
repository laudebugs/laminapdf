# E3d — Initial view, page display mode and article threads (read)

| Field | Value |
| --- | --- |
| ID | `E3d` |
| Status | `ongoing` |
| Plan | [E3 plan §5](../../plans/editor/e3-navigation-attachments-inspection.md#5-the-viewer-and-the-initial-view) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3d-initial-view-and-threads` |
| PR title | `feat(ops): initial view and viewer preferences; read article threads` |
| Size | M (200–400): one op, the catalog and threads views, the viewer's initial view and the dialog |
| PR | [#516](https://github.com/laudebugs/laminapdf/pull/516) |

## Goal

Edit `/OpenAction` (initial page and zoom), `/PageMode`, `/PageLayout` and `/ViewerPreferences`, honour them on open as the [E3 plan §5](../../plans/editor/e3-navigation-attachments-inspection.md#5-the-viewer-and-the-initial-view) describes, and read article threads (list and navigate beads).

## Context

- **Design (Paper):** [O13 Initial view and article threads](../../design/paper/pages-navigation-extraction.md#o13-initial-view-and-article-threads). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §1.5 viewer preferences, page mode/layout](../../research/pdf/04-interactive-security.md#15-page-labels-thumbnails-viewer-preferences-page-modelayout)
- ISO 32000-2 §12.2 (viewer preferences), §12.4.3 (articles)

## Scope

**In scope**

- `SetInitialView { open, page_mode, page_layout, preferences }` op; an `/OpenAction` that is another action is shown and kept, never run (editing it is [E17b](../ready/E17b-action-editor.md)'s)
- A catalog view reading the open destination, page mode, page layout and viewer preferences
- Initial-view dialog in `@laminapdf/ui`
- Article-thread view in `laminapdf-model` and a read-only threads panel

**Out of scope** (and which todo covers it)

- Editing article threads → E15's feature plan, [E15a](../ready/E15a-accessibility-feature-plan.md) ([E3 plan §11](../../plans/editor/e3-navigation-attachments-inspection.md#11-decisions-on-the-open-questions))
- The desktop column → [E3k](../ready/E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.06` initial view and zoom; page display mode; `I.09` article threads; `I.11` outline and navigation preferences
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `@laminapdf/ui`
- **`Op` variants:** `SetInitialView` (class `Document`)
- **`SaveConstraint`:** as E3b
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.06` → `round-trip` (core, pdf, web); `I.09` → `read` (core, pdf, web); `I.11` → `round-trip` (core, pdf, web)

## Tasks

- [ ] Op and dialog
- [ ] Apply initial view on open in `@laminapdf/viewer`
- [ ] Threads view and panel

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Opening the saved file in pdf.js honours page mode/layout and open action

## Post-merge checks

None.

## Tests

- Unit, round-trip, e2e

## Docs to update

- API docs
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
