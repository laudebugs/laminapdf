# W3g — A page that helps a developer choose a PDF library

| Field | Value |
| --- | --- |
| ID | `W3g` |
| Status | `ready` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | [W3c](./W3c-package-metadata.md) |
| ADRs | None |
| Branch | `docs/w3g-choosing-a-pdf-library` |
| PR title | `docs: choosing a PDF library, with what Lamina PDF does and doesn't do` |
| Size | M (one page and its sources) |
| PR | — |

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

- Benchmarks of other libraries: [U5b](./U5b-reader-benchmark.md) for readers; this page cites published figures only.

## Tasks

- [ ] Pick the libraries from the research (for example pdf-lib, PDFKit, pdf.js, jsPDF, pdfmake, and for Rust lopdf, printpdf, pdf-writer, krilla, hayro) and confirm each fact against its current docs.
- [ ] Draft the page; the owner approves every claim about another library and about Lamina PDF.
- [ ] Title and description that match the question's wording; a Markdown twin, as every page gets.

## Acceptance criteria

- [ ] Every fact about another library has a source link and a check date.
- [ ] The owner approved the page in the PR.
- [ ] `check docs-prose` passes.

## Post-merge checks

- [ ] W3i's next run asks the questions this page answers, and notes whether the page is cited.

## Tests

The docs build and its link checks; the nightly external link check covers the sources.

## Docs to update

The docs sidebar, and the [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- Facts about other libraries go stale; date each one, and note in the page how to report a mistake.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
