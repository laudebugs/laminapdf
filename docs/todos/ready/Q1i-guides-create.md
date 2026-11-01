# Q1i — Plain-English guides: creating PDFs

| Field | Value |
| --- | --- |
| ID | `Q1i` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1h](../ongoing/Q1h-docs-story.md) |
| Blocks | [Q1l](./Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `docs/q1i-guides-create` |
| PR title | `docs(site): plain-English guides for creating PDFs` |
| Size | M |
| PR | — |

## Goal

This todo rewrites the "Create PDFs" guides to the [docs-writing guideline](../../guidelines/docs-writing.md): each opens with what you'll make and when you'd need it, explains or links every spec term, moves spec detail into "Details" asides, and uses one term per thing. It covers Text and fonts; Shapes, colour and images; Links, outlines and metadata; and the accessible-PDF and archival-PDF guides that [B8b](../done/B8b-conformance-profiles-and-verapdf.md) adds, if merged.

## Context

- [Quality plan §2.1](../../plans/quality.md#21-docs-prose-and-navigation): dense spec terms such as Type 0, `/W` and ToUnicode in `guides/fonts-and-text.mdx`.
- `apps/docs/src/content/docs/guides/{fonts-and-text,images-and-vectors,links-outlines-and-metadata}.mdx`.

## Scope

**In scope**

- Each page: an opening paragraph (≤ 50 words), the code introduced by a sentence, terms explained or linked, a "Details" aside for spec detail, and a link to the next page.
- Their place in the sidebar's "Create PDFs" group.

**Out of scope** (and which todo covers it)

- The read-and-edit guides ([Q1m](./Q1m-guides-read-and-edit.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Rewrite one page at a time with `/writing`; keep every fact, moving spec detail into Details.
- [ ] Run `docs-prose` on each page and fix what it reports.
- [ ] Run `/writing-review` on the changed pages after opening the PR; fix the findings.

## Acceptance criteria

- [ ] `docs-prose` reports nothing on these pages.
- [ ] No fact from the old pages is lost (the PR description lists where each moved).

## Post-merge checks

None.

## Tests

`docs-prose`, the docs build.

## Docs to update

Plan §9.

## Risks and open questions

Rewrites can drop facts: move spec detail into Details rather than deleting it.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
