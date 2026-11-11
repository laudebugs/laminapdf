# W3b — The repository is public: say so everywhere a reader or engine looks

| Field | Value |
| --- | --- |
| ID | `W3b` |
| Status | `ongoing` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | None |
| ADRs | Implements [0026 §6](../../decisions/0026-error-and-diagnostic-contract.md#6-docsurl-and-explain-pages)'s planned move of `docsUrl` to the docs site's code pages |
| Branch | `docs/w3b-public-repository-links` |
| PR title | `docs: the repository is public: GitHub links on the docs, error links on the docs site` |
| Size | M (a constant, regenerated tables, docs site config, many small prose edits) |
| PR | [#436](https://github.com/laudebugs/laminapdf/pull/436) |

## Goal

The GitHub repository went public, but the project still describes and builds itself as if it were private: the docs site hides its "Edit this page" and GitHub links, every error's `docsUrl` points at a GitHub-rendered Markdown file instead of the docs site's code page, and READMEs and plans say the repository is private. Search engines and answer engines weigh links between a project's site and its repository, and visitors judge a library by its source. This todo makes every surface treat the repository as public, and gives the README a first screen that names every part of the project.

## Context

- [Discoverability research §7](../../research/discoverability/search-and-answer-engines.md#7-what-lamina-pdf-exposes-today-audit-2026-11-11), gap 1.
- `apps/docs/plugins/repository.mjs`: `GITHUB = process.env.LAMINA_PDF_DOCS_GITHUB === "1"` ("off while it is private"), which turns on the header's GitHub link, `editLink` and TypeDoc's source links.
- `fixtures/codes.json`'s `docsUrl` base and suffix; ADR 0026 §6 says the base becomes `https://docs.laminapdf.com/reference/codes/<code>/` once production serves the code pages. Check the docs site's actual path for code pages before changing the constant.
- [docs/public-readiness.md](../../public-readiness.md): the owner's visibility checklist; tick what this todo finishes.
- [Plan §4](../../plans/discoverability.md#4-decisions-for-the-owner), decision 3: the repository's description and topics (a GitHub setting for the owner).

## Scope

**In scope**

- GitHub links on the docs site by default; the environment flag removed.
- `docsUrl` moved to the docs site, regenerated on every surface (`cargo xtask gen codes`), with a test that every code's page exists in the built docs.
- Every "the repository is private" statement in READMEs, plans, the website plan §4, CLAUDE.md and code comments updated or removed.
- The root README: one sentence each for the library, the CLI and MCP server, the web reader, the website and the agent docs.

**Out of scope** (and which todo covers it)

- Package READMEs' own wording and links: [W3c](../done/W3c-package-metadata.md).
- Linking llms.txt from the README: [W3f](../ready/W3f-agent-discovery.md).

## Tasks

- [ ] Ask the owner to set the repository's description and topics (the homepage is already `https://laminapdf.com`) (plan §4 decision 3); record the result in the plan.
- [ ] `apps/docs/plugins/repository.mjs`: links on unconditionally; remove `LAMINA_PDF_DOCS_GITHUB` from the config, workflows, README and CLAUDE.md. Check the edit links point at `main` and the right path for generated pages (error pages, TypeDoc), or leave those without one.
- [ ] `fixtures/codes.json`: the docs site's base and suffix; `cargo xtask gen codes`; update ADR 0026 §6's sentence about the move to past tense with the date; update `crates/laminapdf/tests/error_contract.rs` fixtures and `fixtures/errors/contract.json` as generation requires.
- [ ] A docs-site build check: every code in `codes.json` has a built page at the `docsUrl` path.
- [ ] `grep -rn "is private\|while it is private\|private repository"` across the repository; fix each that refers to the repository (not to private npm packages, which stay private).
- [ ] The root README's first screen: what Lamina PDF is, then a line and a link for each part.

## Acceptance criteria

- [ ] A docs page shows "Edit page" and a GitHub link in its header in a default build.
- [ ] `laminapdf explain wrong-password` and an SDK `LaminaPdfError` give a `docsUrl` on `docs.laminapdf.com` that the built site serves.
- [ ] No current statement says the repository is private (done todos, ADRs and research that record the past stay as they are).
- [ ] `cargo xtask check docs` and the docs build pass.

## Post-merge checks

- [ ] After the next release, `docsUrl` of a sample error opens its page on `docs.laminapdf.com`.
- [ ] The repository page on GitHub shows the description, the website and the topics.

## Tests

- The docs site's plugin tests: links on, and the code-page check.
- `cargo xtask gen codes --check` and the error-contract tests over the new base.

## Docs to update

ADR 0026 §6, `apps/docs/README.md`, the [website plan §4](../../plans/website.md#4-the-website), [docs/public-readiness.md](../../public-readiness.md), CLAUDE.md, the root README, and the [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- Changing `docsUrl` is visible to callers who store it; ADR 0026 promises only that the code ends the path, which stays true. Note it in a changeset.
- Generated pages (TypeDoc, error pages) have no source file to edit; leave their edit link off rather than point at a generated path.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
