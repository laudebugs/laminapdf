# W1g — A Paper select for the sites, and the docs site's theme switch on it

| Field | Value |
| --- | --- |
| ID | `W1g` |
| Status | `done` |
| Plan | [Website §4 The website](../../plans/website.md#4-the-website) |
| Round / milestone | Round W (website) |
| Depends on | [W1d](./W1d-one-site-header.md) |
| ADRs | — |
| Branch | `feat/docs-theme-select` |
| PR title | `feat(docs): a Paper select for the sites, and the theme switch on it` |
| Size | S (100–300 lines) |
| PR | [#361](https://github.com/laudebugs/laminapdf/pull/361) |

## Goal

The docs site's theme switch is Starlight's own select, drawn in Starlight's look with its icons. This todo adds design language [§7.19](../../design/paper/design-language.md#719-form-controls-consistent-with-the-page-input-and-note-field)'s Select to `@laminapdf/site-kit`, one Paper select for both Astro sites, and puts the docs site's theme switch on it. The version switcher that the releases plan leaves for 1.0 ([releases §4](../../plans/releases.md#4-docs-that-match-a-version)) can use the same select.

## Context

- Design language §7.19: the Select is a native `<select>` with `appearance: none`, 32 px high, padding `0 30px 0 10px`, and the 16 px chevron in `text-3` 8 px from the right edge. `@laminapdf/ui`'s React `Select` draws the same thing; the sites have no React, so they need an Astro one.
- Starlight's `ThemeSelect` (`@astrojs/starlight/components/ThemeSelect.astro`): a `<starlight-theme-select>` element around a `<select>` with the values `dark`, `light` and `auto`, the preference in `localStorage` under `starlight-theme`. Its theme provider, an inline script in the head, sets the theme before the first paint and calls `StarlightThemeProvider.updatePickers()`, which sets every `starlight-theme-select select`'s value. A `ThemeSelect` override keeps that element and those values, so the provider works unchanged.
- [W1d](../done/W1d-one-site-header.md): the shared header and its browser tests, which render Astro components with the container API.
- The website's build refuses inline styles (`siteChecks`), so the select sizes to its options rather than taking a width in a `style` attribute as Starlight's does.

## Scope

**In scope**

- `@laminapdf/site-kit`: `Select.astro` (a label, visible or hidden, the options, the Paper chevron from `packages/icons/svg/`) and `select.css`, drawn from the Paper tokens.
- The docs site: a `ThemeSelect` override on the new select, with Starlight's labels and behaviour (the stored choice, Auto following the system, every switch on the page kept in step), in the header and the phone menu.
- Browser tests of the select: its name, keyboard and pointer use, focus ring, axe in light and dark.

**Out of scope**

- The version switcher: it waits for 1.0 ([releases §4](../../plans/releases.md#4-docs-that-match-a-version)).
- A custom list popup (design language §7.26's listbox): the select opens the platform's own list, as §7.19 says.

## Tasks

- [x] Claim: the first commit adds this file to `ongoing/`.
- [x] `packages/site-kit/src/Select.astro` and `select.css`, exported from the package; the chevron from `@laminapdf/icons/svg/`, which the icons package now exports.
- [x] `apps/docs/src/components/ThemeSelect.astro`, set as Starlight's `ThemeSelect`.
- [x] Browser tests in `packages/site-kit/test/browser/`, and an e2e test of the switch in `apps/docs/e2e/`.
- [x] Try it in the built docs site in a browser: light, dark and auto, with the mouse and the keyboard, at desktop and phone widths.

## Acceptance criteria

- [x] The docs site's theme switch is the Paper select in the header and in the phone menu, and switching the theme works as before, the choice kept across pages and reloads.
- [x] The select's browser tests pass in Chromium, Firefox and WebKit, with no axe violation in light or dark.

## Post-merge checks

- [ ] `next.docs.laminapdf.com` shows the Paper select once main deploys.

## Tests

`packages/site-kit/test/browser/select.browser.test.ts`, on the select rendered by Astro's container in the global setup: its name, the label above it, Tab and a typed letter, a press on the chevron's side, the focus ring, the Paper colours, axe. `apps/docs/e2e/theme.e2e.test.ts`, on the built site under `wrangler dev` over HTTPS in each browser: Auto at first, a choice kept after a reload, Auto clearing it, the phone menu's switch in step with the header's, no console error.

Tried in the built docs site (`wrangler dev` with its headers) in Chromium, Firefox and WebKit, as a reader:

- Light chosen with the mouse, and kept after a reload.
- Auto following the system and clearing the stored choice.
- The header's and the phone menu's switches in step.
- A letter typed on the focused switch choosing its theme, and the focus ring from the keyboard.
- 44 px with 16 px text on a touch screen.
- No console error in Chromium or Firefox.

WebKit over plain HTTP couldn't load the styles (the CSP's `upgrade-insecure-requests`), so WebKit's look was checked by the browser tests and its behaviour by the e2e test over HTTPS.

## Docs to update

`packages/site-kit/README.md`, `CLAUDE.md`'s site-kit and docs lines, the website plan's todo list.

## Risks and open questions

- A Starlight update could change the theme provider's contract (the element name, the values, the storage key). The override copies Starlight's switching logic, so an update needs a look at it.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, added this file to `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
