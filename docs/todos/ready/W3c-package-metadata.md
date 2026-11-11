# W3c — Package metadata written for users

| Field | Value |
| --- | --- |
| ID | `W3c` |
| Status | `ready` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | None |
| ADRs | Follows [0017](../../decisions/0017-versioning-and-release.md) (what the packages publish) |
| Branch | `docs/w3c-package-metadata` |
| PR title | `docs(packages): descriptions, keywords and READMEs written for users` |
| Size | M (six package manifests, thirteen crate manifests, their READMEs, one check) |
| PR | — |

## Goal

npm and crates.io rank packages on their name, description and keywords, and show the README as the package's page. Several of Lamina PDF's describe themselves in the project's internal language: the render and viewer packages cite ADR numbers and todo IDs, three READMEs say a published package "is private", and READMEs link files by repository-relative paths that break on npm and crates.io. This todo rewrites that metadata for someone choosing a package, and adds a check so internal IDs and broken links don't come back.

## Context

- [Discoverability research §5](../../research/discoverability/search-and-answer-engines.md#5-package-registries-rank-on-metadata-and-readmes) (what the registries read) and §7 gap 6 (the audit's list).
- The six published npm packages (`packages/{sdk,wasm,render,render-hayro,render-pdfium,viewer}/package.json`), the CLI's generated packages (`tools/cli-release/generate.ts`), and the thirteen `publish = true` crates.
- [Docs writing guideline](../../guidelines/docs-writing.md) (no internal IDs in reader-facing text) and `cargo xtask check docs-prose`, which already reads package READMEs.

## Scope

**In scope**

- Descriptions: what the package does and for whom, one sentence, no ADR or todo ID.
- Keywords: the jobs people search for (npm: up to about ten; crates.io: five), and crates.io categories checked against its list.
- `bugs` (npm) pointing at the issue tracker; `homepage` per package where a docs page fits better than the website.
- READMEs: the first paragraph says what the package does; no "private" for a published package; absolute links (docs site or GitHub) instead of repository-relative ones.
- A check in `cargo xtask check docs`: no ADR or todo ID in a published package's description, and no relative link in a published README that the registry can't resolve.

**Out of scope** (and which todo covers it)

- The root README: [W3b](./W3b-public-repository-links.md).
- New docs pages: [W3g](./W3g-choosing-a-pdf-library.md), [W3h](./W3h-docs-titles-for-questions.md).

## Tasks

- [ ] Search npm and crates.io for "pdf" jobs (generate, parse, extract text, render, viewer, edit, sign, form) and note which keywords comparable packages use; pick keywords per package from that list.
- [ ] Rewrite each description and the first README paragraph; keep the SDK's, which already reads well, unless the keywords suggest a change.
- [ ] Add `bugs` to every published npm package and the CLI generator.
- [ ] Replace relative README links with absolute ones; decide per link between the docs site and the GitHub file.
- [ ] Add the check to xtask's docs checks (the `todo-id` rule's ID list and an `ADR \d{4}` pattern), with tests.
- [ ] A changeset for the npm metadata (patch).

## Acceptance criteria

- [ ] No published package's description or README names an ADR or a todo ID, or says the package is private.
- [ ] `pnpm pack` of each npm package and `cargo package --list` of each crate show READMEs whose links are absolute.
- [ ] The new check fails on a fixture description with "ADR 0004" and on a fixture README with `../sdk/README.md`, and passes on the repository.

## Post-merge checks

- [ ] After the next release, npm's and crates.io's pages for `@laminapdf/sdk`, `@laminapdf/viewer` and `laminapdf` show the new descriptions and working links.

## Tests

xtask unit tests for the check; `cargo xtask check docs`.

## Docs to update

The package READMEs, CLAUDE.md's `check docs` line, and the [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- crates.io allows five keywords; pick the ones people search for, not the ones that describe the internals.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
