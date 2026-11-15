# U7e — The docs site and the website: states, cursors and motion

| Field | Value |
| --- | --- |
| ID | `U7e` |
| Status | `ongoing` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7a](../done/U7a-motion-and-state-tokens.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (tokens in layers), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) §4 (headers, CSP) |
| Branch | `feat/u7e-sites-states-and-motion` |
| PR title | `feat(sites): pressed, disabled and reduced-motion states on both sites` |
| Size | M (200–400) |
| PR | — |

## Goal

Apply the plan's §3 and §4 to the docs site, the website and the site kit:

- a pressed state everywhere, hover that doesn't stick on touch, and disabled controls that look disabled (Starlight's own search button among them)
- hover on the code tabs, Starlight's tabs, the select and the runner's buttons, with short tokenised fades
- links: the current page by `aria-current` and an underline, visited links unstyled (plan §3, 9)
- a reduced-motion layer that also covers Starlight's sidebar caret and Expressive Code's Copy
- the runner's busy state while it runs, one focus ring across both sites, and Copy as Markdown's "Copied" clearing

## Context

- The sites [research](../../research/product/motion-and-states/README.md) ([plan §2](../../plans/motion-and-states.md#2-what-the-apps-do-today-2026-11-14)): no `:active` rule, `--paper-pressed` and `--paper-disabled` unused, hover with no touch guard (`header.css:117-120`, `site.css:73,140`, `CopyMarkdown.astro:94`), Starlight's `Search.astro` (imported unchanged by `Header.astro`), `Example.astro:572-577,631-671`, `site.css:36-40`
- ADR 0011 §4's headers allow stylesheet transitions and `@starting-style`, and no inline styles

## Scope

**In scope**

- `apps/docs/src/styles/paper.css` and components, `apps/site` styles, `packages/site-kit` (header, select)
- States in the plan's order, the cursor tokens, the touch guard, links
- A reduced-motion layer for both sites
- The runner: `aria-disabled` and a busy label while running instead of `disabled`, so focus stays; no layout shift when Download appears
- One focus ring, without the website's `border-radius` on focus
- A cursor check over built docs and website pages (plan §6)

**Out of scope**

- Page transitions between docs pages (Starlight doesn't support Astro's router)

## Tasks

- [ ] States and cursors
- [ ] Links
- [ ] Touch guard
- [ ] Reduced motion
- [ ] The runner's busy state
- [ ] Focus ring
- [ ] The cursor check

## Acceptance criteria

- [ ] The site kit's browser tests (Chromium, Firefox, WebKit) check the header's and select's cursors and pressed states
- [ ] The cursor check over built docs and website pages finds no clickable element without the hand
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

- Starlight's components are styled from Paper's CSS, not forked; a rule that reaches into Starlight's markup must survive a Starlight upgrade (pin and note it)

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
