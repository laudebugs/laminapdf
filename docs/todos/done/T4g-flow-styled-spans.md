# T4g — Styled spans in one paragraph: fonts, sizes, rises and languages mixed on a line

| Field | Value |
| --- | --- |
| ID | `T4g` |
| Status | `done` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4a](./T4a-adr-0032-layout-engine.md) |
| ADRs | Implements [ADR 0032 §3](../../decisions/0032-flowing-layout-and-document-inputs.md#3-the-layout-model) (paragraphs of inline spans); follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (HarfRust and ICU4X in `laminapdf-text`) |
| Branch | `feat/t4g-flow-styled-spans` |
| PR title | `feat(text): line breaking over styled spans in several fonts and sizes` |
| Size | M (≈ 350–500 lines of code with unit tests) |
| PR | [#323](https://github.com/laudebugs/laminapdf/pull/323) |

## Goal

Break a paragraph whose words come in different fonts, sizes and languages: a bold word inside a sentence, a superscript, an Arabic name inside an English line. Today `laminapdf-text`'s `break_lines` takes one size, and its font list is a fallback list, not a set of styles. Flowing paragraphs ([T4b](../done/T4b-flow-layout-core.md)), Markdown's emphasis and code spans ([T4d](../done/T4d-markdown-documents.md)) and footnote markers ([T4h](../done/T4h-flow-notes-contents-outline.md)) all need it. This was part of T4b until [T4a](../done/T4a-adr-0032-layout-engine.md) split T4b to keep each todo under 800 lines.

## Context

- `laminapdf-text`'s `break_lines` (`src/layout.rs`, 702 lines against the 800-line module budget) itemizes by bidi level and font coverage, shapes with HarfRust, breaks greedily at UAX #14 opportunities and returns `Line`s whose `LineRun`s may already use several fonts.
- `Surface::text_box` (`laminapdf-build`) draws one font and size; [T4f](../done/T4f-flow-builder-groundwork.md) adds measuring without drawing and a crate-private command that draws a measured layout.
- Shaping goldens in `fixtures/shaping/` (`tools/shaping-goldens/goldens.py`, `hb-shape`) and the UAX conformance subsets in `fixtures/unicode/`.

## Scope

**In scope**

- **Split `src/layout.rs`** into a `src/layout/` folder (the paragraph breaker and the line types in separate modules), so no module passes the 800-line budget.
- **Spans:** a span type (for example `Span { range, fonts, size, rise, language, features }`) and a breaking function over spans (for example `break_spans`). Items split at span boundaries, each item is shaped at its span's size and with its language, a line's ascent and descent come from its tallest span, and each `LineRun` records its span. A single span gives exactly the lines `break_lines` gives.
- **Rises** (superscript, subscript) as a baseline offset per span, carried into `pdf_run`'s `Ts`.
- **Every new item** re-exported from the crate root, as every item there is.

**Out of scope** (and which todo covers it)

- The flow model's `Paragraph` and `TextStyle`, and drawing spans through `Surface`: [T4b](../done/T4b-flow-layout-core.md).
- Justification and hyphenation: later, as [ADR 0032 §3](../../decisions/0032-flowing-layout-and-document-inputs.md#3-the-layout-model) records.

## Tasks

- [x] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`
- [x] Split `src/layout.rs` into `src/layout/` with no change in behaviour (the shaping goldens and layout tests pass unchanged)
- [x] The span type and `break_spans`; `break_lines` becomes one span over the whole text
- [x] Rises through `pdf_run`
- [x] The docs (Docs to update)

## Acceptance criteria

- [x] One span gives the same lines as `break_lines` for every input in `fixtures/shaping/` and the existing layout tests
- [x] A line holding 11 pt and 16 pt spans takes its height from the 16 pt span
- [x] A bold Latin word inside an Arabic paragraph keeps the paragraph's visual order (UAX #9), and extraction reads it back in logical order
- [x] A superscript span draws with a `Ts` rise and extracts as its text
- [x] `cargo xtask check maintainability` passes with no new size `#[expect]`

## Post-merge checks

None.

## Tests

- Unit tests in the new layout modules: one span, mixed sizes, mixed fonts in one line, a break at a span boundary, a span boundary inside a word (no break there), rises.
- A text golden page with mixed spans, written by `laminapdf-text`'s test page writer, with its pypdf, pdf.js and pdftotext facts (`tools/fixtures/text_facts.py`).

## Docs to update

- [x] `laminapdf-text`'s README (layout over spans)
- [x] `CLAUDE.md`'s Fonts API paragraph (`break_spans` beside `break_lines`)

## Risks and open questions

- **Line metrics with fallback fonts.** An Arabic run inside a Latin paragraph can make lines uneven. Take the tallest span's metrics, and test the case.
- **Kerning across span boundaries** is lost when items are shaped apart. Shape the whole run when two spans share a font and size, and split only where the style changes.

## As built

- **The split:** `src/layout/` holds `mod.rs` (the options, `break_lines`, `break_spans`, paragraphs), `breaker.rs` (the greedy breaker), `line.rs` (`Line`, `LineRun`), `span.rs` (`StyledSpan`, its resolved style and the groups of equal styles) and `tests.rs`. The largest is under 400 lines.
- **The span type is `StyledSpan`**, not `Span`: `laminapdf-extract` already has a public `Span`, and the maintainability check wants one home per type name. Each field but the range is optional and defaults to `LayoutOptions` (`fonts: None` is the whole list; a span's features come after the options'), so `StyledSpan::new(range)` alone lays out as `break_lines` does, and `break_lines` is `break_spans` with one span.
- **Itemizing:** `itemize` became `itemize_range` over one group's range and font list (crate-private), with the fonts' character maps read once per layout; `itemize` is the whole text with every font. A group that starts with only common characters (a span of punctuation) takes the script of the neighbouring group's item at its level, as it would inside one span.
- **Kerning across spans:** consecutive spans whose resolved styles are equal form one group, shaped as one run, so `LineRun::spans` is a range of span indices rather than one index: the spans with text on the run's line. An empty span of another style between two equal ones keeps them apart.
- **Line metrics:** `Line::ascent` and `Line::descent` take the highest ascent and lowest descent among the line's runs, each at its size and moved by its rise; a line without runs takes them from the first font of the span it starts in.
- **Rises:** `pdf_line_run(&LineRun, …)` is `pdf_run` with the rise, in thousandths of the run's size rounded to three decimals, added to every segment's.
- **A new code,** `invalid-spans`, for spans that don't cover the text once, in order, between grapheme clusters, and for a span with no fonts or a font index past the list.
- **Deviation, the golden:** `tests/spans.rs` draws the mixed-span page with the test page writer (`TestPdf::spans`) and reads it back with `TextExtractor`, but the page is not a `fixtures/golden/text-*.pdf` golden with pypdf, pdf.js and pdftotext facts. The facts script needs poppler 26.10.0 (pinned in `doctor.rs`), and this cloud session could not build it. Adding the page to `samples::GOLDENS` and running `tools/fixtures/text_facts.py` on a machine with the pinned tools is the follow-up.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
