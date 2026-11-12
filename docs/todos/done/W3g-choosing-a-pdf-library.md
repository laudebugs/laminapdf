# W3g — A page that helps a developer choose a PDF library

| Field | Value |
| --- | --- |
| ID | `W3g` |
| Status | `done` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | [W3c](./W3c-package-metadata.md) |
| ADRs | None |
| Branch | `docs/w3g-choosing-a-pdf-library` |
| PR title | `docs: choosing a PDF library, with what Lamina PDF does and doesn't do` |
| Size | M (one page and its sources) |
| PR | [#438](https://github.com/laudebugs/laminapdf/pull/438) |

## Goal

"Which PDF library should I use for TypeScript?" and its Rust twin are the questions people ask search and answer engines before they pick a library. Answer engines cite pages that answer such questions with specific facts and sources. Lamina PDF has no such page, so engines can only describe it from its package metadata. This todo writes one docs page that answers the question fairly: which jobs each common library does, under which licence and on which runtimes, with every fact sourced, and where Lamina PDF fits and where it doesn't yet.

## Context

- [Discoverability research §4](../../research/discoverability/search-and-answer-engines.md#4-what-raises-the-chance-of-being-cited): the GEO study's finding that citations, quotations and statistics raise visibility.
- [Libraries research](../../research/libraries/shareable-libraries.md) and the [PDF library plan](../../plans/pdf-library.md): the existing comparisons of TypeScript and Rust PDF libraries.
- [Trust and proof plan §3](../../plans/trust-and-proof.md#3-rules-for-this-round): promise nothing the tests don't show.
- [Docs writing guideline](../../guidelines/docs-writing.md).

## Scope

**In scope**

- One docs page, "Choosing a PDF library", with a table per language (TypeScript, Rust) of jobs (generate, read, extract text, render, edit and save, forms, signatures, PDF/A and PDF/UA), licence and runtimes, each cell sourced from the library's own docs or repository with a date.
- Where Lamina PDF is not the right choice today, said plainly (its alpha status and the editing it lacks).
- Links to the How we test page and the Benchmarks page for Lamina PDF's own numbers.

**Out of scope**

- Benchmarks of other libraries: [U5b](../ready/U5b-reader-benchmark.md) for readers; this page cites published figures only.

## Tasks

- [x] Pick the libraries from the research (for example pdf-lib, PDFKit, pdf.js, jsPDF, pdfmake, and for Rust lopdf, printpdf, pdf-writer, krilla, hayro) and confirm each fact against its current docs.
- [x] Draft the page; the owner approves every claim about another library and about Lamina PDF.
- [x] Title and description that match the question's wording; a Markdown twin, as every page gets.

## Acceptance criteria

- [x] Every fact about another library has a source link and a check date.
- [x] The owner approved the page in the PR.
- [x] `check docs-prose` passes.

## Post-merge checks

- [ ] W3i's next run asks the questions this page answers, and notes whether the page is cited.

## Tests

The docs build and its link checks; the nightly external link check covers the sources.

## Docs to update

The docs sidebar, and the [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- Facts about other libraries go stale; date each one, and note in the page how to report a mistake.

## As built

- **The page:** `apps/docs/src/content/docs/start/choosing-a-pdf-library.mdx`, "Choosing a PDF library for TypeScript or Rust", in the sidebar after "What is Lamina PDF". It has two tables per language: the jobs (make, read, text, render, edit, forms, sign, PDF/A and PDF/UA), then licence, latest release and runtimes (or, for Rust, what each crate needs). Short notes follow each pair.
- **Libraries:** pdf-lib, LibPDF, PDFKit, jsPDF, pdfmake and PDF.js; lopdf, printpdf, pdf-writer, krilla, hayro and pdfium-render. Every cell about another library links to its own docs, README or registry entry. The page gives one check date, 11 November 2026, since every fact was checked that day. A dash means the library's docs don't say.
- **Facts:** two fact-checks read each library's own sources and recorded a quote and URL per cell. A claim the sources didn't support was narrowed or dropped: PDFKit's fonts and PDF/A only, pdfium-render reading but not making signatures, and no unsourced descriptions of PDF.js or PDFium.
- **Lamina PDF's row** links to its own guides. Edit is "partly", covering document info and page rotation, because appending pages to an open document isn't in the façade. Forms and signing are "no". The version comes from `{{version}}`.
- **Reporting a mistake:** the page links the repository's issues, which works now that the repository is public.
- **Approval:** the owner approved every claim in the session that drafted it, after reading the tables and the "Where Lamina PDF fits" section (2026-11-11), and the PR records it.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
