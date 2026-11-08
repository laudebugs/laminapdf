# Plan: prove correctness in public and close the trust gaps (Round T)

- **Status:** Draft
- **Date:** 2026-11-06
- **Asked for by:** the project owner (2026-11-06): research what PDF users want, complain about and find interesting in readers, editors, parsers and generators over ten years, and turn it into actionable plans and todos
- **Builds on:**
  - the [Hacker News research](../research/hacker-news/pdf-users-on-hacker-news.md) (3,755 stories and 56,907 comments, Nov 2016 to Nov 2026) and its [notes](../research/hacker-news/README.md)
  - the [product and UX research](../research/product/pdf-app-product-and-ux.md), which reached several of the same conclusions
  - the plans whose work Round T extends: [shareable libraries](shareable-libraries.md) (extraction, CLI and MCP), [PDF library](pdf-library.md) (generation), [web reader](web-app.md), [website](website.md) and [full editor](full-editor.md)
- **Todos:** Round T on the [todo board](../todos/README.md); §5 lists them.

## 1. Goal

Ten years of Hacker News show that PDF users ask for the same things they asked for in 2016, and that what they lack is trust. Free tools in browsers, the OS and a flood of in-browser apps now do the everyday jobs. The complaints left over describe broken promises: saves that damage files, black boxes that leave the text in the file, extraction that invents content, readers that get exploited, and libraries that turn out to be AGPL.

Lamina PDF already has much of what answers those complaints: a fuzzed, tolerant core, a save planner that keeps the original bytes, byte-identical output, a permissive licence, and PDF/UA and PDF/A profiles checked by veraPDF. Round T does two things with that:

- **Publish the proof.** The nightly already measures how Lamina PDF does on thousands of real files; users should see those numbers, the licence and where the code comes from.
- **Close the trust gaps that nothing on the board covers yet.** A check that finds failed redactions and hidden content in any file, extraction that says when it is unsure, flowing layout for generated documents, two reading features, and page and size commands in the CLI.

Round T adds no feature that sends a document anywhere, adds no AI to the reader, and promises nothing the tests don't show.

## 2. What the research found

The [report](../research/hacker-news/pdf-users-on-hacker-news.md) ranks ten recommendations. Each maps to work below:

| Recommendation | Already on the board | Round T adds |
| --- | --- | --- |
| [R1. Prove correctness in public](../research/hacker-news/pdf-users-on-hacker-news.md#r1-prove-correctness-in-public) | U5b reader benchmark | T1a, T1b |
| [R2. Real redaction, and a check for fake redaction](../research/hacker-news/pdf-users-on-hacker-news.md#r2-ship-real-redaction-and-catch-fake-redaction-in-any-file) | E12a–E12f | T2a, T2b; a notice in E4f |
| [R3. Forms and signatures, XFA named plainly](../research/hacker-news/pdf-users-on-hacker-news.md#r3-finish-forms-and-signatures-and-name-xfa-files-plainly) | E8, E10 | nothing: the evidence raises their priority |
| [R4. Extraction users can check](../research/hacker-news/pdf-users-on-hacker-news.md#r4-make-extraction-checkable) | L3g | T3a, T3b, T3c, T1c |
| [R5. Flowing layout, then Markdown and JSON](../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json) | none | T4a–T4j, T1c |
| [R6. A fast, quiet, safe reader](../research/hacker-news/pdf-users-on-hacker-news.md#r6-keep-the-reader-fast-quiet-and-safe) | C7c, U4a, U5a | nothing |
| [R7. Long reading](../research/hacker-news/pdf-users-on-hacker-news.md#r7-serve-long-reading-reflow-a-dark-page-papers-and-manuals) | U2c, U4b, U4c, C5b, C6a, C6b | T5a, T5b |
| [R8. Sanitise without flattening, report what a file reveals](../research/hacker-news/pdf-users-on-hacker-news.md#r8-sanitise-without-flattening-and-report-what-a-file-reveals) | E11d, E3g | T2c |
| [R9. Say what Lamina PDF already has](../research/hacker-news/pdf-users-on-hacker-news.md#r9-say-loudly-what-lamina-already-has) | none | T1b, T1d |
| [R10. Chained jobs in the CLI](../research/hacker-news/pdf-users-on-hacker-news.md#r10-grow-the-cli-and-mcp-server-for-chained-jobs) | E2, E14, E16 | T6a, T6b |

## 3. Rules for this round

1. **Every published number comes from a run anyone can repeat.** Figures on the website and docs come from the nightly, the benchmark or a committed script, with the date, commit and machine beside them. Nothing is typed in by hand.
2. **A check never claims more than it tests.** `laminapdf check` and `laminapdf reveal` say "likely" where they guess, document what they miss, and never call a file safe.
3. **Hidden content is reported by count and location, never by its text,** unless the caller asks for it; ADR 0026 already keeps document text out of messages.
4. **The reader stays quiet.** Previews and remembered positions follow ADR 0024's rules, store nothing off the device (ADR 0011), and add no prompts.
5. **New output fields are additive.** Changes to `laminapdf.extract/1`, `laminapdf.check/1` and the other committed schemas add optional fields; a breaking change needs a new schema version and an ADR amendment.

## 4. What Round T doesn't do

- **No AI in the reader.** AI reader launches had a median of 2 points; Acrobat's AI buttons drew anger.
- **No "local and private" campaign on its own.** It became the most crowded pitch on Show HN; it stays a guarantee (ADR 0011), not the headline.
- **No Word-style editing promise.** E7 keeps its small, honest scope, and [E7f](../todos/ready/E7f-font-handling-and-styling.md) now warns before substituting a font.
- **No rasterising to make files safe or small.** The sanitiser (E11d) and the optimiser (E16, T6b) keep text.

## 5. Todos

Each todo is one PR on the [todo board](../todos/README.md). The six groups are independent of each other, apart from the links listed; several sessions can work on them at once.

### T1. Publish the proof

The nightly already runs Lamina PDF against pdf.js, PDFium, qpdf and pypdf on the external corpus, fuzzes every parser entry point, renders against PDFium and checks every example with veraPDF. T1 turns those runs into figures users can read, and publishes the extraction and generation numbers and a guide that makes byte-identical output useful to users.

| Todo | What | Size | Depends on |
| --- | --- | --- | --- |
| [T1a](../todos/ready/T1a-nightly-proof-figures.md) | The nightly records the figures that show how the core copes with real files | L | — |
| [T1b](../todos/ready/T1b-how-we-test-page.md) | A "How we test" page on the website, with the nightly's figures, the licence and where the code comes from | M | T1a |
| [T1c](../todos/ready/T1c-publish-performance-numbers.md) | Publish extraction scores and speed, and generation timings | L | — |
| [T1d](../todos/done/T1d-snapshot-testing-guide.md) | A docs guide: snapshot-test your own generated PDFs | M | — |

### T2. Find what files hide

A black box over live text is the classic failed redaction, and incremental saves keep earlier revisions inside the file. Extraction already drops invisible, off-page and tiny text (ADR 0030 §4); T2 adds text covered by later shapes, then reports failed redactions in `laminapdf check` and everything a file discloses in a new `laminapdf reveal`, both also served by the MCP server.

| Todo | What | Size | Depends on |
| --- | --- | --- | --- |
| [T2a](../todos/done/T2a-covered-text-flag.md) | Extraction flags text hidden under later shapes, images and annotations (`covered`) | L | L3g |
| [T2b](../todos/ready/T2b-check-failed-redactions.md) | `laminapdf check` warns about failed redactions: covered text, unapplied redaction annotations, text left in earlier revisions | L | T2a |
| [T2c](../todos/ready/T2c-reveal-command.md) | `laminapdf reveal`: what a file discloses, in the CLI and MCP | L | T2b |

### T3. Extraction users can check

Users now ask whether they can check what an extractor returned. T3 gives each page a text-layer status so a pipeline can send only scanned pages to OCR, warns when a page's column order is a close call, and joins tables that continue across pages.

| Todo | What | Size | Depends on |
| --- | --- | --- | --- |
| [T3a](../todos/done/T3a-text-layer-status.md) | A per-page text-layer status that says when a page needs OCR | L | — |
| [T3b](../todos/ready/T3b-reading-order-confidence.md) | Warn when a page's column order is uncertain | L | — |
| [T3c](../todos/done/T3c-tables-across-pages.md) | Join tables that continue across pages | M | — |

### T4. Pages that flow

The library plan lists flowing layout as a non-goal for now and asks, in its open question 4, whether it should become one. The evidence says yes: control over page breaks is the generation request with the clearest gap. T4a decided how in [ADR 0032](../decisions/0032-flowing-layout-and-document-inputs.md), after the [`flow-layout` spike](../../spikes/flow-layout/README.md) weighed an engine of our own over the builder's `Surface` against embedding Typst: our own engine. T4b–T4j build it, each at most L: T4f and T4g lay the groundwork in the builder and the text layout, T4b, T4c and T4h make the flow layer, and T4d, T4i, T4e and T4j add Markdown, JSON documents and their surfaces.

| Todo | What | Size | Depends on |
| --- | --- | --- | --- |
| [T4a](../todos/done/T4a-adr-0032-layout-engine.md) | ADR 0032: a layout engine for flowing pages, with a spike | M | — |
| [T4f](../todos/ready/T4f-flow-builder-groundwork.md) | Builder groundwork: elements across pages, measuring, artifact subtypes, marks under PDF/UA-1 | L | T4a |
| [T4g](../todos/ready/T4g-flow-styled-spans.md) | Styled spans in one paragraph | M | T4a |
| [T4b](../todos/ready/T4b-flow-layout-core.md) | Flowing layout: paragraphs across pages, keep-together, running headers and footers, page X of Y | L | T4f, T4g |
| [T4c](../todos/ready/T4c-flow-tables-notes-contents.md) | Flowing layout: tables across pages with repeated headers and kept totals | L | T4b |
| [T4h](../todos/ready/T4h-flow-notes-contents-outline.md) | Flowing layout: footnotes, a contents page with page numbers, and the outline | L | T4b |
| [T4d](../todos/ready/T4d-markdown-documents.md) | Markdown in Rust, flowing pages out | L | T4c, T4h |
| [T4i](../todos/ready/T4i-build-command-and-bindings.md) | `laminapdf build`, its MCP tool and the TypeScript `markdown` call | L | T4d |
| [T4e](../todos/ready/T4e-json-documents-and-templates.md) | JSON documents and templates in Rust and the CLI | L | T4i |
| [T4j](../todos/ready/T4j-documents-in-typescript.md) | Typed documents and `builder.flow` in TypeScript | L | T4e, T4i |

### T5. Long reading

Readers of papers, manuals and textbooks want to see where a link or citation goes before they jump, and to come back to a document where they left it. Both build on C6b's links and history; T5a also needs U3a's touch gestures, and T5b C7b's local storage and settings.

| Todo | What | Size | Depends on |
| --- | --- | --- | --- |
| [T5a](../todos/ready/T5a-link-previews.md) | Preview internal links and citations before jumping | L | C6b, U3a |
| [T5b](../todos/ready/T5b-reopen-at-last-position.md) | Reopen each document where the reader left it | L | C6b, C7b |

### T6. Chained jobs in the CLI

Merge, split and the optimiser are planned in the library (E2, E16) but not in the `laminapdf` CLI. T6 adds the commands developers chain in scripts, under ADR 0031's grammar.

| Todo | What | Size | Depends on |
| --- | --- | --- | --- |
| [T6a](../todos/ready/T6a-cli-page-commands.md) | `laminapdf pages` merge, split, delete, extract, reorder and insert | L | L4e, E2d |
| [T6b](../todos/ready/T6b-cli-optimize-target-size.md) | `laminapdf optimize` with a target size | M | E16c, T6a |

## 6. Sequence

```
 T1a ─► T1b          T1c          T1d                          (start now)
 T2a ─► T2b ─► T2c                                             (T2a starts now; L3g is done)
 T3a    T3b    T3c                                             (start now)
 T4a ─► T4f, T4g ─► T4b ─► T4c, T4h ─► T4d ─► T4i ─► T4e ─► T4j  (T4a: ADR 0032)
 C6b, U3a ─► T5a;  C6b, C7b ─► T5b
 L4e, E2d ─► T6a ─► T6b ◄─ E16c
```

Start with T1a, T1d, T3a and T4a: they need nothing else and each answers a top finding. T1b waits on T1a's figures and on the owner's approval of its copy. T2a can start too, now that L3g, which edits the same extraction flags, is done. T5 and T6 follow the reader, CLI and editor rounds they extend.

## 7. Changes to existing todos and plans

This plan's PR also changes these files, so the evidence and the new todos reach work that is already on the board:

- **The board:** `tools/xtask/src/todos.rs` accepts the letter T, and the website's dashboard names the section (`apps/site/src/data/rounds.ts`).
- **Editor rules from the research:** [E4f](../todos/ready/E4f-annotation-tools-ui.md) shows a covered-text notice when a user draws an opaque shape over text, offering the redaction tool once it exists. [E7f](../todos/ready/E7f-font-handling-and-styling.md) never substitutes a font silently.
- **Todos that now build on Round T:**
  - [L3c](../todos/done/L3c-model-plugins.md)'s OCR plug-in reads pages with no visible text; T3a switches it to its `needs-ocr` pages; [E13c](../todos/ready/E13c-ocr-text-layer.md) checks that its layer reads as `ocr-layer`.
  - [E3g](../todos/ready/E3g-inspectors.md), [E10h](../todos/ready/E10h-revision-diff-and-attacks.md) and [E14b](../todos/ready/E14b-revision-browser.md) use T2c's revision index and T2b's `open_revision`.
  - [E11d](../todos/ready/E11d-sanitiser.md)'s sanitiser removes what `laminapdf reveal` lists, by its category names, and [E12f](../todos/ready/E12f-leak-tests.md)'s redacted fixtures must pass T2b's checks.
  - [E6d](../todos/ready/E6d-watermarks-headers-footers.md) uses T4b's page-number fields.
- **Out-of-scope pointers:** [C6b](../todos/ready/C6b-links-and-history.md) and the reader design's R07 point the link preview at T5a; [C7b](../todos/ready/C7b-offline-pwa.md) points the remembered position at T5b; E2b–E2d point the CLI's page commands at T6a; E16b and E16c point `optimize --target-size` at T6b.
- **Evidence:** [E8b](../todos/ready/E8b-form-filling-feature-plan.md), [E10b](../todos/ready/E10b-signatures-feature-plan.md), [E11a](../todos/ready/E11a-security-and-sanitisation-feature-plan.md), [E12a](../todos/ready/E12a-redaction-feature-plan.md), [U5a](../todos/ready/U5a-pricing-and-promises-page.md) and [U5b](../todos/ready/U5b-reader-benchmark.md) gain a Context line; U5a and U5b link T1b's and T1c's pages rather than repeating their numbers.
- **Plans:** the [library plan](pdf-library.md)'s open question 4 points to T4a; the [architecture plan §3](architecture.md#3-decisions-to-record-adrs) reserves ADR 0032 for it; the [website plan](website.md) lists `/how-we-test`; the [roadmap](roadmap.md), the [plans index](README.md), the [todo board's ID list](../todos/README.md) and CLAUDE.md name Round T.

## 8. Risks

- **Published figures can embarrass.** A low share of corpus files opened, or a slow benchmark, would be public. The answer is to publish anyway, with the method and the trend, and fix what the numbers show; hiding them would contradict the round's point. The owner approves the page's copy (T1b).
- **Covered-text detection can flag honest layouts.** Shading drawn over text, white boxes used for layout, and form widgets can look like a redaction box. T2a tests those cases and T2b words its warnings as "likely"; false positives cost trust too.
- **Round T competes for time with the launch rounds.** T1 and T3 are the parts worth doing early; T2a can start now that L3g is done, and T5 and T6 wait on the rounds they extend.
- **Parallel todos touch the same files.** T1c, T3a and T3b all edit the extraction benchmark (`crates/laminapdf/examples/differential/extract-bench.rs`, `tools/xtask/src/extract_bench.rs`); T2a and T3a both forward image events through `TextExtractor::page_with`. Each todo says so, and whichever lands second extends the first's code.
- **The WASM text module grows.** [ADR 0032 §9](../decisions/0032-flowing-layout-and-document-inputs.md#9-budgets) raises its budget from 1,200,000 to 1,280,000 B gzip for the flow layer, Markdown and documents together; the spike measured +19 KB for a paginator and +60 KB for the Markdown parser.

## 9. Open questions

The owner decides these; the todo named in each holds the details and a recommendation.

**Across the round**

1. **Tagged output by default.** Should `BuildOptions` tag documents unless asked not to, or at least flowing output (T4b)? Tagging changes the byte goldens and adds structure to every file. *[ADR 0032 §4](../decisions/0032-flowing-layout-and-document-inputs.md#4-tagging) decides it for flowing output; the fixed-position builder keeps its opt-in.*
2. **MCP tools that write.** ADR 0031 keeps the MCP server read-only unless started with `--allow-write`. T4i adds a `laminapdf_build` tool under that flag ([ADR 0032 §7](../decisions/0032-flowing-layout-and-document-inputs.md#7-surfaces) amends ADR 0031), and T6a proposes write tools; form filling would be next.
3. **Where the proof lives,** on `laminapdf.com` (T1b's proposal) or in the docs, and whether "How we test" joins the site header's links.

**Proof (T1)**

4. **Where the code comes from.** Many commits carry `Co-Authored-By: Claude` trailers. T1b's statement must describe the AI coding agents' part truthfully; only the owner can write or approve it.
5. **Which quiet machine** publishes the benchmark numbers (T1c), and whether a two-page invoice case is needed beside the one-page invoice and the 100-page report.
6. **Examples that import the render packages** (T1d's page-comparison example), or a separate example folder in `@laminapdf/render-hayro`.

**What files hide (T2)**

7. **Exit codes.** `laminapdf reveal --fail-on` exits 6 (T2c's recommendation) or reuses 3; `check`'s new findings exit 3 and are exempt from `--strict` (T2b), which ADR 0003 §2 may need to say.
8. **A new warning group and kind.** T2b adds `facadeWarnings` with kind `security`; a new kind for disclosures would amend ADR 0026.
9. **Document text in `reveal`.** It shows Info values, comments and form values in data fields, and `--include-hidden-text` on the command line only. Is that acceptable under ADR 0026's rule against document text in messages?

**Extraction (T3)**

10. **OCR layers.** ADR 0030 §4 drops invisible text, so a page made searchable by OCRmyPDF or Acrobat extracts as nothing. Should an opt-in keep the layer's text on `ocr-layer` pages (an ADR 0030 amendment)?
11. **Strict mode.** Should `extract-needs-ocr` fail strict extractions, as every other warning does?
12. **The reading-order corpus.** T3b adds hard layouts and resets the U2b score with a history entry; the alternative keeps them in a set the score ignores.

**Layout (T4)**

13. **ADR 0032's route:** an engine of our own, Typst, or a hybrid; and the template language for T4e (placeholders in the JSON, MiniJinja or Typst). *[ADR 0032](../decisions/0032-flowing-layout-and-document-inputs.md) decides: our own engine and placeholders in the JSON (accepted 2026-11-08).*
14. **The default font** when a caller gives none: the standard 14 fonts set Latin only and fail PDF/UA-1, while a bundled Noto face costs size (the library plan's open question 2). *[ADR 0032 §8](../decisions/0032-flowing-layout-and-document-inputs.md#8-fonts) decides the standard 14 and no bundled font.*

**Reader and CLI (T5, T6)**

15. **Remembered positions for `#url=` documents,** by content hash only (T5b's recommendation) or off by default like C7b's remembered URLs; and the cap of 500 documents.
16. **Signed files in the CLI.** T6a and T6b refuse them with no override flag; and "MB" in `--target-size` reads as decimal.

## 10. Later

Work the agents found while writing the todos, left out of Round T for now:

- More covers for T2a: strokes, shadings and rotated shapes; a flag for text in optional-content layers that are off.
- WASM and TypeScript bindings, and a reader panel, for `check`'s findings and `reveal`.
- `extract`'s MCP tool answering in JSON, so agents see T3a's `textLayer`; extraction thresholds as `doc.extract()` options; a per-page reading-order confidence key; a switch to turn off table joining.
- A home-page sample of Arabic, Devanagari and Chinese text (R9), beside the licence statement.

## 11. To-dos

- [x] Research what PDF users want on Hacker News, 2016–2026 ([report](../research/hacker-news/pdf-users-on-hacker-news.md), [notes](../research/hacker-news/README.md), [scripts](../../tools/hn-research/README.md))
- [x] Write the Round T todos and amend E4f, E7f and the feature-plan todos (§7)
- [ ] T1: publish the proof ([T1a](../todos/ready/T1a-nightly-proof-figures.md), [T1b](../todos/ready/T1b-how-we-test-page.md), [T1c](../todos/ready/T1c-publish-performance-numbers.md), [T1d](../todos/done/T1d-snapshot-testing-guide.md))
- [ ] T2: find what files hide ([T2a](../todos/done/T2a-covered-text-flag.md), [T2b](../todos/ready/T2b-check-failed-redactions.md), [T2c](../todos/ready/T2c-reveal-command.md))
- [ ] T3: extraction users can check ([T3a](../todos/done/T3a-text-layer-status.md), [T3b](../todos/ready/T3b-reading-order-confidence.md), [T3c](../todos/done/T3c-tables-across-pages.md))
- [ ] T4: pages that flow ([T4a](../todos/done/T4a-adr-0032-layout-engine.md), [T4f](../todos/ready/T4f-flow-builder-groundwork.md), [T4g](../todos/ready/T4g-flow-styled-spans.md), [T4b](../todos/ready/T4b-flow-layout-core.md), [T4c](../todos/ready/T4c-flow-tables-notes-contents.md), [T4h](../todos/ready/T4h-flow-notes-contents-outline.md), [T4d](../todos/ready/T4d-markdown-documents.md), [T4i](../todos/ready/T4i-build-command-and-bindings.md), [T4e](../todos/ready/T4e-json-documents-and-templates.md), [T4j](../todos/ready/T4j-documents-in-typescript.md))
- [ ] T5: long reading ([T5a](../todos/ready/T5a-link-previews.md), [T5b](../todos/ready/T5b-reopen-at-last-position.md))
- [ ] T6: chained jobs in the CLI ([T6a](../todos/ready/T6a-cli-page-commands.md), [T6b](../todos/ready/T6b-cli-optimize-target-size.md))
- [ ] The owner answers §9's open questions
