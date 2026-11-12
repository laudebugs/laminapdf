# Q4b — A guide to reading text and Markdown out of PDFs, in TypeScript and Rust

| Field | Value |
| --- | --- |
| ID | `Q4b` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0030](../../decisions/0030-extraction-output-and-model-plugins.md) (the extraction output) |
| Branch | `docs/q4b-extraction-guide` |
| PR title | `docs: a guide to extracting text and Markdown in TypeScript and Rust` |
| Size | M |
| PR | [#461](https://github.com/laudebugs/laminapdf/pull/461) |

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

- [x] Outline the guide from a reader's questions: how do I get the text, as Markdown, with positions, in chunks for retrieval?
- [x] Write the examples in `packages/sdk/examples/` and `crates/laminapdf/examples/`, with their tests and output goldens.
- [x] Write the page; reuse the extraction diagrams.
- [x] Add the home page row and the links; place the guide in the sidebar and llms.txt.

## Acceptance criteria

- [x] The guide has a tested TypeScript and Rust example for text, Markdown, JSON and chunks.
- [x] The home page lists extraction with its status.
- [x] `check docs-prose` and `check docs-snippets` pass.

## Post-merge checks

None.

## Tests

The new examples' tests and output goldens; the docs build.

## Docs to update

The guide, the home page, the recipe and command-line page's links.

## Risks and open questions

- The JSON output is long; show a trimmed excerpt with the full file linked, as the recipe does.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- The guide is `apps/docs/src/content/docs/guides/extract-text.mdx`, "Extract text and Markdown", after "Save a PDF" in the Read and edit PDFs group. It covers the four outputs, then reading order (a table of the `readingOrder` values), what extraction leaves out, tables, and the warnings and `textLayer` statuses to expect. It ends with a pointer to the model plug-ins (`extractWithModels`, `ExtractOptions::with_models`). It reuses the `extract-pipeline` and `reading-order` diagrams.
- Three new example pairs sit beside `extract_markdown`, which the guide reuses for Markdown:
  - `extract_text` / `extract-text.ts`: the text format.
  - `extract_blocks` / `extract-blocks.ts`: each block's page, kind and box, read from the JSON output (left, top, width and height, as the JSON writes them). The Rust twin rounds to a thousandth, as the JSON does, so the two print the same numbers.
  - `extract_chunks` / `extract-chunks.ts`: chunks with their headings, pages and size, at a 60-token budget so the short invoice gives more than one.
- Each prints a golden for `golden/examples/invoice.pdf` in `fixtures/extract/examples/` (`invoice.txt`, `invoice-blocks.txt`, `invoice-chunks.txt`). `crates/laminapdf/tests/examples.rs` writes and checks them, and `packages/sdk/test/extract.test.ts` checks the TypeScript twins against the same files. `Example.astro` now shows `.txt` outputs as well as `.md`.
- The three examples are added to `READERS` in `crates/laminapdf/tests/examples.rs` and `tools/xtask/src/examples.rs`, to `[[example]]` in the crate's `Cargo.toml`, and as pairs in `tools/xtask/docs-snippets.toml`. A single list in place of the two `READERS` copies was left out of scope.
- Links: the home page's "What works today" table has an Extract row; the extraction recipe opens with a pointer to the guide, and the command line's extract section ends with one. The guide links to the extraction troubleshooting page from Q4h.
