# C4a — `@laminapdf/ui`: React 19 reader chrome and the `Platform` adapter

| Field | Value |
| --- | --- |
| ID | `C4a` |
| Status | `ready` |
| Plan | [web reader §6 C4. App shell on Cloudflare](../../plans/web-app.md#c4-app-shell-on-cloudflare) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C2a](../done/C2a-render-adrs.md), [C3b](../ongoing/C3b-viewer-virtualiser-and-zoom.md), [U1a](./U1a-adr-0024-app-shell-ux.md), [U6e](./U6e-overlays-and-sheets.md) |
| Blocks | [C4b](./C4b-web-app-shell.md), [D2a](./D2a-adr-0008-and-desktop-scaffold.md), [D2d](./D2d-tile-transport-and-first-document.md), [U1b](./U1b-command-registry-and-palette.md), [U1c](./U1c-inverted-l-shell-and-modes.md), [E1d](./E1d-edit-session-undo-ui.md), [E10e](./E10e-timestamps-and-tsa-proxy.md) |
| ADRs | Implements 0009 (React chrome around the framework-free viewer) and 0010 (`Platform` interface, capability flags, web implementation) |
| Branch | `feat/c4a-ui-chrome` |
| PR title | `feat(ui): add the react reader chrome and the platform adapter interface` |
| Size | L (600–1,500 lines incl. tests) |
| PR | — |

## Goal

Build the reusable reader UI both shells share: `@laminapdf/ui`, a React 19 (React Compiler) package with the toolbar, page indicator, zoom controls, open affordances, error/repair banners and password dialog, driven by `@laminapdf/viewer`'s store through `useSyncExternalStore`; and the `Platform` interface from ADR 0010 with its **web** implementation. UI code depends only on capability flags, so the desktop app (D2) can drop in its own `Platform` without touching components.

## Context

- **Design (Paper):** [F01](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.1–§5.2](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2) — `ReaderShell` must expose the left-rail, inspector, secondary-toolbar and status slots that [U1c](./U1c-inverted-l-shell-and-modes.md) fills; ADR 0024 ([U1a](./U1a-adr-0024-app-shell-ux.md)) fixes the layout first, so this todo depends on it; toolbar buttons become command ids in [U1b](./U1b-command-registry-and-palette.md), so keep handlers in `ReaderController`, not in components.
- ADR 0009 and ADR 0010 (from [C2a](../done/C2a-render-adrs.md)).
- [Research: the web client](../../research/architecture/app-and-package-architecture.md#the-web-client-is-react-chrome-around-an-imperative-viewer-core) — React 19 + Compiler; interaction modes in a state machine; view state in an external store; `Platform` flags (`canWriteInPlace`, `canWatchFiles`, `hasNativePrint`, `multiWindow`).
- [web-frontend notes](../../research/architecture/notes/web-frontend.md) §1 (framework, accessible primitives: React Aria / Radix / Base UI), §4 (state), §7 (a11y, theming via CSS custom properties).
- [web-app.md §4 functional 1, 8, 10](../../plans/web-app.md#functional) — open affordances, keyboard, error handling.
- [TypeScript guidelines](../../guidelines/typescript.md); [code organization §2](../../guidelines/code-organization.md#2-boundaries-and-dependency-direction) (packages never import apps).

## Scope

**In scope**

- The reader shell in `packages/ui` (`@laminapdf/ui`), which [U6d](./U6d-core-components.md) creates (package scaffold, React 19, tests, Biome config) and [U6e](./U6e-overlays-and-sheets.md) extends; C4a adds the shell and platform modules to it and does not re-scaffold the package.
- Accessible primitives library chosen per ADR 0009 (recommendation: React Aria Components for i18n/RTL/keyboard) — licence checked (Apache-2.0).
- Components: `ReaderShell` (layout: toolbar, optional sidebar slot, viewer slot, status region), `Toolbar` (open, page indicator with input, prev/next, zoom out/in, zoom menu with fit width / fit page / actual size / presets, rotate, download/print placeholders disabled by capability, overflow menu), `OpenDropZone` (empty state with "Open a file", drag-and-drop target, paste hint), `PasswordDialog`, `ErrorBanner` / `RepairNotice` (ADR 0003 warnings in plain words, list of unsupported features), `LoadingState`, `DebugMenu` slot (backend switch hayro/PDFium for diffing — hidden behind `#debug`, a fragment per ADR 0011 §3.6).
- `useViewer(viewer)` hooks over the store (`useSyncExternalStore`), `ReaderController` that owns the interaction state machine (`empty → opening → needs-password → open → error`).
- `Platform` interface + `createWebPlatform()`: `openFiles()` (`showOpenFilePicker` when available, else hidden `<input type=file>`), `saveAs(bytes, name)` (download via `Blob` URL; `showSaveFilePicker` where available), `print(job)` (stub until C6c), `copyText`, `openExternal(url)`, capability flags with web values, `storage` (OPFS/IndexedDB handles, used by C7b).
- Theming foundation: CSS custom properties tokens (light/dark/high-contrast hooks used by C7a), no CSS-in-JS runtime.
- i18n foundation: all strings through a tiny message catalogue (`en` only in v0.1), no hard-coded strings in components.

**Out of scope** (and which todo covers it)

- `apps/web`, Vite config, Cloudflare, actual file opening flows → [C4b](./C4b-web-app-shell.md), [C4c](./C4c-remote-files-encryption-privacy.md).
- Sidebars (thumbnails, outline) → [C6a](./C6a-thumbnails-outline-page-labels.md); find bar → [C5b](./C5b-find-in-document.md); properties dialog, print → [C6c](./C6c-print-download-properties.md).
- Full keyboard map, help dialog, dark/high-contrast themes, landmarks polish → [C7a](./C7a-accessibility-and-theming.md) (this PR makes every control focusable and labelled).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add the shell and platform modules to `packages/ui` as created by U6d (no new scaffold; extend its `exports`).
- [ ] Check licences of React, React Aria (or the chosen primitives) and icons (the Paper set in `@laminapdf/icons`, [U6c](./U6c-icon-set.md)); `cargo xtask npm-licenses` passes.
- [ ] `src/platform/types.ts`: `Platform`, `Capabilities` exactly as ADR 0010; `src/platform/web.ts`: web implementation with feature detection (no UA sniffing); `src/platform/context.tsx`: `PlatformProvider`, `usePlatform()`, `useCapability(flag)`.
- [ ] `src/state/reader-machine.ts`: a small typed state machine (hand-written or a tiny lib; justify) for the reader lifecycle and interaction mode (`select` only in v0.1); unit tests for every transition.
- [ ] `src/hooks/use-viewer.ts`: `useViewerState(selector)` with `useSyncExternalStore`; no re-render on every scroll frame (selectors return stable primitives).
- [ ] Components listed in scope, each with an accessible name, visible focus ring, tooltip with shortcut text, and a story-like test page (`examples/ui-demo.tsx`) using a fake viewer.
- [ ] Page indicator: shows `n / total` (page labels arrive in C6a — design the prop now), accepts typed page numbers, Enter jumps, invalid input is announced.
- [ ] Zoom control: percentage input + menu; keyboard operable; reflects viewer state.
- [ ] Password dialog: masked input, show/hide toggle, wrong-password error message, cancel → back to empty state; never stores or logs the password; `autocomplete="off"`.
- [ ] Error/repair UI: maps ADR 0003 warning and error codes (`fixtures/codes.json`) to messages; "repaired" notice is dismissible; unsupported-feature list expandable.
- [ ] Use `@laminapdf/tokens` ([U6a](./U6a-paper-tokens.md)) for all styles and compose the shell from `@laminapdf/ui`'s Paper components and overlays ([U6d](./U6d-core-components.md), [U6e](./U6e-overlays-and-sheets.md)); the password prompt uses U6e's dialog. Respect `prefers-color-scheme` and `prefers-reduced-motion` (full themes in C7a).
- [ ] README with the component map and how desktop supplies its own `Platform`.

## Acceptance criteria

- [ ] Component tests (Vitest browser mode, three engines) cover: open button calls `platform.openFiles`; drop zone accepts a dropped PDF and rejects non-PDF with a message; password dialog submit/cancel/wrong-password; toolbar reflects and changes viewer zoom/page; capability-gated buttons hidden or disabled correctly for web values.
- [ ] No component imports anything from `apps/` or reads `window.__TAURI__`/UA strings (lint/test check); UI branches only on `Capabilities`.
- [ ] axe-core run on the demo page reports zero serious/critical violations for the chrome.
- [ ] All user-visible strings come from the message catalogue.
- [ ] React Compiler builds the package's demo without bail-out warnings for components in `src/components/` (or bail-outs are justified in code comments).

## Post-merge checks

None.

## Tests

`packages/ui/src/**/*.test.tsx` (Testing Library, Vitest browser mode); state-machine unit tests; axe check on `examples/ui-demo`; fake `Viewer` and fake `Platform` in `packages/ui/test/fakes.ts`.

## Docs to update

`packages/ui/README.md`; `CLAUDE.md` layout (`packages/ui`); tick "Create `packages/ui` (React 19, React Compiler) with the `Platform` interface and web implementation" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **React Compiler + Rolldown/Vite 8 integration** maturity; fall back to Babel plugin in Vite if needed and note it.
- **Primitives choice** (React Aria vs Radix/Base UI) is a long-lived dependency; ADR 0009 should name it; if it doesn't, decide here and amend 0009.
- **Shortcut conflicts** with browser shortcuts (ctrl+F, ctrl+P) are settled in C5b/C6c/C7a; this PR only reserves the plumbing.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
