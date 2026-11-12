# Q4d — The result beside the code, and "Your first PDF" runnable

| Field | Value |
| --- | --- |
| ID | `Q4d` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the CSP the runner works under) |
| Branch | `docs/q4d-result-beside-code` |
| PR title | `docs: show each example's first page beside its code` |
| Size | S–M |
| PR | [laudebugs/lamina#469](https://github.com/laudebugs/lamina/pull/469) |

## Goal

Readers start from example code and want to see what it makes. The docs have the pictures (38 page images from the example goldens), but each sits behind a Result tab that a reader has to find, and the first page a newcomer reads, "Your first PDF", has no "Edit and run". No PDF library the research looked at shows code and its rendered page side by side. This todo shows the first page beside the code by default and makes the first tutorial runnable.

## Context

- [Docs research: recommendation 3](../../research/docs-site/developer-docs.md#recommendations-in-order).
- `apps/docs/src/components/Example.astro` (the tabs), `public/examples/preview/manifest.json`, `runner/examples.ts` (`RUNNABLE`).
- The [live examples research](../../research/docs-site/live-examples.md) and the [docs examples plan](../../plans/docs-examples.md).

## Scope

**In scope**

- On wide screens, the first page picture beside the code, without script; on narrow screens, below it.
- "Edit and run" reachable from the picture, not only inside the Result tab.
- "Your first PDF"'s example (`packages/sdk/examples/quick-start/node.ts`, or a browser twin of it) added to `RUNNABLE`.

**Out of scope**

- New runnable examples beyond the first tutorial.
- Rust in the browser.

## Tasks

- [x] Design the layout with the Paper tokens; check it at the docs' content width and at 390 px.
- [x] Change `Example.astro`; keep the Result tab for every page of a multi-page result.
- [x] Make the first tutorial runnable, with the inputs it needs.
- [x] Update the e2e tests for the new layout.

## Acceptance criteria

- [x] A page with an example shows its first page beside or below the code with no click.
- [x] "Your first PDF" has "Edit and run".
- [x] No new script runs before a reader presses "Edit and run".

## Post-merge checks

None.

## Tests

The docs e2e tests (layout at two widths, the runner on the first tutorial); axe on a page with an example.

## Docs to update

The docs examples plan's notes; the first tutorial.

## Risks and open questions

- Side-by-side squeezes the code on mid-width screens; pick the breakpoint by measuring, and keep the code readable first.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
