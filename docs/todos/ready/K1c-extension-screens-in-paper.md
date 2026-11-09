# K1c — The extension's screens in Paper

| Field | Value |
| --- | --- |
| ID | `K1c` |
| Status | `ready` |
| Plan | [Browser extension §5 K1](../../plans/browser-extension.md#k1-prove-it-decide-it-draw-it) |
| Round / milestone | Round K (browser extension) — M2b Browser extension |
| Depends on | [K1b](./K1b-adr-0033-browser-extension.md) |
| ADRs | Follows 0033 (the permission model the screens explain), [0024](../../decisions/0024-app-shell-ux.md) (tokens, commands) |
| Branch | `design/k1c-extension-screens` |
| PR title | `design(extension): draw the popup, onboarding, settings and fallback screens` |
| Size | S (< 200 changed lines: a canvas, its area document and the screen inventory rows) |
| PR | — |

## Goal

Draw the extension's own screens before anyone builds them, as the [Paper design plan](../../plans/paper-design.md) requires for every screen. The reader inside the extension reuses the reader's screens. The new screens are the toolbar popup, the onboarding page that offers automatic opening and asks for access to all sites, the settings page, and the notice that offers the browser's own viewer when a fetch fails.

## Context

- [Paper design plan](../../plans/paper-design.md) and [design language](../../design/paper/design-language.md): how screens are drawn, named and documented.
- [Screen inventory](../../design/paper/screens.md): the list `cargo xtask check docs` holds to the design files.
- ADR 0033: what each permission is for, in words the onboarding page must use.
- [Competitors note](../../research/browser-extension/notes/competitors-and-demand.md): users dislike extensions that take PDFs over without asking.

## Scope

**In scope**

- A new canvas and area document, `docs/design/paper/browser-extension.md`, with each screen's purpose, entry points, controls with command ids, states and accessibility.
- Screens:
  - the toolbar popup: open a file, open this page's PDF, automatic opening on or off;
  - onboarding;
  - settings;
  - the fallback notice;
  - the permission-revoked state.
- Light, dark and the browsers' popup sizes.

**Out of scope** (and which todo covers it)

- Building the screens: [K2a](./K2a-extension-app-and-manual-open.md) and [K2b](./K2b-automatic-pdf-takeover-chromium.md).
- Store screenshots and listing art: [K2c](./K2c-chromium-release.md).

## Tasks

- [ ] Draw the screens in the design tool used for the other canvases, from the Paper tokens.
- [ ] Write the area document and add the screens to `screens.md`.
- [ ] Run `cargo xtask check docs`.

## Acceptance criteria

- [ ] Every screen in scope is drawn and described, and `cargo xtask check docs` passes.
- [ ] The onboarding text names each permission it asks for and why, in plain English.

## Post-merge checks

None.

## Tests

None; the design screens check in `cargo xtask check docs`.

## Docs to update

- `docs/design/paper/screens.md`, the new area document, and the plan (tick K1c).

## Risks and open questions

- Browsers size popups differently: Chrome caps them at 800 × 600, and Safari on iOS shows them as a sheet. Draw the smallest case first.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
