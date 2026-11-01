# Q1i — Plain-English guides: why first, terms explained, split and renamed

| Field | Value |
| --- | --- |
| ID | `Q1i` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1h](./Q1h-docs-story.md) |
| Blocks | [Q1l](./Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `docs/q1i-guides-plain-english` |
| PR title | `docs(site): plain-English guides that open with why` |
| Size | M |
| PR | — |

## Goal

Rewrite every guide to the [docs writing guideline](../../guidelines/docs-writing.md): open with what you'll make and when you need it, explain or link spec terms, move spec detail into "Details" asides, use one term per thing ("Cloudflare Worker", "Web Worker", "reference PDF", "the standard 14 fonts"). Split "Open, edit and save" into "Edit a PDF" and "Save a PDF", rename "Limits" to "Open files you don't trust" and "Encrypted files" to "Open password-protected files", and fix the password-order sentence.

## Context

- [Quality plan §2.1](../../plans/quality.md#21-docs-prose-and-navigation) (worst passages with rewrites), `apps/docs/src/content/docs/guides/*.mdx`, the guides Rex's [B8b](../ongoing/B8b-conformance-profiles-and-verapdf.md) adds (Accessible PDFs, Archival PDFs) if merged by then.
- `crates/lamina-syntax/src/crypt/mod.rs` (password order: owner first for R5/R6).

## Scope

**In scope**

- Every page under `guides/` (and B8b's if present): opening paragraph, terms, Details asides, consistent terms, `<Snippet>` introduced by a sentence.
- Split `open-edit-and-save.mdx`; rename two pages with redirects; place them in the sidebar per plan §3.

**Out of scope** (and which todo covers it)

- New pages ([Q1h](./Q1h-docs-story.md), [Q1j](./Q1j-recipes-and-troubleshooting.md)).
- Error-code pages ([Q1k](./Q1k-internal-ids-out-of-reader-text.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Rewrite page by page; run `docs-prose`; build; read each page aloud once.

## Acceptance criteria

- [ ] `docs-prose --strict` passes on `guides/**`.
- [ ] The encrypted-files sentence matches the code ("tried as both").

## Post-merge checks

None.

## Tests

`docs-prose`, the docs build.

## Docs to update

Plan §9.

## Risks and open questions

Rewrites can drop facts: keep every fact, move spec detail into Details rather than deleting it.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
