# Q4b — A guide to reading text and Markdown out of PDFs, in TypeScript and Rust

| Field | Value |
| --- | --- |
| ID | `Q4b` |
| Status | `ongoing` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0030](../../decisions/0030-extraction-output-and-model-plugins.md) (the extraction output) |
| Branch | `docs/q4b-extraction-guide` |
| PR title | `docs: a guide to extracting text and Markdown in TypeScript and Rust` |
| Size | M |
| PR | — |

## Goal

Reading PDFs is half of Lamina PDF, and the docs barely show it. Extraction has one recipe and a command-line page, and the home page's "What works today" table has no extraction row. Missing content is the largest complaint the research found, so this todo writes the guide a developer needs to get text, Markdown, JSON and chunks out of a PDF from code, in both languages, and puts extraction on the home page.

## Context

- [Docs research: recommendation 2](../../research/docs-site/developer-docs.md#recommendations-in-order) and the [audit](../../research/docs-site/developer-docs/current_docs_site.md)'s missing topics.
- `apps/docs/src/content/docs/recipes/extract-markdown.mdx` and `cli/extract-and-render.mdx`: what exists.
- The extraction API: `doc.extract` in `@laminapdf/sdk`, `Document::extract` in `laminapdf`, and `crates/laminapdf/examples/extract_markdown.rs`.
- The diagrams Q3e drew for extraction ("From page to Markdown", "Reading order").

## Scope

**In scope**

- A guide, `guides/extract-text`, in the "Read and edit PDFs" group: text, Markdown, the JSON output and chunks, reading order, what is dropped and why, tables, and the warnings to expect.
- Tested examples in both languages for each output, with an Output tab.
- An extraction row in the home page's "What works today" table.
- Links from the extraction recipe and the command-line page to the guide.

**Out of scope**

- Models (layout, OCR) beyond a short pointer to the API.
- The viewer and rendering (Q4c).

## Tasks

- [ ] Outline the guide from a reader's questions: how do I get the text, as Markdown, with positions, in chunks for retrieval?
- [ ] Write the examples in `packages/sdk/examples/` and `crates/laminapdf/examples/`, with their tests and output goldens.
- [ ] Write the page; reuse the extraction diagrams.
- [ ] Add the home page row and the links; place the guide in the sidebar and llms.txt.

## Acceptance criteria

- [ ] The guide has a tested TypeScript and Rust example for text, Markdown, JSON and chunks.
- [ ] The home page lists extraction with its status.
- [ ] `check docs-prose` and `check docs-snippets` pass.

## Post-merge checks

None.

## Tests

The new examples' tests and output goldens; the docs build.

## Docs to update

The guide, the home page, the recipe and command-line page's links.

## Risks and open questions

- The JSON output is long; show a trimmed excerpt with the full file linked, as the recipe does.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
