# Q1b — The `docs-prose` check for reader-facing text

| Field | Value |
| --- | --- |
| ID | `Q1b` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| ADRs | None |
| Branch | `feat/q1b-docs-prose-check` |
| PR title | `feat(xtask): docs-prose check for reader-facing text` |
| Size | M |
| PR | [#144](https://github.com/laudebugs/lamina/pull/144) |

## Goal

The [docs-writing guideline](../../guidelines/docs-writing.md) (written with the Round Q plan) needs a check that runs while people write. This todo adds `cargo xtask check docs-prose` to `cargo xtask check docs`. It is strict on every file a PR adds or changes, so new writing is checked as it is written, and it only warns on untouched files until the rewrite todos clear them.

## Context

- The [docs-writing guideline](../../guidelines/docs-writing.md): the rules, and its "How it's checked" section.
- [Quality plan §4](../../plans/quality.md#4-standards): why the check is written in xtask rather than Vale.
- `tools/xtask/src/docs_links.rs` (Markdown file discovery and code-fence handling to reuse) and `tools/xtask/src/changes.rs` (changed paths against the merge base).

## Scope

**In scope**

- `cargo xtask check docs-prose [--all-strict] [--warnings] [--json]` over `apps/docs/src/content/docs/**/*.mdx`, `docs/errors/*.md` and `packages/*/README.md`.
- A stripping pass: frontmatter, fenced and inline code, MDX imports and components, tables and link URLs.
- Rules from the guideline's "Write in plain English" and "Show code" sections, plus the opening paragraph of guides and recipes:
  - frontmatter `title` and `description` (≤ 160 characters);
  - sentences ≤ 40 words, paragraphs ≤ 100 words;
  - internal IDs: ADR numbers, todo IDs, "Round", "fast lane", "golden", CI, repository paths;
  - glossary terms linked or listed under `prose.allow`;
  - banned words and exclamation marks;
  - one term per thing (the guideline's fixed spellings);
  - sentence-case headings;
  - a sentence before each `<Snippet>`.
- Strict on files changed since the merge base with `main`; warnings elsewhere; `--all-strict` makes every file strict (Q1l turns it on).
- Configuration in `tools/xtask/docs-prose.toml`: banned words, proper nouns, glossary terms and exceptions to the ID rule. Deviation: the ID patterns live in the code. Todo IDs match only with their letter suffix, so `R2`–`R6`, `A4` and `Q3` never match; `id_exceptions` lists any other ID-like word.

**Out of scope** (and which todo covers it)

- Fixing the text it reports ([Q1i](../done/Q1i-guides-create.md), [Q1k](../done/Q1k-internal-ids-out-of-reader-text.md) and the other rewrites).
- Checks on example code ([Q1c](../done/Q1c-examples-checks.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Write the stripping pass with unit tests on tricky MDX: imports, `<Tabs>`, nested lists, tables and admonitions.
- [x] Write each rule as its own function with pass and fail tests; read configuration from `docs-prose.toml`.
- [x] Add the changed-files logic by reusing `changes.rs`; make failures on changed files exit non-zero.
- [x] Wire the step into `cargo xtask check docs`; print `path:line: rule: message` and a count per rule.
- [x] Run it on `main`; record the warning counts per rule in the PR description as the baseline the rewrite todos work down.
- [x] Add a line to the guideline's "How it's checked" if anything differs from what it says.

## Acceptance criteria

- [x] Each rule reports file and line; a report on a changed file fails the step; `--all-strict` fails on any report.
- [x] Unit tests cover every rule both ways.
- [x] `cargo xtask check docs` passes on `main`.

## Post-merge checks

None.

## Tests

Unit tests in `tools/xtask` for the stripper and each rule, both ways. Deviation: xtask has no `tests/` folder, so the cases live inline in `docs_prose/{extract,rules}.rs` (one passing and one failing case per rule) instead of a fixture tree.

## Docs to update

The docs-writing guideline if needed, the commands list in CLAUDE.md, plan §9.

## Risks and open questions

The sentence splitter will meet abbreviations and version numbers: keep an abbreviation list. A rule that stays noisy is dropped from the check, not suppressed page by page.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
