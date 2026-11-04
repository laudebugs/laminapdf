# Q1m — Plain-English guides: reading and editing PDFs

| Field | Value |
| --- | --- |
| ID | `Q1m` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1h](./Q1h-docs-story.md) |
| Blocks | [Q1l](../ongoing/Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `docs/q1m-read-edit-guides` |
| PR title | `docs(site): plain-English guides for reading and editing PDFs` |
| Size | M |
| PR | [#182](https://github.com/laudebugs/laminapdf/pull/182) |

## Goal

This todo rewrites the "Read and edit" and "Production" guides to the [docs-writing guideline](../../guidelines/docs-writing.md). It also restructures them: "Open, edit and save" splits into "Edit a PDF" and "Save a PDF", "Limits" becomes "Open files you don't trust", "Encrypted files" becomes "Open password-protected files", and the password-order sentence is corrected.

## Context

- [Quality plan §2.1](../../plans/quality.md#21-docs-prose-and-navigation): the overlay passage and the password-order error.
- `apps/docs/src/content/docs/guides/{open-edit-and-save,encrypted-files,limits,off-the-main-thread}.mdx` and `concepts/errors-and-warnings.mdx` (moved there by Q1h); `crates/laminapdf-syntax/src/crypt/mod.rs` (owner password first for R5/R6).

## Scope

**In scope**

- Split `open-edit-and-save.mdx`; rename two pages with redirects; rewrite all five; fix the password sentence ("tried as both").
- Their places in the sidebar.

**Out of scope** (and which todo covers it)

- The create guides ([Q1i](./Q1i-guides-create.md)).

## Tasks

- [x] Claim by opening the work PR from a fresh worktree off the updated `main`.
- [x] Split and rename the pages; add redirects; update the sidebar.
- [x] Rewrite each page with `/writing`, keeping every fact.
- [x] Correct the password sentence against `crypt/mod.rs`.
- [x] Run `docs-prose`; build the site and follow the old URLs.
- [x] Run `/writing-review` after opening the PR; fix the findings.

## Acceptance criteria

- [x] `docs-prose` reports nothing on these pages; the old URLs redirect.
- [x] The password sentence matches the code.

## Post-merge checks

None.

## Tests

`docs-prose`, the docs build.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- The guides are now Edit a PDF, Save a PDF, Open password-protected files (`password-protected-files`), Open files you don't trust (`untrusted-files`) and Keep the page responsive (`keep-the-page-responsive`); the four old URLs redirect. The concepts page Errors and warnings is rewritten too.
- Two facts were corrected against the code: the password order (owner first for R5/R6) and encrypted saves (an edited encrypted file saves only as a full rewrite with `removeEncryption`). The Unicode-password and Web Worker passages were completed.
