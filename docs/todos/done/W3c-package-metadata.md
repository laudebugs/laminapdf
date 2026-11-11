# W3c — Package metadata written for users

| Field | Value |
| --- | --- |
| ID | `W3c` |
| Status | `done` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | None |
| ADRs | Follows [0017](../../decisions/0017-versioning-and-release.md) (what the packages publish) |
| Branch | `docs/w3c-package-metadata` |
| PR title | `docs(packages): descriptions, keywords and READMEs written for users` |
| Size | M (six package manifests, thirteen crate manifests, their READMEs, one check) |
| PR | [#432](https://github.com/laudebugs/laminapdf/pull/432) |

## Goal

npm and crates.io rank packages on their name, description and keywords, and show the README as the package's page. Several of Lamina PDF's describe themselves in the project's internal language: the render and viewer packages and the `laminapdf-extract`, `laminapdf-render` and `laminapdf-render-pdfium` crates cite ADR numbers and todo IDs, and four READMEs (`@laminapdf/wasm`, `viewer`, `render-hayro`, `render-pdfium`) say a published package "is private". This todo rewrites that metadata for someone choosing a package, and adds a check so internal IDs don't come back.

## Context

- [Discoverability research §5](../../research/discoverability/search-and-answer-engines.md#5-package-registries-rank-on-metadata-and-readmes) (what the registries read) and §7 gap 6 (the audit's list).
- The six published npm packages (`packages/{sdk,wasm,render,render-hayro,render-pdfium,viewer}/package.json`), the CLI's generated packages (`tools/cli-release/generate.ts`), and the thirteen `publish = true` crates.
- [Docs writing guideline](../../guidelines/docs-writing.md) (no internal IDs in reader-facing text) and `cargo xtask check docs-prose`, which already reads package READMEs.

## Scope

**In scope**

- Descriptions: what the package does and for whom, one sentence, no ADR or todo ID.
- Keywords: the jobs people search for (npm: up to about ten; crates.io: five), and crates.io categories checked against its list.
- `bugs` (npm) pointing at the issue tracker; `homepage` per package where a docs page fits better than the website.
- READMEs: the first paragraph says what the package does; no "private" for a published package. Repository-relative links resolve on npm and crates.io through the `repository` field now that the repository is public; check one package page on each registry, and change only links that point at files a reader shouldn't need (ADRs, todos).
- A check in `cargo xtask check docs`: no ADR or todo ID in a published package's description or the first paragraph of its README.

**Out of scope** (and which todo covers it)

- The root README: [W3b](../ready/W3b-public-repository-links.md).
- New docs pages: [W3g](../done/W3g-choosing-a-pdf-library.md), [W3h](../ready/W3h-docs-titles-for-questions.md).

## Tasks

- [x] Search npm and crates.io for "pdf" jobs (generate, parse, extract text, render, viewer, edit, sign, form) and note which keywords comparable packages use; pick keywords per package from that list.
- [x] Rewrite each description and the first README paragraph; keep the SDK's, which already reads well, unless the keywords suggest a change.
- [x] Add `bugs` to every published npm package and the CLI generator.
- [x] Open `@laminapdf/sdk` on npmjs.com and `laminapdf` on crates.io and follow their README links; fix any that don't resolve.
- [x] Add the check to xtask's docs checks (the `todo-id` rule's ID list and an `ADR \d{4}` pattern), with tests.
- [x] A changeset for the npm metadata (patch).

## Acceptance criteria

- [x] No published package's description or README names an ADR or a todo ID, or says the package is private.
- [x] The new check fails on a fixture description with "ADR 0004" and on a fixture README whose first paragraph names a todo ID, and passes on the repository.

## Post-merge checks

- [ ] After the next release, npm's and crates.io's pages for `@laminapdf/sdk`, `@laminapdf/viewer` and `laminapdf` show the new descriptions, and their README links resolve.

## Tests

xtask unit tests for the check; `cargo xtask check docs`.

## Docs to update

The package READMEs, CLAUDE.md's `check docs` line, and the [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- crates.io allows five keywords; pick the ones people search for, not the ones that describe the internals.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
