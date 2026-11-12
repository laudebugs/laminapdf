# E1d — Edit session, interaction modes, undo/redo UI and history panel

| Field | Value |
| --- | --- |
| ID | `E1d` |
| Status | `done` |
| Plan | [E1 editing foundation §5](../../plans/editor/e1-editing-foundation.md#5-the-edit-session-and-the-modes) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B6b](./B6b-lamina-pdf-package.md), [C4e](./C4e-reader-chrome.md), [E1a](./E1a-editing-foundation-feature-plan.md), [E1g](./E1g-redraw-after-edit.md) |
| Dependency notes | [E1g](../done/E1g-redraw-after-edit.md) (the worker's redraw after an edit, which the session builds on), `C4` ([web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare): `@laminapdf/ui`, `apps/web`), `B6` ([pdf-library §B6](../../plans/pdf-library.md#b6-wasm-and-laminapdfsdk): `laminapdf-wasm` and `@laminapdf/sdk`). The desktop part is [E1h](../ready/E1h-desktop-edit-session.md), after D4b ([E1 plan §8](../../plans/editor/e1-editing-foundation.md#8-differences-between-the-shells)) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1d-edit-session-undo-ui` |
| PR title | `feat(ui): edit session with undo/redo and history panel` |
| Size | L (600–1,500) |
| PR | [#412](https://github.com/laudebugs/laminapdf/pull/412) |

## Goal

Turn the web reader into an editor shell: an edit session per open document that holds the op log in the document worker, the interaction modes in `@laminapdf/ui` that later rounds add tools to ([E1 plan §5](../../plans/editor/e1-editing-foundation.md#5-the-edit-session-and-the-modes)), dirty state, undo/redo commands with shortcuts, and a history panel listing transactions. The desktop's session, Edit menu and close guard are [E1h](../ready/E1h-desktop-edit-session.md).

## Context

- **Design (Paper):** [O01 Edit session and history](../../design/paper/edit-session-organise.md#o01-edit-session-and-history). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor §6](../../plans/full-editor.md#6-design-summary): interaction modes as one state machine; tools as small modules over `@laminapdf/viewer` layers
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §4: undo per document, local origin only, `UndoOutcome::Partial`; §7 two dirty levels
- [web C3](../../plans/web-app.md#c3-viewer-core): the worker RPC this extends; [E1g](../done/E1g-redraw-after-edit.md): the redraw after an edit
- [editing notes §2](../../research/architecture/notes/editing-architecture.md#2-command-pattern--operation-log-undoredo-transactions-dirty-tracking): command pattern, transactions, coalescing

## Scope

**In scope**

- `EditSession` in the document worker's RPC: `apply(ops)`, `undo()`, `redo()`, `history()`, `plan(options)`, dirty events with touched pages
- Interaction modes in `@laminapdf/ui`: `select`, `hand`, `textSelect` now, as a registry of tool modules whose names later rounds extend by declaration merging (`annotate`, `formFill`, `formDesign`, `contentEdit`, `redact`, `measure`)
- Undo/redo toolbar buttons and keyboard shortcuts (Cmd/Ctrl+Z, Shift+Cmd+Z / Ctrl+Y), registered as `edit.undo` and `edit.redo` in the command registry, which D4b turns into desktop menu items later
- History panel: transaction list with labels, current position, `Partial`/`Skipped` outcomes surfaced as toasts
- Unsaved indicator (semantic dirty) in the tab/title; the `beforeunload` guard
- Re-render only touched pages reported by `Applied`, through E1g

**Out of scope** (and which todo covers it)

- Saving and the constraint banner → [E1e](../done/E1e-save-and-constraint-banner.md)
- Metadata editing UI → [E1f](../done/E1f-metadata-ui-capability-page.md)
- The desktop session, Edit menu and close guard → [E1h](../ready/E1h-desktop-edit-session.md)

**Editor facts**

- **Crates and packages:** `laminapdf-wasm` (session bindings), `@laminapdf/viewer` (the session in the worker), `@laminapdf/ui`, `apps/web`
- **`Op` variants:** none new (uses B3 `RotatePage` and E1c ops to exercise it)
- **`SaveConstraint`:** no saving here; dirty state follows ADR 0006 §7
- **Web vs desktop:** web only here; the desktop part is E1h
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none flipped (infrastructure)

## Tasks

- [x] Add session RPC methods and events to the worker protocol (ADR 0009 types)
- [x] Implement the mode state machine (XState-like reducer or hand-written; no new dependency without licence check) with tests for every transition
- [x] Wire undo/redo commands and shortcuts (desktop menus come from them in E1h); disable when stacks are empty; announce outcomes via ARIA live region
- [x] History panel component with virtualised list; clicking an entry undoes/redoes to that point
- [x] Dirty indicator and the web's close guard
- [x] Invalidate and re-render touched pages only; measure and log time per invalidation (`viewer.edits`' `lastMs`, from the call to the pages redrawn, shown under `#debug` in the status bar beside the first page's time)

## Acceptance criteria

- [x] Playwright e2e in Chromium, Firefox and WebKit: rotate a page, undo, redo, check rendering and dirty indicator each step
- [x] Undo after 500 transactions drops the oldest (depth limit) and the panel reflects it
- [x] Closing a dirty document prompts; closing a clean one does not
- [ ] Keyboard-only and screen-reader pass of the history panel recorded (axe: zero serious violations). Done: the keyboard-only pass (Tab reaches one row, Up, Down, Home, End, Page Up, Page Down and Enter, a Tab stop kept when the wheel scrolls; `history.browser.test.tsx`) and axe with no violations in light, dark and high contrast, both directions, in Chromium, Firefox and WebKit. Checked for a screen reader without one: the list is an `<ol>` named "Edit history" of buttons whose names are the step's label and meta, each item with `aria-posinset`/`aria-setsize` of the whole list, the current one `aria-current="step"`; Undo and Redo are named by their step with `aria-keyshortcuts`; the save state is `aria-live="polite"`; toasts are announced by the toast region's `role="status"`. Owed: a pass with VoiceOver and NVDA, which can't run here.

## Post-merge checks

None.

## Tests

- Unit tests for the mode reducer and session store (Vitest)
- Worker RPC contract tests
- e2e in `apps/web/e2e/edit-session.e2e.test.ts`

## Docs to update

- `@laminapdf/ui` README (modes and how tools register)
- Keyboard map/help dialog from C7 (new shortcuts)
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Keeping the op log in the worker while the UI shows history needs a projection; avoid sending full documents across the boundary
- Shortcut conflicts with browser defaults (Ctrl+Y in some browsers) — test in Chromium, Firefox, WebKit
- Found while building [E1g](../done/E1g-redraw-after-edit.md), for this todo to settle:
  - Undo and redo should redraw through E1g's path, but the façade's `undo`/`redo` drop the touched pages that `laminapdf-ops` records; expose them through the WASM module and `@laminapdf/sdk`.
  - E1g saves the model into memory after each edit, so `hasUnsavedChanges` reads false afterwards: the session needs its own dirty flag.
  - A failed in-memory save undoes the transaction, which leaves a redo entry behind; clear it.
  - A worker restart reopens the original file, so unsaved edits are lost; the session should replay its op log or warn.
  - An encrypted document can't take an edit, since its in-memory save is refused (`encrypted-base`); disable editing with the reason until E11.
  - After a page turns at a fixed zoom, a landscape page can be wider than the window; refit when the zoom mode is fit-width.
  - Each edit appends a revision whose cross-reference section PDFium reads on every reopen. Twenty edits in a row cost nothing measurable on 100 pages; decide whether a long session compacts the chain (a full rewrite into memory after some number of edits).
  - E1g's worker refuses an edit that changes the page count, since the view names pages by index; E2's page-structure ops need the view to map indices across an edit.

## What was built, and how E1g's follow-ups were settled

- **Serial edits.** The worker runs one edit call per document at a time (`serial` in `edit.ts`), so concurrent callers can't compute the same target, and an undo or redo reads the position only when its turn comes; the page's `EditSession` reads the history inside its queue too.
- **The session.** The document worker keeps a history per document (`packages/viewer/src/worker/session.ts`): a step per committed transaction with its label, pages and time, the position, 500 steps (the model's own undo depth), and the saved position. Its methods are `applyEdit` (with a label), `undoEdit`, `redoEdit`, `goToStep` (one redraw however many steps) and `editHistory`; the client fires `edit` after each, with the touched pages and the history. `viewer.edits` is the store the chrome reads. `plan(options)` and `save(options)` are E1e's: the SDK has no save plan yet, and E1d saves nothing.
- **Undo and redo redraw through E1g.** The façade's `undo` and `redo` now return `Reverted { outcome, touched_pages }`, through `laminapdf-wasm` and `@laminapdf/sdk` (a breaking change, under a minor changeset). `Edited.changed` says whether an edit made a step, so the session's steps stay the model's undo stack.
- **Its own dirty flag.** The session counts the steps between the document now and its saved position (the file as opened, until E1e saves), so E1g's in-memory saves don't clear it. Steps the depth limit drops still count.
- **The redo entry a failed save leaves.** Settled without a new API: the session records no step for it and drops its own undone steps (the model's transaction cleared them), so nothing ever redoes the stale entry, and the next edit clears it.
- **A worker restart.** The client keeps each step's ops (`edit-log.ts`) and replays them into the new worker (`restoreEdits`), then undoes the undone ones: the history and the pages come back as they were (`restarted`'s `edits: "restored"`). A replay can't recreate an undo or redo that restored only part of a step, or none of it, so after one of those, or if the replay fails, the file opens as it was and the reader says the changes were lost.
- **Encrypted documents** open with `encrypted: true`; the worker refuses their edits and every edit command is disabled with the reason, until E11.
- **Fit-width after a turn.** A fit mode fits again when a page changes size.
- **Compacting the revision chain: not needed yet.** Turning a page 400 times in a row through the reader's palette (Chromium, M1 Pro) took a median 100 ms per edit for edits 1–20 and for edits 381–400, most of it typing into the palette: the chain doesn't slow edits within a session's 500 steps. E1e's full rewrite on save is the natural place to compact.
- **Page-count-changing edits** are still refused; mapping indices across an edit is E2's.
- **Interaction modes** are `@laminapdf/ui`'s `ToolRegistry` (select, hand, textSelect; `InteractionModes` extended by declaration merging) and `attachTools`. The reader has no control to switch them yet: no screen draws one, so `select` stays on, as before.
- **Turning a page.** O06's page grid is E2's, so the reader turns the current page from More › Rotate page right or left (`pages.rotate-right`, `pages.rotate-left`, also in the palette), without the Organise grid's R and Shift R.

- **The startup script stays in its budget.** The edit session first put the reader's startup script at 278,123 bytes gzipped (main: 267,696; budget 270,000). What only an open document needs now loads after open, in one chunk (`apps/web/src/later.ts`): the new `@laminapdf/ui/editing` entry (the top bar's `EditArea`, the History tab, the edit commands, the tools) and `@laminapdf/ui/reader-mode` (reader mode's column, until now in the startup script), with Print and Properties as before. The startup script is now 269,870 bytes.

**Handed on to E1e** (its tasks list them): the session's `plan(options)`, the close confirmation's Save button, the History footer's sentence about Save, and Undo and Redo's tooltips (they belong with saving's work on the top bar, and loading the tooltip after open keeps the startup script in its budget).

**Deviations from O01 and O03:** closing a document with unsaved changes asks with Cancel and Close without saving only, since Save comes with E1e; on the web the Edit menu's items sit at the top of More; choosing a row in the History tab that undoes or redoes one step shows the same toast as Undo; Undo and Redo have no hover tooltip (their names carry the step and the keys), since the tooltip's code would have put the startup script over its budget.

**Checked in the built reader** in Chromium (light and dark), Firefox and WebKit: turn, undo, redo, the History tab by mouse and keys, the close guard. The gallery's screenshots (the History tab's new examples) can't be updated here, since `cargo xtask gallery --update` times out in Docker.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
