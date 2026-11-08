# Q3e — Diagrams for extraction: from page to Markdown, and reading order

| Field | Value |
| --- | --- |
| ID | `Q3e` |
| Status | `ready` |
| Plan | [Docs diagrams §2](../../plans/docs-diagrams.md#2-where-a-diagram-helps) |
| Round / milestone | Round Q3 (docs diagrams) |
| Depends on | [Q3a](../done/Q3a-diagram-kit-and-first-diagram.md), [Q3c](./Q3c-coordinates-diagram.md) |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) |
| Branch | `feat/q3e-extraction-diagrams` |
| PR title | `feat(docs): diagrams for how a page becomes Markdown, and its reading order` |
| Size | S |
| PR | — |

## Goal

Extraction is the library's most-used reading feature, and its recipe explains six stages in one dense section. A pipeline diagram shows each stage and what it drops or groups, and a page sketch shows the order a two-column page comes out in, so a reader who gets text in a surprising order knows which stage decided it.

## Context

- [Docs diagrams plan](../../plans/docs-diagrams.md) §3: the kit, the style and the checks Q3a built; every diagram here uses them.
- `apps/docs/src/content/docs/recipes/extract-markdown.mdx`: "How it works".
- [ADR 0030](../../decisions/0030-extraction-output-and-model-plugins.md): the stages, the dropped-text flags and the reading-order rules.

## Scope

**In scope**

- Diagram 6, "From page to Markdown": text runs, hidden text set aside (invisible, off-page, tiny, covered: into `dropped`), lines, blocks (paragraphs, headings, tables), reading order (the structure tree when the file is tagged, else column cuts), then the three outputs: Markdown, `laminapdf.extract/1` JSON and chunks.
- Diagram 7, "Reading order on a two-column page": a page sketch with numbered blocks: a title across both columns first, then down the left column, then the right.

**Out of scope** (and which todo covers it)

- Model plug-ins (layout and OCR models): a later diagram if their guide grows.
- The page outline (`PageSheet`): Q3c builds it, so this todo depends on Q3c.

## Tasks

- [ ] `components/diagrams/ExtractPipeline.astro` (a horizontal pipeline, vertical at phone width) and `components/diagrams/ReadingOrder.astro` (a `PageSheet` with numbered blocks).
- [ ] Place both under "How it works", the pipeline first.

## Acceptance criteria

- [ ] Each diagram is built from the Q3a kit, renders in both themes (screenshots in the PR) and passes the lint, twin and browser tests.
- [ ] Each page's `.md` twin holds the caption and text description, and no SVG or JSX.
- [ ] Each diagram is under 8 KB gzipped (reported in the PR).
- [ ] `cargo xtask check docs` and the docs build pass.
- [ ] The stage names and the drop reasons match the recipe's prose and the JSON's field names.

## Post-merge checks

- [ ] The diagrams show on `next.docs.laminapdf.com` in both themes.

## Tests

The Q3a lint, twin and browser tests cover each new diagram once it is under `apps/docs/src/components/diagrams/`; no new test files unless a diagram needs a layout the kit lacks.

## Docs to update

The pages named in the tasks, and the plan's §9 to-do.

## Risks and open questions

- Six stages make a long pipeline; group the hidden-text step as one box with its four reasons listed under it.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
