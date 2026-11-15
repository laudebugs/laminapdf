# U7k — The sites' pass findings: overlays that enter and leave, and steady tabs and buttons

| Field | Value |
| --- | --- |
| ID | `U7k` |
| Status | `ready` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7h](../done/U7h-delight-pass.md) |
| ADRs | Follows [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) §4 (no inline styles on the sites) |
| Branch | `fix/u7k-sites-pass-fixes` |
| PR title | `fix(site-kit): the sites' overlays enter and leave` |
| Size | M (200–400) |
| PR | — |

## Goal

Fix what U7h's by-hand pass over the docs site and the website found: no overlay on either site animates in or out, Copy as Markdown slides out from under the pointer, and a few links and options don't follow the state model.

## Context

- [Plan §3, 3 and 6, §4.3](../../plans/motion-and-states.md#43-rules-for-motion): the sites use `@starting-style` with `transition-behavior: allow-discrete`
- [U7e](../done/U7e-sites-states-and-motion.md): the states and motion the sites already have

## Scope

**In scope**

- **Overlays:** the search dialog and its `::backdrop`, the theme select's list, Starlight's phone menu (`#starlight__sidebar`), the phone "On this page" (`mobile-starlight-toc .dropdown`), the contents rail's panel (`.right-sidebar-panel`) and the website's phone menu all appear and vanish in one frame; none of the sites' CSS has a `@starting-style` (§3, 3 and 6). Files: `packages/site-kit/src/menu-select.css`, `header.css`, `apps/docs/src/styles/paper.css`
- **Copy as Markdown:** "Copied" appears inline, growing the group from 156 to 198 px, so the button slides about 41 px left, out from under the pointer, and back 2.5 s later (`apps/docs/src/components/CopyMarkdown.astro`; §3, 7)
- **Links:** prose, footer and version links snap their colour (`transition: all 0s`; §3, 2): `packages/site-kit/src/base.css`'s `:any-link`, `apps/site/src/styles/site.css` or `paper.css`; the unreleased banner's "released docs" link has no hover or pressed state
- **The theme select:** a hovered option takes the same tint and edge as the `data-active` one, so two look current (`menu-select.css`, or move `data-active` with the pointer in `menu-select.ts`; §4.2); its focus ring sits 1 px out where every other control's is 2 px
- Expressive Code's "Copied!" bubble is its default green, not a Paper token (`apps/docs/ec.config.mjs`); decide whether it changes

**Out of scope**

- The reader → [U7j](./U7j-reader-pass-fixes.md)

## Tasks

- [ ] Enter and exit for each overlay above, with reduced motion fading only
- [ ] Copy as Markdown's status out of the button's way
- [ ] The links, the theme select, the code-copy bubble
- [ ] Check Firefox: U7h's pass couldn't launch Playwright's Firefox outside Vitest on the Mac

## Acceptance criteria

- [ ] Every overlay listed fades in and out (enter-small/exit-small or enter-panel/exit-panel) in Chromium, Firefox and WebKit
- [ ] Pressing Copy as Markdown leaves the button where it was

## Post-merge checks

None.

## Tests

- The docs' and the website's e2e suites, and `packages/site-kit`'s browser tests, beside the U7e checks

## Docs to update

- Plan §7: this row

## Risks and open questions

- Starlight's dialog and menus are its own markup: style them from `paper.css`, not by forking components (checked against Starlight 0.42.8)

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built sites, as a person uses them (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
