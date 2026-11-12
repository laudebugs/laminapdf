# Q4o — Usage pages for `Document` and `Builder` beside their reference

| Field | Value |
| --- | --- |
| ID | `Q4o` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | None |
| Branch | `docs/q4o-usage-pages` |
| PR title | `docs: usage pages for Document and Builder` |
| Size | M |
| PR | [#462](https://github.com/laudebugs/laminapdf/pull/462) |

## Goal

The TypeScript reference is generated from the types, and readers who reach it get signatures without the "how" and "why". react.dev splits each API into a reference and a usage section with common cases and pitfalls, and Hacker News commenters ask for examples beside a full reference. This todo writes usage pages for the two types readers use most, `Document` and `Builder`, and links them from the reference.

## Context

- [Docs research: recommendation 12](../../research/docs-site/developer-docs.md#recommendations-in-order) (an opinion in the report).
- starlight-typedoc's output under `reference/`, and the guides that already use the two types.

## Scope

**In scope**

- `reference/document-usage` and `reference/builder-usage`: common tasks as short tested examples, pitfalls, links to the guides.
- Links between each usage page and its generated reference page.

**Out of scope**

- Usage pages for the other types, until these two show they help.

## Tasks

- [x] List the tasks each type is used for, from the guides and examples.
- [x] Write the examples and the pages; link them from the reference.

## Acceptance criteria

- [x] Each usage page's examples are tested.
- [x] The generated reference links to its usage page.

## Post-merge checks

None.

## Tests

The examples' tests; the docs build.

## Docs to update

The reference section.

## Risks and open questions

- Usage pages can repeat the guides; keep each entry short and link to the guide for depth.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
