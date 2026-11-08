# B7d — Docs: a Result tab beside each example that makes a document

| Field | Value |
| --- | --- |
| ID | `B7d` |
| Status | `ongoing` |
| Plan | [Docs examples §2](../../plans/docs-examples.md#result-and-output-tabs-b7d) |
| Round / milestone | Round B (library) — docs |
| Depends on | [B7a](../done/B7a-docs-site.md) |
| ADRs | Follows 0011 (same-site loads, strict headers) |
| Branch | `feat/b7d-docs-example-results` |
| PR title | `feat(docs): show each example's result beside its code` |
| Size | M (200–400 changed lines of code; the WebP pictures don't count) |
| PR | — |

## Goal

Let a reader see what an example makes without leaving the page. Every example that makes a PDF gets Code and Result tabs: Result shows each page as a picture, with the link to open the PDF under it. The text-extraction recipe gets an Output tab with the Markdown it prints. Setup and test examples keep their code alone.

## Context

- [Docs examples plan](../../plans/docs-examples.md) §1 lists every page and what it gets; §2 the design.
- [Research: showing what an example makes](../../research/docs-site/live-examples.md): why pictures first, and why not a hosted sandbox.
- `apps/docs/src/components/Snippet.astro` shows the tested example files; `apps/docs/plugins/integrations.mjs` copies the golden PDFs to `/examples/pdf/`.
- `fixtures/render-golden/README.md` and `tools/render`: the render goldens at 72 dpi, from the same PDFs.

## Scope

**In scope**

- An `Example` component (language tabs plus Result or Output), used on every page in the plan's §1 table.
- Pictures of every page of every example golden, 144 dpi WebP, with a manifest of each PDF's SHA-256, and the command that writes them.
- A `cargo xtask check docs` step that fails when a golden PDF's hash differs from the manifest.
- The Markdown golden for `extract_markdown`'s output, if the example tests don't keep one yet.

**Out of scope** (and which todo covers it)

- Editing and running examples: [B7e](B7e-docs-edit-and-run.md).
- The report recipe's preview: it has no golden PDF; a later todo can add one.

## Tasks

- [ ] `tools/render`: a mode that renders every page of `fixtures/golden/examples/*.pdf` at 144 dpi to `apps/docs/public/examples/preview/<example>-<page>.webp`, and writes `manifest.json` (example, pages, page sizes, PDF SHA-256); wire it as `cargo xtask docs-previews`.
- [ ] `cargo xtask check docs`: compare the manifest's hashes with the golden PDFs; on a mismatch, name the example and the command.
- [ ] `apps/docs/src/components/Example.astro`: TypeScript and Rust tabs (synced like today's `syncKey="lang"`), plus a Result tab (pictures with alt text "Page N of the example's PDF", lazy-loaded; the open link and the file size under them) or an Output tab (the text).
- [ ] Replace the `<Tabs>` + `<Snippet>` pairs and the "open the reference PDF" sentences on every page in the plan's §1 table; keep `Snippet` for code-only examples.
- [ ] `apps/docs` tests: the component's markup (tabs, alt text, link), and that every example with a golden PDF has its pictures.

## Acceptance criteria

- [ ] Every page in the plan's §1 table shows the tabs it lists; no recipe page keeps a bare "open the reference PDF" link.
- [ ] Changing an example's golden PDF without `cargo xtask docs-previews` fails `cargo xtask check docs` with the example's name.
- [ ] The built site loads no picture until its tab opens, and makes no request to another origin.
- [ ] The pictures pass the docs site's checks (alt text, same-site loads).

## Post-merge checks

- [ ] The docs preview of the PR shows the Result tabs.

## Tests

`apps/docs` tests for the component and the manifest; the xtask check's unit test with a changed hash.

## Docs to update

`apps/docs/README.md` (the component and the command), CLAUDE.md's docs site line, the plan's to-do.

## Risks and open questions

- WebP size: 144 dpi pages are about 50–150 KB each; about 20 pages in all. If the repository growth matters, AVIF or a lower density for thumbnails.
- hayro or PDFium for the pictures: hayro needs no Python and is already built by `tools/render`; PDFium matches the reader. Pick hayro unless a page differs visibly.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
