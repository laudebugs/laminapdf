# U3a — Responsive mobile and touch layout for the web reader

| Field | Value |
| --- | --- |
| ID | `U3a` |
| Status | `ongoing` |
| Plan | [UI/UX §6 U3. Mobile and touch](../../plans/ui-ux.md#u3-mobile-and-touch) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C4b](../done/C4b-web-app-shell.md), [U1b](../done/U1b-command-registry-and-palette.md), [U1c](../done/U1c-inverted-l-shell-and-modes.md) |
| ADRs | Implements 0024 (phone layout); follows 0009, 0010 |
| Branch | `feat/u3a-mobile-and-touch` |
| PR title | `feat(ui): add the phone and tablet layout with viewer-owned gestures` |
| Size | L (500–900 lines incl. tests) |
| PR | — |

## Goal

Make the web reader good on phones and tablets, not just "works": a thumb-reachable bottom bar, bottom sheets for navigation, safe-area handling, viewer-owned pan and pinch that don't fight browser zoom, and bitmap budgets that keep iOS Safari from killing the tab on large documents.

## Context

- **Design (Paper):** [F03](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards), [F04](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards), [R17 Tablet reading](../../design/paper/reader-b.md#r17-tablet-reading), [R18 Phone: outline, find, more](../../design/paper/reader-b.md#r18-phone-outline-find-more), [C08 Phone annotate](../../design/paper/comment-review.md#c08-phone-annotate). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.4](../../plans/ui-ux.md#54-mobile-and-touch-r17) and ADR 0024.
- [Research: mobile ergonomics](../../research/product/pdf-app-product-and-ux.md#mobile-ergonomics-bottom-bar-sheets-with-detents-and-the-viewer-owns-gestures) — Hoober's grip data, Apple sheets and detents, `viewport-fit=cover`, WCAG 2.5.7/2.5.8.
- [Engines notes](../../research/product/notes/engines-sdks.md) — iOS Safari canvas caps (≈ 16.7 MP per canvas, 224–384 MB total) and pdf.js's 5 MP mobile cap.
- [C3b](../done/C3b-viewer-virtualiser-and-zoom.md) — tiles, zoom anchoring; [C4b](../done/C4b-web-app-shell.md) — the app shell.
- [web-app plan open question 5](../../plans/web-app.md#8-open-questions) — tablet UX in v0.1; this todo answers it for reading.

## Scope

**In scope**

- Layout below the phone breakpoint: auto-hiding top bar (title, back, overflow) and a bottom bar (Search, Pages, Reader when U4b exists, More); tablet: the desktop layout with touch targets.
- `BottomSheet` with medium and large detents, drag handle plus buttons (SC 2.5.7), focus trap and Escape; hosts thumbnails, outline, properties and settings.
- `viewport-fit=cover`, `env(safe-area-inset-*)` padding, `100dvh` sizing; no horizontal page scroll at 320 px width.
- Gestures in `@laminapdf/viewer`: `touch-action: none` on the canvas, pan with momentum, pinch around the focal point (CSS transform during the gesture, tiles after), double-tap to fit width / zoom in; long-press passes through for text selection.
- Bitmap budgets on coarse-pointer devices: tile ≤ 5 MP, total budget from `navigator.deviceMemory` when available (else a conservative default), LRU eviction.
- Touch targets ≥ 44 px.

**Out of scope** (and which todo covers it)

- Native mobile apps → [U3b](../ready/U3b-mobile-shell-spike.md) and ADR 0025.
- Stylus annotation → E4 rounds (and open question 2 of the plan).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add Playwright device profiles (iPhone 15, Pixel 8, iPad) to the e2e suite with touch emulation.
- [ ] Implement the bottom bar and sheets in `@laminapdf/ui`; reuse U1b commands.
- [ ] Implement the gesture recogniser in `@laminapdf/viewer` with unit tests for pinch focal-point maths and double-tap timing.
- [ ] Implement budgets and eviction; add a memory probe to the perf harness for mobile profiles.
- [ ] Manual pass on a real iPhone (Safari) and Android phone (Chrome): record results in the PR.

## Acceptance criteria

- [ ] Open, scroll, zoom, find, outline jump and copy work in the three Playwright device profiles.
- [ ] Pinch zoom keeps the point under the fingers fixed within 2 px (test with synthetic touches).
- [ ] A 1,000-page fixture scrolls end to end on a real iPhone without a tab reload; recorded in the PR.
- [ ] No horizontal scroll at 320 px; all touch targets ≥ 44 px (automated check); axe zero serious/critical violations in mobile profiles.
- [ ] Browser zoom of the chrome still works for accessibility (the canvas handles its own zoom).

## Post-merge checks

None.

## Tests

Playwright mobile profiles; viewer gesture unit tests; perf-harness mobile memory probe.

## Docs to update

`packages/ui/README.md` (breakpoints, sheets); tick the mobile items in [UI/UX §9](../../plans/ui-ux.md#9-to-dos); answer open question 5 in [web-app §8](../../plans/web-app.md#8-open-questions).

## Risks and open questions

- Custom gestures can break VoiceOver/TalkBack swipe navigation: test with both screen readers and keep the canvas out of the swipe order where the text layer provides content.
- `navigator.deviceMemory` is unavailable in Safari: the default budget must be safe for the smallest supported iPhone.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
