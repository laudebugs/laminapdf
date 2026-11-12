# Q4h — Troubleshooting for Rust, the command line, extraction and rendering

| Field | Value |
| --- | --- |
| ID | `Q4h` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | None |
| Branch | `docs/q4h-troubleshooting` |
| PR title | `docs: troubleshooting for Rust, the command line, extraction and rendering` |
| Size | S–M |
| PR | [#457](https://github.com/laudebugs/laminapdf/pull/457) |

## Goal

The troubleshooting page has ten entries, all about JavaScript setup, encryption or memory. A Rust developer, a command-line user, or someone whose extracted text comes out in the wrong order finds nothing. This todo adds entries for each area, written symptom first, so a reader matches what they see to a fix. The command line already prints each error's docs page (`= see: <docs URL>`), so this todo covers troubleshooting alone.

## Context

- [Docs research: recommendation 6](../../research/docs-site/developer-docs.md#recommendations-in-order); Robillard and DeLine on matching a scenario.
- `apps/docs/src/content/docs/troubleshooting.mdx` (now `troubleshooting/`) and `docs/errors/` (the code pages).
- Issues and support questions so far, for the real symptoms.

## Scope

**In scope**

- Entries for Rust (features, build targets, the version pin), the command line (exit codes, passwords, overwrites), extraction (reading order, missing text, OCR), rendering (backend not found, blank pages).
- The page split into sections by area, or one page per area if it grows past about twenty entries.

**Out of scope**

- New error codes.

## Tasks

- [x] Collect symptoms from issues and the code pages; pick the ones readers hit.
- [x] Write each entry: the symptom as the reader sees it, why, the fix, a link to the code page.
- [x] Link the entries from the guides they belong to.

## Acceptance criteria

- [x] Each area has at least three entries.
- [x] Every entry names a symptom a reader can see, and links its error code where one exists.

## Post-merge checks

None.

## Tests

The docs build and `check docs-prose`.

## Docs to update

The troubleshooting page; links from guides.

## Risks and open questions

- Entries written without real reports guess at symptoms; prefer ones seen in issues, tests or the corpus.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- 28 entries, past the scope's twenty, so the page became one page per area under `apps/docs/src/content/docs/troubleshooting/`, with a landing page (`index.mdx`) at the old `/troubleshooting/` URL and a sidebar group (`apps/docs/plugins/sidebar.mjs`):
  - `javascript.mdx`: the old setup and memory entries, plus npm's `0.0.0` placeholder (8);
  - `rust.mdx`: the placeholder, alpha version requirements, missing features, `explain`, the minimum Rust version (5);
  - `command-line.mdx`: exit 3 under `set -e`, `overwrite-refused`, passwords, `binary-output-to-terminal`, `invalid-page-range` (5);
  - `files.mdx`: passwords, signed files and `limit-exceeded`, which apply in every language (3);
  - `extraction.mdx`: `extract-needs-ocr`, OCR layers, `extract-column-order-uncertain`, `extract-struct-tree-ignored`, `unmapped-glyph`, dropped furniture (6);
  - `rendering.mdx`: `render-backend-unavailable`, `render-too-large`, hayro against PDFium, output-name clashes (`invalid-command-line`), `output-unwritable` (5).
- The repository has no issues or support questions yet, so the symptoms come from the code pages in `docs/errors/`, the CLI's own refusals (`crates/laminapdf-cli/src/commands/render.rs`), the render limits and Cargo's own error messages.
- Build targets: no entry. Nothing in the tests or the code pages shows a target-specific failure, and the risk note asks for real symptoms rather than guesses.
- Links in: the Rust quick start, the CLI's scripts and extract-and-render pages, the extraction recipe, how a page is drawn, untrusted files, and the Workers quick start and recipe (now pointing at the JavaScript page's Worker entry).
- The install page named two of the `laminapdf` crate's five default features; it now lists all five, plus `build`, `extract` and `explain`, which the Rust entries point to.
- The release check (`apps/docs/plugins/release.mjs`) rejects any manifest line that names a version other than the release, so the Rust entry about version requirements quotes the requirement alone (`"0.2"`).
- `laminapdf extract` has no flags for reading order, kept spans or furniture; the extraction page says so, and its fixes give the TypeScript and Rust options.
