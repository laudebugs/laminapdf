# Q4h — Troubleshooting for Rust, the command line, extraction and rendering

| Field | Value |
| --- | --- |
| ID | `Q4h` |
| Status | `ongoing` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | None |
| Branch | `docs/q4h-troubleshooting` |
| PR title | `docs: troubleshooting for Rust, the command line, extraction and rendering` |
| Size | S–M |
| PR | — |

## Goal

The troubleshooting page has ten entries, all about JavaScript setup, encryption or memory. A Rust developer, a command-line user, or someone whose extracted text comes out in the wrong order finds nothing. This todo adds entries for each area, written symptom first, so a reader matches what they see to a fix. The command line already prints each error's docs page (`= see: <docs URL>`), so this todo covers troubleshooting alone.

## Context

- [Docs research: recommendation 6](../../research/docs-site/developer-docs.md#recommendations-in-order); Robillard and DeLine on matching a scenario.
- `apps/docs/src/content/docs/troubleshooting.mdx` and `docs/errors/` (the code pages).
- Issues and support questions so far, for the real symptoms.

## Scope

**In scope**

- Entries for Rust (features, build targets, the version pin), the command line (exit codes, passwords, overwrites), extraction (reading order, missing text, OCR), rendering (backend not found, blank pages).
- The page split into sections by area, or one page per area if it grows past about twenty entries.

**Out of scope**

- New error codes.

## Tasks

- [ ] Collect symptoms from issues and the code pages; pick the ones readers hit.
- [ ] Write each entry: the symptom as the reader sees it, why, the fix, a link to the code page.
- [ ] Link the entries from the guides they belong to.

## Acceptance criteria

- [ ] Each area has at least three entries.
- [ ] Every entry names a symptom a reader can see, and links its error code where one exists.

## Post-merge checks

None.

## Tests

The docs build and `check docs-prose`.

## Docs to update

The troubleshooting page; links from guides.

## Risks and open questions

- Entries written without real reports guess at symptoms; prefer ones seen in issues, tests or the corpus.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
