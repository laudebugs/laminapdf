# C4a — `@laminapdf/ui`: the `Platform` adapter, the reader controller and the message catalogue

| Field | Value |
| --- | --- |
| ID | `C4a` |
| Status | `done` |
| Plan | [web reader §6 C4. App shell on Cloudflare](../../plans/web-app.md#c4-app-shell-on-cloudflare) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C2a](./C2a-render-adrs.md), [C3b](./C3b-viewer-virtualiser-and-zoom.md), [U1a](./U1a-adr-0024-app-shell-ux.md), [U6e](./U6e-overlays-and-sheets.md) |
| ADRs | Implements 0010 (`Platform` interface, capability flags, web implementation, rule 1's check) and 0009's state side (the viewer's store read through `useSyncExternalStore`) |
| Branch | `feat/c4a-ui-chrome` |
| PR title | `feat(ui): add the platform adapter, the reader controller and the message catalogue` |
| Size | L (≈ 1,700 lines, two-thirds of them tests; split on 2026-11-05 as its size required: the components, the demo page and the React Compiler moved to [C4e](../done/C4e-reader-chrome.md)) |
| PR | [#259](https://github.com/laudebugs/laminapdf/pull/259) |

## Goal

Build the layer under the reader chrome in `@laminapdf/ui`. It holds the `Platform` interface from ADR 0010 with its **web** implementation, the reader's lifecycle machine and the `ReaderController` that runs it, the store hooks, and the message catalogue. UI code depends only on capability flags, so the desktop app (D2) can drop in its own `Platform` without touching components. [C4e](../done/C4e-reader-chrome.md) builds the components on this layer.

## Context

- ADR 0009 and ADR 0010 (from [C2a](./C2a-render-adrs.md)).
- [Research: the web client](../../research/architecture/app-and-package-architecture.md#the-web-client-is-react-chrome-around-an-imperative-viewer-core) — interaction modes in a state machine; view state in an external store; `Platform` flags (`canWriteInPlace`, `canWatchFiles`, `hasNativePrint`, `multiWindow`).
- [web-frontend notes](../../research/architecture/notes/web-frontend.md) §4 (state), §6 (`showOpenFilePicker` not Baseline; input + drag-drop fallback).
- [UI/UX plan §5.1–§5.2](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2) — toolbar buttons become command ids in [U1b](../ready/U1b-command-registry-and-palette.md), so keep handlers in `ReaderController`, not in components.
- [TypeScript guidelines](../../guidelines/typescript.md); [code organization §2](../../guidelines/code-organization.md#2-boundaries-and-dependency-direction) (packages never import apps).

## Scope

**In scope**

- The platform and state modules in `packages/ui` (`@laminapdf/ui`), which [U6d](./U6d-core-components.md) creates and [U6e](./U6e-overlays-and-sheets.md) extends; no new scaffold.
- `Platform` interface + `createWebPlatform()`: `openFiles()` (`showOpenFilePicker` when available, else hidden `<input type=file>`), drops and pastes, the launch queue, `saveAs(bytes, name)` (download via `Blob` URL; `showSaveFilePicker` where available), `saveInPlace`, `print(job)` (stub until C6c), `copyText`, `openExternal(url)`, capability flags with web values, `storage` (OPFS/IndexedDB, used by C7b), recent files.
- `PlatformProvider`, `usePlatform()`, `useCapability(flag)`.
- `ReaderController`, which owns the reader's state machine (`empty → opening → needs-password → open → error`, `select` mode only), and the store hooks over `useSyncExternalStore`.
- i18n foundation: every platform string through a tiny message catalogue (`en` only in v0.1).
- ADR 0010 rule 1's check in `cargo xtask check ts`.

**Out of scope** (and which todo covers it)

- The components → [C4e](../done/C4e-reader-chrome.md); the demo page and the React Compiler → [C4f](../done/C4f-reader-home-repairs-and-demo.md).
- `apps/web`, Vite config, Cloudflare, actual file opening flows → [C4b](./C4b-web-app-shell.md), [C4c](../ready/C4c-remote-files-encryption-privacy.md).
- Print → [C6c](../ready/C6c-print-download-properties.md); offline storage → [C7b](../ready/C7b-offline-pwa.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] `src/platform/types.ts`: `Platform`, `Capabilities` exactly as ADR 0010; `src/platform/web.ts`: web implementation with feature detection (no UA sniffing); `src/platform/context.tsx`: `PlatformProvider`, `usePlatform()`, `useCapability(flag)`.
- [x] `src/state/reader-machine.ts`: a small typed state machine (hand-written or a tiny lib; justify) for the reader lifecycle and interaction mode (`select` only in v0.1); unit tests for every transition.
- [x] `src/state/hooks.ts`: `useStoreState(store, selector)` with `useSyncExternalStore`, for the viewer and the controller; no re-render on every scroll frame (selectors return stable primitives).
- [x] `src/messages.ts`: the catalogue and `message(id, values)`.
- [x] A fake `Platform` with any combination of flags in `packages/ui/test/fakes.ts`.
- [x] The platform boundary check in `cargo xtask check ts`.
- [x] README: how a shell supplies its `Platform`.

## Acceptance criteria

- [x] Unit tests cover every transition of the reader machine, and the controller's open, password, cancel, failure and superseded-open paths.
- [x] The web platform's tests cover its capability detection, `howToEnable`, both save paths, `saveInPlace`'s two gates, `openExternal`'s schemes and recent files, with fake windows and in three browsers.
- [x] No package imports `@tauri-apps/*`, calls `isTauri()` or reads Tauri's globals or UA strings (`cargo xtask check ts`); UI branches only on `Capabilities`.
- [x] All platform strings come from the message catalogue.

## Post-merge checks

None.

## Tests

`packages/ui/test/*.test.ts` (Node: the machine, the controller, the web platform over fake windows, the catalogue); `packages/ui/test/browser/platform.browser.test.tsx` (capability gates, re-renders, the real web platform); the boundary check's unit tests in `tools/xtask`.

## Docs to update

`packages/ui/README.md`; `CLAUDE.md` layout (`packages/ui`) and `check ts`. [C4f](../done/C4f-reader-home-repairs-and-demo.md) ticks the [web-app.md §9](../../plans/web-app.md#9-to-dos) to-do, which includes the React Compiler.

## Risks and open questions

- **Handles across reloads:** `recent.reopen` reads a handle kept in IndexedDB and asks for permission again; browsers differ in what they keep, so it exists only where `canReopenRecent`.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
