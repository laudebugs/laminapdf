# C4f — `@laminapdf/ui`: the home screen's drop zone, repairs and failures, and the demo page

| Field | Value |
| --- | --- |
| ID | `C4f` |
| Status | `ready` |
| Plan | [web reader §6 C4. App shell on Cloudflare](../../plans/web-app.md#c4-app-shell-on-cloudflare) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C4e](./C4e-reader-chrome.md) |
| ADRs | Implements 0009 (React chrome around the framework-free viewer); follows 0003 (repairs in plain words) and 0010 (UI branches only on capability flags) |
| Branch | `feat/c4f-reader-home-repairs-and-demo` |
| PR title | `feat(ui): add the drop zone, the repair and failure ui, and the reader demo` |
| Size | L (≈ 900–1,200 lines of components, the demo page and tests; split from C4e on 2026-11-05) |
| PR | — |

## Goal

Finish the reader chrome on [C4e](./C4e-reader-chrome.md)'s frame: the home screen's drop zone (R01), the repair details panel and the failure page (R04) with a plain sentence for every repair and error code, the loading state (R02), the `#debug` menu, and a demo page that composes the whole chrome over a fake viewer, built with the React Compiler and checked with axe.

## Context

- **Design (Paper):** [R01](../../design/paper/reader-a.md#r01-home--open), [R02](../../design/paper/reader-a.md#r02-loading-states), [R04](../../design/paper/reader-a.md#r04-repaired--damaged-file).
- [C4a](../done/C4a-ui-chrome-and-platform.md)'s platform, controller and catalogue, and C4e's shell, top bar and `fakePlatform`/`FakeViewer` in `packages/ui/test/fakes.ts`.
- `fixtures/codes.json` for the warning and error codes (ADR 0003, ADR 0026).

## Scope

**In scope**

- `OpenDropZone` (R01: the zone, Choose a file, drop and paste anywhere in the window, a non-PDF refused with a message).
- `RepairDetails` and `OpenFailure` (R04), and the plain sentence for each code (`fixtures/codes.json`), a test holding every warning to one.
- `LoadingState` (R02's whole-file download card) and the `DebugMenu` slot (backend switch hayro/PDFium for diffing, behind `#debug`, a fragment per ADR 0011 §3.6).
- `examples/ui-demo.tsx` composing the chrome over a fake viewer; React Compiler for its build.
- README: the component map; tick the web-app plan's to-do.

**Out of scope** (and which todo covers it)

- Recent files (C7b), Open from URL and its errors (C4c), the app shell and real file opening (C4b).

## Tasks

- [ ] The components in scope, each control focusable and labelled, every string from the catalogue.
- [ ] Error/repair UI: maps ADR 0003 warning and error codes (`fixtures/codes.json`) to messages; "repaired" panel closes with Escape; unsupported-feature list.
- [ ] The demo page and its React Compiler build.

## Acceptance criteria

- [ ] Component tests (Vitest browser mode, three engines) cover: open button calls `platform.openFiles`; drop zone accepts a dropped PDF and rejects non-PDF with a message; the repair panel and the failure page.
- [ ] axe-core run on the demo page reports zero serious/critical violations for the chrome.
- [ ] React Compiler builds the package's demo without bail-out warnings for components in `src/components/` and `src/reader/` (or bail-outs are justified in code comments).

## Post-merge checks

None.

## Tests

`packages/ui/test/browser/*.browser.test.tsx` (three engines); `packages/ui/test/codes.test.ts` (Node); axe on `examples/ui-demo`.

## Docs to update

`packages/ui/README.md`; `CLAUDE.md` layout (`packages/ui`); tick "Create `packages/ui` (React 19, React Compiler) with the `Platform` interface and web implementation" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **React Compiler + Rolldown/Vite 8 integration** maturity; fall back to Babel plugin in Vite if needed and note it.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
