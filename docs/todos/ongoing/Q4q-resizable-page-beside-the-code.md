# Q4q — A divider to resize the page beside the code

| Field | Value |
| --- | --- |
| ID | `Q4q` |
| Status | `ongoing` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4d](../done/Q4d-result-beside-the-code.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the CSP: no inline styles in markup) |
| Branch | `claude/dazzling-sagan-s9woz0` |
| PR title | `feat(docs): a divider to resize an example's page beside its code` |
| Size | S |
| PR | [laudebugs/laminapdf#537](https://github.com/laudebugs/laminapdf/pull/537) |

## Goal

Since Q4d, an example's first page sits beside its code on wide screens, in a column 15rem wide. That width keeps the code readable, but the page is too small to read, and a reader who edits and runs an example can't see what changed. The owner asked for a divider like VS Code's between its panes: drag it to give the page or the code more room.

## Context

- `apps/docs/src/components/Example.astro` (the grid, the page's column, "Edit and run").
- `apps/docs/runner/editor.ts` (`PAGE_PIXELS`: the width a run's pages are drawn at).
- `apps/docs/e2e/example.e2e.test.ts` (the layout at 1440 and 390 px, with and without JavaScript).
- The [docs examples plan](../../plans/docs-examples.md).

## Scope

**In scope**

- On screens where the page sits beside the code, a divider between them: drag it, or focus it and use the arrow keys (Shift for bigger steps), Home and End; double-click or Enter puts back the default.
- One width for every example, remembered in the browser (`localStorage`, a per-reader convenience).
- Bounds: the code keeps at least 20rem and the page at least 12rem.
- A run's pages drawn at the column's width, so a wider page stays sharp.

**Out of scope**

- Narrow screens, where the page is below the code.
- Pages with JavaScript off: they keep Q4d's layout, with no divider.
- Resizing the code's height.

## Tasks

- [ ] The divider in `Example.astro`: a custom element with `role="separator"`, pointer and keyboard handling, the width as a custom property set through the CSSOM (the CSP blocks `style` attributes in markup).
- [ ] The run's pages drawn at the column's width in `editor.ts`.
- [ ] e2e tests: drag, keys, reset, the width kept after a reload, no divider with JavaScript off, axe with the divider.
- [ ] The docs examples plan's notes.

## Acceptance criteria

- [ ] At 1440 px, dragging the divider left widens the page and narrows the code, and the width is kept after a reload.
- [ ] The divider works from the keyboard and has a name and value for screen readers.
- [ ] No horizontal scroll at any width the divider allows.

## Post-merge checks

None.

## Tests

The docs e2e suite (`example.e2e.test.ts`) in Chromium, Firefox and WebKit, run by `cargo xtask pdf-runtimes <browser>`.

## Docs to update

The docs examples plan (§2's notes, §4's table).

## Risks and open questions

- The page pictures are 1190 px wide, sharp up to about 37rem on a screen with two pixels per point; a page widened past that is stretched and looks soft.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
