# E1d — Edit session, interaction modes, undo/redo UI and history panel

| Field | Value |
| --- | --- |
| ID | `E1d` |
| Status | `ready` |
| Plan | [E1 editing foundation §5](../../plans/editor/e1-editing-foundation.md#5-the-edit-session-and-the-modes) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B6b](../done/B6b-lamina-pdf-package.md), [C4e](../done/C4e-reader-chrome.md), [E1a](../done/E1a-editing-foundation-feature-plan.md), [E1g](../ongoing/E1g-redraw-after-edit.md) |
| Dependency notes | [E1g](../ongoing/E1g-redraw-after-edit.md) (the worker's redraw after an edit, which the session builds on), `C4` ([web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare): `@laminapdf/ui`, `apps/web`), `B6` ([pdf-library §B6](../../plans/pdf-library.md#b6-wasm-and-laminapdfsdk): `laminapdf-wasm` and `@laminapdf/sdk`). The desktop part is [E1h](./E1h-desktop-edit-session.md), after D4b ([E1 plan §8](../../plans/editor/e1-editing-foundation.md#8-differences-between-the-shells)) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1d-edit-session-undo-ui` |
| PR title | `feat(ui): edit session with undo/redo and history panel` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Turn the web reader into an editor shell: an edit session per open document that holds the op log in the document worker, the interaction modes in `@laminapdf/ui` that later rounds add tools to ([E1 plan §5](../../plans/editor/e1-editing-foundation.md#5-the-edit-session-and-the-modes)), dirty state, undo/redo commands with shortcuts, and a history panel listing transactions. The desktop's session, Edit menu and close guard are [E1h](./E1h-desktop-edit-session.md).

## Context

- **Design (Paper):** [O01 Edit session and history](../../design/paper/edit-session-organise.md#o01-edit-session-and-history). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor §6](../../plans/full-editor.md#6-design-summary): interaction modes as one state machine; tools as small modules over `@laminapdf/viewer` layers
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §4: undo per document, local origin only, `UndoOutcome::Partial`; §7 two dirty levels
- [web C3](../../plans/web-app.md#c3-viewer-core): the worker RPC this extends; [E1g](../ongoing/E1g-redraw-after-edit.md): the redraw after an edit
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

- Saving and the constraint banner → [E1e](E1e-save-and-constraint-banner.md)
- Metadata editing UI → [E1f](E1f-metadata-ui-capability-page.md)
- The desktop session, Edit menu and close guard → [E1h](./E1h-desktop-edit-session.md)

**Editor facts**

- **Crates and packages:** `laminapdf-wasm` (session bindings), `@laminapdf/viewer` (the session in the worker), `@laminapdf/ui`, `apps/web`
- **`Op` variants:** none new (uses B3 `RotatePage` and E1c ops to exercise it)
- **`SaveConstraint`:** no saving here; dirty state follows ADR 0006 §7
- **Web vs desktop:** web only here; the desktop part is E1h
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none flipped (infrastructure)

## Tasks

- [ ] Add session RPC methods and events to the worker protocol (ADR 0009 types)
- [ ] Implement the mode state machine (XState-like reducer or hand-written; no new dependency without licence check) with tests for every transition
- [ ] Wire undo/redo commands and shortcuts (desktop menus come from them in E1h); disable when stacks are empty; announce outcomes via ARIA live region
- [ ] History panel component with virtualised list; clicking an entry undoes/redoes to that point
- [ ] Dirty indicator and the web's close guard
- [ ] Invalidate and re-render touched pages only; measure and log time per invalidation

## Acceptance criteria

- [ ] Playwright e2e in Chromium, Firefox and WebKit: rotate a page, undo, redo, check rendering and dirty indicator each step
- [ ] Undo after 500 transactions drops the oldest (depth limit) and the panel reflects it
- [ ] Closing a dirty document prompts; closing a clean one does not
- [ ] Keyboard-only and screen-reader pass of the history panel recorded (axe: zero serious violations)

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

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
