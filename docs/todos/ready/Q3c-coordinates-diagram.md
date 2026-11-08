# Q3c — A diagram for page space and user space, upright and rotated

| Field | Value |
| --- | --- |
| ID | `Q3c` |
| Status | `ready` |
| Plan | [Docs diagrams §2](../../plans/docs-diagrams.md#2-where-a-diagram-helps) |
| Round / milestone | Round Q3 (docs diagrams) |
| Depends on | [Q3a](./Q3a-diagram-kit-and-first-diagram.md) |
| ADRs | Follows [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) |
| Branch | `feat/q3c-coordinates-diagram` |
| PR title | `feat(docs): a diagram for page space and PDF user space` |
| Size | S |
| PR | — |

## Goal

Readers place text with `at: [72, 96]` and need to know where that lands. PDF's own coordinates start at the bottom-left, Lamina PDF's start at the top-left of the page you see, and a rotated page moves that corner. One figure with two panels shows both systems on the same page, upright and with `/Rotate 90`, so the reader stops guessing which corner is zero.

## Context

- [Docs diagrams plan](../../plans/docs-diagrams.md) §3: the kit, the style and the checks Q3a built; every diagram here uses them.
- `apps/docs/src/content/docs/concepts/pages-and-coordinates.mdx`: "The top-left corner of the page you see" and "PDF's own coordinates".
- [ADR 0027](../../decisions/0027-coordinates-units-and-page-indexing.md): page space, the crop box, rotation, and the worked example the second panel reuses.

## Scope

**In scope**

- Diagram 4: panel one shows the media box with the crop box inside it, page space (origin top-left, y down) and user space (origin bottom-left, y up), the point `[72, 96]` and a text baseline; panel two shows the same page with `/Rotate 90`, page space following the displayed top-left corner, and ADR 0027's worked rectangle.

**Out of scope** (and which todo covers it)

- Coordinates inside the viewer (zoom, device pixels): a later viewer page.

## Tasks

- [ ] `components/diagrams/PageSpace.astro` with the two panels side by side, stacked at phone width.
- [ ] Place panel one under "The top-left corner of the page you see" and repeat the figure's second panel under "PDF's own coordinates", or place the whole figure once where the page first compares the two (decide by reading the page).
- [ ] Use the kit's `PageSheet` so the page looks like a page in a viewer; axes in `text-2`, the point in `accent`.

## Acceptance criteria

- [ ] Each diagram is built from the Q3a kit, renders in both themes (screenshots in the PR) and passes the lint, twin and browser tests.
- [ ] Each page's `.md` twin holds the caption and text description, and no SVG or JSX.
- [ ] Each diagram is under 8 KB gzipped (reported in the PR).
- [ ] `cargo xtask check docs` and the docs build pass.
- [ ] The numbers in the figure match the page's prose and ADR 0027's worked example.

## Post-merge checks

- [ ] The diagrams show on `next.docs.laminapdf.com` in both themes.

## Tests

The Q3a lint, twin and browser tests cover each new diagram once it is under `apps/docs/src/components/diagrams/`; no new test files unless a diagram needs a layout the kit lacks.

## Docs to update

The pages named in the tasks, and the plan's §9 to-do.

## Risks and open questions

- Axis labels inside a small figure fall under 12 px at phone width; the stacked layout keeps them readable.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
