# Q1m — Plain-English guides: reading and editing PDFs

| Field | Value |
| --- | --- |
| ID | `Q1m` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1h](../done/Q1h-docs-story.md) |
| Blocks | [Q1l](./Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `docs/q1m-guides-read-and-edit` |
| PR title | `docs(site): plain-English guides for reading and editing PDFs` |
| Size | M |
| PR | — |

## Goal

This todo rewrites the "Read and edit" and "Production" guides to the [docs-writing guideline](../../guidelines/docs-writing.md). It also restructures them: "Open, edit and save" splits into "Edit a PDF" and "Save a PDF", "Limits" becomes "Open files you don't trust", "Encrypted files" becomes "Open password-protected files", and the password-order sentence is corrected.

## Context

- [Quality plan §2.1](../../plans/quality.md#21-docs-prose-and-navigation): the overlay passage and the password-order error.
- `apps/docs/src/content/docs/guides/{open-edit-and-save,encrypted-files,limits,off-the-main-thread,errors-and-warnings}.mdx`; `crates/lamina-syntax/src/crypt/mod.rs` (owner password first for R5/R6).

## Scope

**In scope**

- Split `open-edit-and-save.mdx`; rename two pages with redirects; rewrite all five; fix the password sentence ("tried as both").
- Their places in the sidebar.

**Out of scope** (and which todo covers it)

- The create guides ([Q1i](../ongoing/Q1i-guides-create.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Split and rename the pages; add redirects; update the sidebar.
- [ ] Rewrite each page with `/writing`, keeping every fact.
- [ ] Correct the password sentence against `crypt/mod.rs`.
- [ ] Run `docs-prose`; build the site and follow the old URLs.
- [ ] Run `/writing-review` after opening the PR; fix the findings.

## Acceptance criteria

- [ ] `docs-prose` reports nothing on these pages; the old URLs redirect.
- [ ] The password sentence matches the code.

## Post-merge checks

None.

## Tests

`docs-prose`, the docs build.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
