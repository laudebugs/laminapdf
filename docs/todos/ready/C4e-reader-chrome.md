# C4e — `@laminapdf/ui`: the reader shell, the top bar and the password prompt

| Field | Value |
| --- | --- |
| ID | `C4e` |
| Status | `ready` |
| Plan | [web reader §6 C4. App shell on Cloudflare](../../plans/web-app.md#c4-app-shell-on-cloudflare) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C3b](../done/C3b-viewer-virtualiser-and-zoom.md), [C4a](../done/C4a-ui-chrome-and-platform.md), [U1a](../done/U1a-adr-0024-app-shell-ux.md), [U6e](../done/U6e-overlays-and-sheets.md) |
| Blocks | [C4f](./C4f-reader-home-repairs-and-demo.md), [U1b](./U1b-command-registry-and-palette.md), [U1c](./U1c-inverted-l-shell-and-modes.md), [E1d](./E1d-edit-session-undo-ui.md) |
| ADRs | Implements 0009 (React chrome around the framework-free viewer); follows 0010 (UI branches only on capability flags) and 0024 (the shell's slots) |
| Branch | `feat/c4e-reader-chrome` |
| PR title | `feat(ui): add the reader shell, the top bar and the password prompt` |
| Size | L (≈ 1,400 lines of components, tokens and tests; split from C4a on 2026-11-05, then the home screen's parts, the repair and failure UI, the demo page and the React Compiler moved to [C4f](./C4f-reader-home-repairs-and-demo.md)) |
| PR | — |

## Goal

Build the frame of the reader both shells share, in `@laminapdf/ui`: the shell layout, the top bar with its page indicator and zoom control, and the password prompt. The chrome reads `@laminapdf/viewer`'s store and [C4a](../done/C4a-ui-chrome-and-platform.md)'s `ReaderController` through `useStoreState`. It asks the `Platform` only for capability flags, so the desktop app (D2) drops in its own `Platform` without touching a component. This todo was split from C4a when C4a was claimed, because the platform layer and the chrome together passed the 1,500-line budget; [C4f](./C4f-reader-home-repairs-and-demo.md) builds the rest of the chrome on this frame.

## Context

- **Design (Paper):** [F01](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [C4a](../done/C4a-ui-chrome-and-platform.md): `Platform`, `PlatformProvider`, `useCapability`, `ReaderController` and its machine, `useStoreState`, the message catalogue and `test/fakes.ts`'s `fakePlatform`.
- [UI/UX plan §5.1–§5.2](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2) — `ReaderShell` must expose the left-rail, inspector, secondary-toolbar and status slots that [U1c](./U1c-inverted-l-shell-and-modes.md) fills; toolbar buttons become command ids in [U1b](./U1b-command-registry-and-palette.md), so keep handlers in `ReaderController`, not in components.
- [web-frontend notes](../../research/architecture/notes/web-frontend.md) §1 (accessible primitives), §7 (a11y, theming via CSS custom properties).
- [web-app.md §4 functional 1, 8, 10](../../plans/web-app.md#functional) — open affordances, keyboard, error handling.
- [TypeScript guidelines](../../guidelines/typescript.md); [code organization §2](../../guidelines/code-organization.md#2-boundaries-and-dependency-direction) (packages never import apps).

## Scope

**In scope**

- `ReaderShell` (layout: top bar, secondary toolbar, rail, canvas, inspector and status slots) and `ReaderTopBar` (design language §7.1's Reader v0.1 variant: sidebar toggle, file name with "Loading pages…" or "Locked", page input, zoom, Commands, More), `PageInput`, `ZoomControl` (out, the percentage with a menu of fit width / fit page / actual size / presets, in), `useGatedEntry` for capability-gated menu items.
- `PasswordDialog` (R03).
- Every string through C4a's message catalogue; styles from `@laminapdf/tokens` only.

**Out of scope** (and which todo covers it)

- The home screen's drop zone, the repair and failure UI, the loading state, the debug menu, the demo page and the React Compiler → [C4f](./C4f-reader-home-repairs-and-demo.md).
- The `Platform`, the reader machine and controller, the hooks and the catalogue → [C4a](../done/C4a-ui-chrome-and-platform.md).
- History buttons (C6b), Find (C5b), View and Reader (U4b), Undo/Redo and the mode switcher (E1d).

## Tasks

- [ ] The shell and the top bar's components, each control focusable and labelled.
- [ ] Page indicator: shows `n / total`, a page label with its position (`A-1 (22 / 24)`), accepts typed page numbers or labels, Enter jumps, invalid input is announced.
- [ ] Zoom control: steps and a menu; keyboard operable; reflects viewer state.
- [ ] Password dialog: masked input, show/hide toggle, wrong-password error message, cancel → back to empty state; never stores or logs the password.
- [ ] README: the reader chrome's map.

## Acceptance criteria

- [ ] Component tests (Vitest browser mode, three engines) cover: password dialog submit/cancel/wrong-password; toolbar reflects and changes viewer zoom/page; capability-gated items hidden or disabled correctly.
- [ ] The top bar's states pass axe in the three themes and both directions, and meet the target sizes.
- [ ] All user-visible strings come from the message catalogue.

## Post-merge checks

None.

## Tests

`packages/ui/test/browser/*.browser.test.tsx` (Vitest browser mode, three engines); axe check on `examples/ui-demo`; a fake `Viewer` beside `fakePlatform` in `packages/ui/test/fakes.ts`.

## Docs to update

`packages/ui/README.md`; `CLAUDE.md` layout (`packages/ui`).

## Risks and open questions

- **Shortcut conflicts** with browser shortcuts (ctrl+F, ctrl+P) are settled in C5b/C6c/C7a; this PR only reserves the plumbing.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
