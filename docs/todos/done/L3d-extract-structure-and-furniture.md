# L3d — Extraction: headings, lists, running headers, footers and artifacts

| Field | Value |
| --- | --- |
| ID | `L3d` |
| Status | `done` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction) |
| Round / milestone | Round L3 (shareable libraries): AI-ready extraction |
| Depends on | [L3a](./L3a-lamina-extract-reading-order.md), [L3b](./L3b-extract-tables-and-serialisers.md) |
| Blocks | [L3g](../ready/L3g-extract-structure-tree.md) |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) §3–§4 (the layout half of what its §9 lists as L3d's) |
| Branch | `feat/l3d-extract-structure` |
| PR title | `feat(extract): headings, lists, running headers, footers, page numbers and artifacts` |
| Size | L |
| PR | [#255](https://github.com/laudebugs/laminapdf/pull/255) |

## Goal

Add ADR 0030 §3–§4's layout rules to `laminapdf-extract`: headings and list items from font sizes, bold and markers, and running headers, footers, page numbers and artifact-marked text moved to `dropped`. [L3a](../done/L3a-lamina-extract-reading-order.md) already reads blocks in layout order and drops invisible, off-page and tiny text.

> **Split (2026-11-05).** Structure-tree order and the clipped and low-contrast flags need work in three crates (the clip in `laminapdf-content`, run fields in `laminapdf-text`, the tree walk in `laminapdf-extract`), so the rest moved to [L3g](../ready/L3g-extract-structure-tree.md): reading order from the structure tree, tagged headings, lists and tables, and the `clipped` and `low-contrast` flags.

## Context

- [L3a's todo](../done/L3a-lamina-extract-reading-order.md) (its Scope, Acceptance criteria and Tests sections for these parts) and ADR 0030 §3–§4, §9.
- ISO 32000-2 §14.8.2.2 (artifacts, `/Type /Pagination`).

## Scope

**In scope**

- **Headings** from font-metric clusters: the body size covers the most characters; short blocks set larger, and bold at the body size, rank into up to six levels.
- **List items** from bullets and numbering with a hanging indent; depth from the indent. A line starting with a marker starts a block, and a lone marker joins the text up to four font sizes beside it.
- **Running headers, footers and page numbers** (ADR 0030 §4) and `Artifact` marked content (`artifact`): `laminapdf-text`'s runs carry whether they are in an artifact; `DropOptions::furniture` (`drop: { furniture }` in TypeScript) turns removal off.
- Tests on files built with `laminapdf-build`: headings and lists, and a five-page file with running headers, footers, page numbers and artifacts.

**Out of scope** (and which todo covers it)

- Structure-tree order, tagged headings, lists and tables, `clipped` and `low-contrast`: [L3g](../ready/L3g-extract-structure-tree.md).
- Model plug-ins: [L3c](../ready/L3c-model-plugins.md).

## Tasks

- [x] Claim via its work PR; a fresh worktree off `main`
- [x] The artifact field on `laminapdf-text`'s glyphs and runs
- [x] `headings.rs`, `lists.rs`, `furniture.rs` in `laminapdf-extract`; the option in Rust, WASM and TypeScript
- [x] Tests; the extraction benchmark run again
- [x] README: what the rules handle now

## Acceptance criteria

- [x] Headings, list items (with depth) and paragraphs match the expected kinds on the heading and list file
- [x] Running headers, footers and page numbers on a five-page file are absent from `blocks` and listed in `dropped` with their reasons; artifact-marked text is dropped as `artifact`
- [x] With `DropOptions::furniture` off they stay in the blocks

## Post-merge checks

None.

## Tests

Unit tests per module; integration tests on files built in `tests/structure.rs`; the `extract` fuzz target keeps running.

## Docs to update

- [x] ADR 0030 §9
- [x] `laminapdf-extract`'s README

## Definition of done

- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
