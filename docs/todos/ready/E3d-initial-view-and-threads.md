# E3d — Initial view, page display mode and article threads (read)

| Field | Value |
| --- | --- |
| ID | `E3d` |
| Status | `ready` |
| Plan | [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E3a](./E3a-navigation-attachments-inspection-feature-plan.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3d-initial-view-and-threads` |
| PR title | `feat(ops): initial view and viewer preferences; read article threads` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

Edit `/OpenAction` (initial page and zoom), `/PageMode`, `/PageLayout` and `/ViewerPreferences`, honour them on open in both apps, and read article threads (list and navigate beads).

## Context

- [04 §1.5 viewer preferences, page mode/layout](../../research/pdf/04-interactive-security.md#15-page-labels-thumbnails-viewer-preferences-page-modelayout)
- ISO 32000-2 §12.2 (viewer preferences), §12.4.3 (articles)

## Scope

**In scope**

- `SetInitialView { open_action, page_mode, page_layout, prefs }` op
- Initial-view dialog in `@lamina/ui`
- Article-thread view in `lamina-model` and a read-only threads panel

**Out of scope** (and which todo covers it)

- Editing article threads (deferred; plan says read now, edit later) → a later round decided in the E3 feature plan

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.06` initial view and zoom; page display mode; `I.09` article threads
- **Crates and packages:** `lamina-model`, `lamina-ops`, `@lamina/ui`
- **`Op` variants:** `SetInitialView` (class `Document`)
- **`SaveConstraint`:** as E3b
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.06` → `round-trip` (core, pdf, web, desktop); `I.09` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] Op and dialog
- [ ] Apply initial view on open in `@lamina/viewer`
- [ ] Threads view and panel

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Opening the saved file in pdf.js honours page mode/layout and open action

## Post-merge checks

None.

## Tests

- Unit, round-trip, e2e

## Docs to update

- API docs
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
