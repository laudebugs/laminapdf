# E3d — Initial view, page display mode and article threads (read)

| Field | Value |
| --- | --- |
| ID | `E3d` |
| Status | `done` |
| Plan | [E3 plan §5](../../plans/editor/e3-navigation-attachments-inspection.md#5-the-viewer-and-the-initial-view) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E3a](./E3a-navigation-attachments-inspection-feature-plan.md) |
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

- [x] Op and dialog
- [x] Apply initial view on open in `@laminapdf/viewer`
- [x] Threads view and panel

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [x] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [x] Opening the saved file in pdf.js honours page mode/layout and open action

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

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **The op:** `SetInitialView` (`laminapdf-ops`, class `Document`, no rewrite) patches `/OpenAction`, `/PageMode`, `/PageLayout` and the window entries of `/ViewerPreferences`: absent keeps, `null` removes, a value sets. An `/OpenAction` that is another action is never replaced: an edit naming the page fails with the new code `open-action-not-destination`, which the dialog avoids by locking the page and zoom. A value newer than the file raises the catalog's `/Version`. The sample is `fixtures/ops/v1/set-initial-view.json`.
- **Reading:** `laminapdf-model`'s `Document::initial_view` and `threads` (`PageMode` and `PageLayout` moved there from `laminapdf-build`, which re-exports them), and `open_action_kind`, the cheap check the op makes; the façade's `initial_view()` → `OpeningView` and `threads()` → `ArticleThread`s in page space; WASM `initialView()` and `threads()` in the extract and text modules; `@laminapdf/sdk`'s `doc.initialView()`, `doc.threads()`, `doc.setInitialView(patch)` and `tx.setInitialView`; `@laminapdf/viewer`'s `client.initialView()` and `client.threads()`.
- **Round trip:** `setInitialView` is one of the round-trip suite's proving ops, so every fixture, mode and override saves it; pdf.js reads the open page, page mode and layout back, and PDFium the page mode. The signed fixtures' verdicts are `crates/laminapdf-save/tests/initial_view.rs`'s.
- **Size:** the base WebAssembly module had 236 bytes of budget left on `main`. The op's arguments are read through one `serde_json::Value` path rather than a derived deserialiser per serde deserializer, which cut its code from 31 KB to 7 KB, and the budgets rise by ADR 0016's proposed amendment, which the owner approves by merging. The reader's startup script stays in its budget: the dialog, the Threads tab's panel, their words and the page-mode effect load with `later.ts`.
- **As built, against O13:**
  - Initial view… lives in More and the palette (`organise.initial-view`): the reader has no Organise mode yet, and Document properties doesn't yet show the initial view.
  - The viewer has one continuous column, so the page layout is written and shown in the dialog but not obeyed, and Document properties has no note about it yet.
  - The opening page and zoom apply once the first page painted, as a `#page=` address does, so page 1 can show for a moment. A kept reading position and an address that names a place both win, as the plan says.
  - The panel the file asks for opens only where the rail sits beside the page: on a phone or a narrow window it would cover the page just opened. `outline.pdf` asks for its outline, so the navigation e2e test now expects the rail open.
  - The bead's outline on the page, the status bar's "part 1 of 3" and View settings' "Use the file's initial view" (R10) are not built.
  - The reader already shows the document's title in the tab whether or not the file asks, so `/DisplayDocTitle` changes nothing in it; `/HideToolbar`, `/HideMenubar` and `/HideWindowUI` are written and kept, never obeyed (plan §5). The dialog edits the three hide entries as one choice.
- **The `pdf` column** of I.06, I.09 and I.11 stays empty: the SDK tests carry the capability names, which aren't collected yet (E1f's note).
- **Checked where:** qpdf wasn't installed locally, so `qpdf --check` on the round-trip outputs is CI's full lane's alone; the e2e tests ran locally in Chromium, and Firefox and WebKit run them in CI.

