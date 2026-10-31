# E1d — Edit session, interaction modes, undo/redo UI and history panel

| Field | Value |
| --- | --- |
| ID | `E1d` |
| Status | `ready` |
| Plan | [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B6b](../done/B6b-lamina-pdf-package.md), [C4a](./C4a-ui-chrome-and-platform.md), [D4b](./D4b-native-menus.md), [E1a](./E1a-editing-foundation-feature-plan.md) |
| Blocks | [E1e](./E1e-save-and-constraint-banner.md), [E4b](./E4b-annotation-model-and-overlay.md) |
| Dependency notes | [E1a](E1a-editing-foundation-feature-plan.md), `C4` ([web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare): `@laminapdf/ui`, `apps/web`), `D4` ([desktop-app §D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing): windows, tabs, menus), `B6` ([pdf-library §B6](../../plans/pdf-library.md#b6-wasm-and-laminapdfsdk): `lamina-wasm` and `@laminapdf/sdk`) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1d-edit-session-undo-ui` |
| PR title | `feat(ui): edit session with undo/redo and history panel` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Turn the readers into an editor shell: an edit session per open document that holds the op log in the worker/engine process, the interaction-mode state machine in `@laminapdf/ui` that later rounds add tools to, dirty state, undo/redo commands with shortcuts and menus on both shells, and a history panel listing transactions.

## Context

- **Design (Paper):** [O01 Edit session and history](../../design/paper/edit-session-organise.md#o01-edit-session-and-history). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor §6](../../plans/full-editor.md#6-design-summary): interaction modes as one state machine; tools as small modules over `@laminapdf/viewer` layers
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §4: undo per document, local origin only, `UndoOutcome::Partial`; §7 two dirty levels
- [web C3](../../plans/web-app.md#c3-viewer-core) and [desktop D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing): the worker RPC, menus and accelerators this extends
- [editing notes §2](../../research/architecture/notes/editing-architecture.md#2-command-pattern--operation-log-undoredo-transactions-dirty-tracking): command pattern, transactions, coalescing

## Scope

**In scope**

- `EditSession` in the worker RPC (web) and engine-process commands (desktop): `apply(ops)`, `undo()`, `redo()`, `history()`, dirty events with touched pages
- Interaction-mode state machine in `@laminapdf/ui`: `select`, `hand`, `textSelect` now; typed extension points for `annotate`, `formFill`, `formDesign`, `contentEdit`, `redact`, `measure`
- Undo/redo toolbar buttons, keyboard shortcuts (Cmd/Ctrl+Z, Shift+Cmd+Z / Ctrl+Y), native menu items on desktop (shared shortcut map test from D4)
- History panel: transaction list with labels, current position, `Partial`/`Skipped` outcomes surfaced as toasts
- Unsaved indicator (semantic dirty) in the tab/title; close/quit guards on both shells
- Re-render only touched pages reported by `Applied`

**Out of scope** (and which todo covers it)

- Saving and the constraint banner → [E1e](E1e-save-and-constraint-banner.md)
- Metadata editing UI → [E1f](E1f-metadata-ui-capability-page.md)

**Editor facts**

- **Crates and packages:** `lamina-wasm` (session bindings), `apps/desktop/src-tauri` (engine commands), `@laminapdf/viewer` (touched-page invalidation), `@laminapdf/ui`
- **`Op` variants:** none new (uses B3 `RotatePage` and E1c ops to exercise it)
- **`SaveConstraint`:** no saving here; dirty state follows ADR 0006 §7
- **Web vs desktop:** identical behaviour. Desktop: native Edit menu items and window-close guard (Tauri `CloseRequested`); web: `beforeunload` guard
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none flipped (infrastructure)

## Tasks

- [ ] Add session RPC methods and events to the worker protocol (ADR 0009 types) and desktop engine commands (load `/tauri-v2` skill first)
- [ ] Implement the mode state machine (XState-like reducer or hand-written; no new dependency without licence check) with tests for every transition
- [ ] Wire undo/redo commands, shortcuts, menus; disable when stacks are empty; announce outcomes via ARIA live region
- [ ] History panel component with virtualised list; clicking an entry undoes/redoes to that point
- [ ] Dirty indicator and close guards on web and desktop
- [ ] Invalidate and re-render touched pages only; measure and log time per invalidation

## Acceptance criteria

- [ ] Playwright (web) and WebdriverIO (desktop) e2e: rotate a page, undo, redo, check rendering and dirty indicator each step
- [ ] Undo after 500 transactions drops the oldest (depth limit) and the panel reflects it
- [ ] Closing a dirty document prompts on both shells; closing a clean one does not
- [ ] Keyboard-only and screen-reader pass of the history panel recorded (axe: zero serious violations)

## Post-merge checks

None.

## Tests

- Unit tests for the mode reducer and session store (Vitest)
- Worker RPC contract tests
- e2e in `apps/web/e2e/edit-session.spec.ts` and `apps/desktop/e2e/edit-session.e2e.ts`

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
