# U1c — Inverted-L shell: left rail, docked inspector, modes and focus mode

| Field | Value |
| --- | --- |
| ID | `U1c` |
| Status | `ready` |
| Plan | [UI/UX §6 U1. Shell foundations](../../plans/ui-ux.md#u1-shell-foundations) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C4e](../ongoing/C4e-reader-chrome.md), [U1a](../done/U1a-adr-0024-app-shell-ux.md), [U1b](./U1b-command-registry-and-palette.md) |
| ADRs | Implements 0024 (layout and modes); follows 0009, 0010 |
| Branch | `feat/u1c-inverted-l-shell` |
| PR title | `feat(ui): add the left rail, inspector, mode switcher and focus mode` |
| Size | M (300–600 lines incl. tests) |
| PR | — |

## Goal

Give `ReaderShell` the layout the research recommends — a collapsible left rail, an inspector docked on the right that appears only on selection, a mode switcher with one secondary toolbar per mode, and a focus mode — so the reader opens into a spare reading view and every later feature has a fixed place to land.

## Context

- **Design (Paper):** [F01](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards), [F02](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards), [R16 Focus mode](../../design/paper/reader-b.md#r16-focus-mode). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.1](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2) and ADR 0024.
- [Research: inverted-L layout](../../research/product/pdf-app-product-and-ux.md#desktop-and-web-an-inverted-l-layout-with-contextual-disclosure) — Linear's inverted L, Figma's reattached panels and Minimize UI, Microsoft's stability warning.
- [C4e](./C4e-reader-chrome.md) — `ReaderShell` and its slots; [U1b](./U1b-command-registry-and-palette.md) — the registry the rail, inspector and focus-mode toggles are commands in.

## Scope

**In scope**

- `LeftRail` (resizable, collapsible, keyboard-resizable with arrow keys, width and state kept locally), with tab slots that C6a fills (thumbnails, outline) and later E3 (bookmarks, attachments).
- `Inspector` (docked right, opens on selection events from the viewer or editor, closes on Escape or deselect, never overlays the canvas; the canvas reflows its width).
- `ModeSwitcher` and a `SecondaryToolbar` slot: modes are registered (`id`, label, icon, toolbar component, allowed commands); v0.1 registers Read only, so the switcher is hidden until a second mode exists.
- Focus mode: a command (from U1b) that hides the rail and the secondary toolbar; the status bar stays, because it carries the network indicator and the repaired-file notice (plan §5.1, §5.7); the inspector reappears temporarily on selection.
- Narrow-window behaviour: below a breakpoint the rail becomes an overlay drawer (phone layout itself is U3a).
- Landmarks: `banner`, `navigation` (rail), `main` (canvas), `complementary` (inspector), `contentinfo` (status).

**Out of scope** (and which todo covers it)

- Thumbnails and outline contents → [C6a](./C6a-thumbnails-outline-page-labels.md).
- Phone layout (bottom bar, sheets) → [U3a](./U3a-responsive-mobile-and-touch.md).
- Editor modes → E rounds (E1a defines Edit, E4f Comment, E8e Fill & Sign, E2h Organise).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Implement the components with CSS grid; no layout shift when the inspector opens (canvas width animates only when reduced motion is off).
- [ ] Mode registry and tests (register, switch, switching away clears mode-only selection).
- [ ] Focus mode command and shortcut; persists per session only.
- [ ] Wire rail and inspector toggles to U1b commands.
- [ ] Update `examples/ui-demo.tsx` with fake rail tabs and a fake selection to open the inspector.

## Acceptance criteria

- [ ] Opening a document shows only the top bar, the canvas and the status bar unless the user has opened the rail before (test).
- [ ] Selecting an object opens the inspector without covering any canvas pixel (visual test asserts the canvas box shrinks rather than being overlapped).
- [ ] Rail resize works by mouse and keyboard; focus mode toggles by shortcut and palette.
- [ ] axe: zero serious/critical violations; landmarks present and unique.

## Post-merge checks

None.

## Tests

`packages/ui/src/shell/*.test.tsx` (Vitest browser mode, three engines); mode-registry unit tests; a Playwright visual check on the demo page.

## Docs to update

`packages/ui/README.md` (layout regions, how a mode registers); tick the shell item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Inspector reflowing the canvas can re-trigger tile rendering: debounce width changes and reuse the zoom anchor from C3b.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
