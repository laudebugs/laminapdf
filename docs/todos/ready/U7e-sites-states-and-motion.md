# U7e — The docs site and the website: states, cursors and motion

| Field | Value |
| --- | --- |
| ID | `U7e` |
| Status | `ready` |
| Plan | [Motion and interaction states](../../plans/motion-and-states.md) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7a](./U7a-motion-and-state-tokens.md) |
| ADRs | Follows 0024 (tokens in layers), 0010 (capability flags) |
| Branch | `feat/u7e-sites-states-and-motion` |
| PR title | `feat(sites): pressed, disabled and reduced-motion states on both sites` |
| Size | M (200–600) |
| PR | — |

## Goal

Apply plan §4 to the docs site, the website and the site kit: a pressed state everywhere, hover that doesn't stick on touch, disabled controls that look disabled (Starlight's search button among them), hover on the code tabs, Starlight's tabs, the select and the runner's buttons, short tokenised fades, a reduced-motion layer that also covers Starlight's sidebar caret and Expressive Code's Copy, the runner's busy state while it runs, one focus ring across both sites, and Copy as Markdown's "Copied" clearing.

## Context

- The sites research summarised in [plan §2](../../plans/motion-and-states.md#2-what-the-apps-do-today-2026-11-14): no `:active` rule, `--paper-pressed` and `--paper-disabled` unused, hover with no touch guard (`header.css:117-120`, `site.css:73,140`, `CopyMarkdown.astro:94`), `Search.astro:176`, `Example.astro:572-577,631-671`, `site.css:36-40`
- ADR 0011 §4: the CSP allows stylesheet transitions and `@starting-style`, no inline styles

## Scope

**In scope**

- `apps/docs/src/styles/paper.css` and components, `apps/site` styles, `packages/site-kit` (header, select)
- States in the plan's order, the cursor tokens, the touch guard
- A reduced-motion layer for both sites
- The runner: `aria-disabled` and a busy label while running instead of `disabled`, so focus stays; no layout shift when Download appears
- One focus ring, without the website's `border-radius` on focus

**Out of scope**

- Page transitions between docs pages (Starlight doesn't support Astro's router)

## Tasks

- [ ] States and cursors
- [ ] Touch guard
- [ ] Reduced motion
- [ ] The runner's busy state
- [ ] Focus ring

## Acceptance criteria

- [ ] The site kit's browser tests (Chromium, Firefox, WebKit) check the header's and select's cursors and pressed states
- [ ] A cursor check over built docs and website pages finds no clickable element without the hand
- [ ] With reduced motion, Starlight's sidebar caret doesn't rotate

## Post-merge checks

None.

## Tests

- The site kit's browser tests
- The docs' e2e (the runner)
- Both sites' builds and their page checks

## Docs to update

- Plan §7: point this todo at `done/`

## Risks and open questions

- Starlight's own components are overridden, not forked; an override must survive a Starlight upgrade (pin and note it)

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
