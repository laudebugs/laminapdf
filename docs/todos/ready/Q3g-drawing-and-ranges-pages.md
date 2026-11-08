# Q3g — Two new pages with their diagrams: drawing pages, and opening large files by ranges

| Field | Value |
| --- | --- |
| ID | `Q3g` |
| Status | `ready` |
| Plan | [Docs diagrams §2](../../plans/docs-diagrams.md#2-where-a-diagram-helps) |
| Round / milestone | Round Q3 (docs diagrams) |
| Depends on | [Q3a](./Q3a-diagram-kit-and-first-diagram.md) |
| ADRs | Follows [0004](../../decisions/0004-render-backend.md) |
| Branch | `feat/q3g-drawing-and-ranges` |
| PR title | `feat(docs): pages and diagrams for drawing pages and opening large files by ranges` |
| Size | M |
| PR | — |

## Goal

Two features have no page of their own: how a page is drawn (and why its text never comes from the renderer), and how `Document.openRange` opens a large remote file without downloading it. Each gets a short page built around its diagram, so readers find both in the sidebar instead of in passing mentions.

## Context

- [Docs diagrams plan](../../plans/docs-diagrams.md) §3: the kit, the style and the checks Q3a built; every diagram here uses them.
- `apps/docs/src/content/docs/concepts/dependencies.mdx`: "Drawing pages is the one job it hands to PDFium" and "PDFium only draws, and only in a Web Worker", which the drawing page expands.
- [ADR 0004](../../decisions/0004-render-backend.md): render requests, tile plans, backends and the rule that text comes from Lamina PDF's own model.
- `packages/sdk/src` (`Document.openRange`, `httpRangeSource`, `ensure`) and `crates/laminapdf-source`: the range-loading loop.
- `apps/docs/plugins/sidebar.mjs`: every new page goes into the explicit sidebar.

## Scope

**In scope**

- Diagram 11, "Drawing a page", on a new `concepts/drawing-pages.mdx`: a render request (page, scale, a tile's clip, rotation), the tile plan, the backend (PDFium or hayro), RGBA pixels; beside it, text, links and selection from Lamina PDF's own model, both reading the same bytes.
- Diagram 12, "Opening a large file by ranges", on a new `guides/open-large-files.mdx`: a call, `data-pending` with the ranges it needs, the ranges fetched (`httpRangeSource`), supplied, the call retried by `doc.ensure`, the result.

**Out of scope** (and which todo covers it)

- The viewer's components (main thread, Web Worker, tiles, cache): a viewer page once `@laminapdf/viewer`'s API is stable (plan §2).

## Tasks

- [ ] Write `concepts/drawing-pages.mdx` and `guides/open-large-files.mdx`: title, a description of 160 characters or fewer, an opening paragraph of 50 words or fewer, then the diagram and the sections that explain it; examples through `<Snippet>` from tested example files, as every page does.
- [ ] `components/diagrams/DrawingPages.astro` and `RangeLoading.astro`.
- [ ] Add both pages to `apps/docs/plugins/sidebar.mjs` (plan §8 asks where: decide when starting) and link them from `concepts/dependencies.mdx` and `guides/untrusted-files.mdx`.

## Acceptance criteria

- [ ] Each diagram is built from the Q3a kit, renders in both themes (screenshots in the PR) and passes the lint, twin and browser tests.
- [ ] Each page's `.md` twin holds the caption and text description, and no SVG or JSX.
- [ ] Each diagram is under 8 KB gzipped (reported in the PR).
- [ ] `cargo xtask check docs` and the docs build pass.
- [ ] Both pages pass `cargo xtask check docs-prose` and `docs-snippets`, and appear in the sidebar and in `llms.txt`.
- [ ] Every example on them runs in the example suite.

## Post-merge checks

- [ ] The diagrams show on `next.docs.laminapdf.com` in both themes.

## Tests

The Q3a lint, twin and browser tests cover each new diagram once it is under `apps/docs/src/components/diagrams/`; no new test files unless a diagram needs a layout the kit lacks.

## Docs to update

The pages named in the tasks, and the plan's §9 to-do.

## Risks and open questions

- The range-loading API may change before the page ships; check `packages/sdk/src` on main when starting.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
