# Q1b — Docs writing guideline and the `docs-prose` check

| Field | Value |
| --- | --- |
| ID | `Q1b` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q1h](./Q1h-docs-story.md), [Q1k](./Q1k-internal-ids-out-of-reader-text.md) |
| ADRs | None |
| Branch | `feat/q1b-docs-prose-check` |
| PR title | `feat(xtask): docs-prose check for reader-facing text` |
| Size | M |
| PR | — |

## Goal

Make the writing rules for reader-facing text checkable while we write. Turn [`docs/guidelines/docs-writing.md`](../../guidelines/docs-writing.md) (written with the Round Q plan) into a `cargo xtask check docs-prose` step, part of `cargo xtask check docs`, over the docs site's pages, `docs/errors/*.md` and package READMEs. It starts at warning level so the rewrite todos can land; [Q1l](./Q1l-docs-checks-strict.md) makes it fail.

## Context

- [Docs writing guideline](../../guidelines/docs-writing.md): the rules this checks.
- [Quality plan §4.1](../../plans/quality.md#4-standards): which rules are automated and why not Vale.
- `tools/xtask/src/docs_links.rs`: the existing Markdown walker (reuse its file discovery and its code-fence handling).
- Design language §9 (writing style).

## Scope

**In scope**

- `cargo xtask check docs-prose [--strict] [--json]`: strips frontmatter, fenced and inline code, JSX/MDX components, tables and link URLs; then checks the guideline's rules 1–8 (frontmatter `title` and `description` ≤ 160; sentence length warn > 30 / fail > 40 words, paragraphs ≤ 100; the opening paragraph of guides and recipes; internal IDs: `ADR \d{4}`, todo IDs like `B8b`, `Round`, `fast lane`, `golden`, repository paths; glossary terms linked on first use or allowed in frontmatter `prose.allow`; banned words and `!`; a sentence before each `<Snippet>`; sentence-case headings with a proper-noun list).
- Files: `apps/docs/src/content/docs/**/*.mdx`, `docs/errors/*.md`, `packages/*/README.md`.
- Warnings print `path:line: rule: message`; `--strict` turns them into failures; `check docs` runs it without `--strict` until Q1l.
- Config in `tools/xtask/docs-prose.toml`: banned words, proper nouns, glossary terms, the internal-ID patterns and allowed exceptions (e.g. `R2`–`R6` security handler revisions).

**Out of scope** (and which todo covers it)

- Fixing the text it reports (Q1i, Q1k).
- Example code checks ([Q1c](./Q1c-examples-check.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write the stripping pass with unit tests on tricky MDX (imports, `<Tabs>`, nested lists, tables).
- [ ] Implement each rule as a function with its own tests (pass and fail cases).
- [ ] Wire into `check docs`; print a summary count per rule.
- [ ] Record today's warning counts per rule in the PR description (the baseline Q1i–Q1k work down).

## Acceptance criteria

- [ ] `cargo xtask check docs-prose` reports each rule with file and line; `--strict` exits non-zero on any report.
- [ ] Unit tests cover every rule both ways.
- [ ] `cargo xtask check docs` still passes on `main` (warnings only).

## Post-merge checks

None.

## Tests

Unit tests in `tools/xtask` for the stripper and each rule; a fixture `.mdx` under `tools/xtask/tests/` with one violation per rule.

## Docs to update

The guideline ("How it's checked" points at the command), CLAUDE.md commands list, plan §9.

## Risks and open questions

False positives in the sentence splitter (abbreviations, version numbers): keep an abbreviation list; a rule that stays noisy is dropped, not suppressed page by page.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
