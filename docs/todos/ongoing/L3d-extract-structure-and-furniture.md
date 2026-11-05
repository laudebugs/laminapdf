# L3d — Extraction: structure-tree order, headings, lists, running headers and the last two hidden-text flags

| Field | Value |
| --- | --- |
| ID | `L3d` |
| Status | `ongoing` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction) |
| Round / milestone | Round L3 (shareable libraries): AI-ready extraction |
| Depends on | [L3a](../done/L3a-lamina-extract-reading-order.md), [L3b](../done/L3b-extract-tables-and-serialisers.md) |
| Blocks | None |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) §2–§4 (this todo implements what its §9 lists as L3d's); [0023](../../decisions/0023-content-stream-reading.md) for the clip and fill colour |
| Branch | `feat/l3d-extract-structure` |
| PR title | `feat(extract): structure-tree order, headings, lists, running headers, clipped and low-contrast text` |
| Size | L |
| PR | — |

## Goal

[L3a](../done/L3a-lamina-extract-reading-order.md) split its todo at the line its Risks section drew. `laminapdf-extract` now reads blocks in layout order and drops invisible, off-page and tiny text. This todo adds the rest of ADR 0030: reading order from a plausible structure tree, headings and list items, running headers, footers and page numbers moved to `dropped`, and the `clipped` and `low-contrast` flags.

## Context

- [L3a's todo](../done/L3a-lamina-extract-reading-order.md) (its Scope, Acceptance criteria and Tests sections for these parts) and ADR 0030 §2–§4, §9.
- ISO 32000-2 §14.7 (logical structure, `/K`, `/ParentTree`, `RoleMap`), §14.8.2.2 (artifacts, `/Type /Pagination`), §14.8.4 (standard structure types), §8.5.4 (clipping), §8.6.8 (current colour).
- `laminapdf-model`'s `StructTree` view (B8a) and `laminapdf-content`'s graphics state (no clip tracked yet).

## Scope

**In scope**

- **Structure-tree order** (`structSource: "tagged"`): the plausibility checks and thresholds of ADR 0030 §2, `RoleMap`, `(page, MCID)` to runs, `structPath`, `structRef`, confidence 1; `extract-struct-tree-ignored` and `extract-untagged-content`, each with its `codes.json` entry and page. `ReadingOrder::Tagged` to force it.
- **Headings** from font-metric clusters and tagged `H`/`H1`–`H6`; **list items** from bullets and numbering with a hanging indent, and tagged `L`/`LI`/`Lbl`/`LBody`.
- **Running headers, footers and page numbers** (ADR 0030 §4) and `Artifact` marked content (`artifact`): `laminapdf-text`'s runs carry whether they are in an artifact; `ExtractOptions::headers_footers` turns removal off.
- **`clipped` and `low-contrast`:** `laminapdf-content` tracks the clip's bounding box and `laminapdf-text`'s runs carry it and the fill colour (small changes in those crates, own commits); the heuristics as ADR 0030 §4 describes, leaning towards keeping text.
- **Tagged tables:** a `Table` element with `TR`/`TH`/`TD` (and `/RowSpan`, `/ColSpan`) gives a table block's rows, cells, spans and header flags, ahead of the ruled grid L3b detects; `detector: "tagged"` and confidence 1 (ISO 32000-2 §14.8.4.8.3).
- Fixtures built with `laminapdf-build`: `headings-lists`, `tagged-table`, `running-headers` (5 pages), `tagged-reordered`, `tagged-broken`, and clipped and white-on-white text added to the hidden-text test; the reading-order corpus's header-footer files then drop their headers and footers.

**Out of scope** (and which todo covers it)

- Ruled and unruled tables, serialisers, the JSON Schema and the Rust façade: [L3b](../done/L3b-extract-tables-and-serialisers.md); WASM, TypeScript and the benchmark: [L3e](../done/L3e-extract-bindings-and-benchmark.md).
- Model plug-ins: [L3c](../ready/L3c-model-plugins.md).

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`
- [ ] The run fields in `laminapdf-text` (artifact, clip box, fill colour), then the clip box in `laminapdf-content`
- [ ] `order/tagged.rs`, `headings.rs`, `lists.rs`, `furniture.rs` in `laminapdf-extract`; the new warning codes
- [ ] Fixtures and tests; the thresholds tuned on the corpus and recorded in ADR 0030 §2
- [ ] README: what the rules handle now

## Acceptance criteria

- [ ] A tagged fixture whose content order differs from its structure order comes out in structure order with `structSource: "tagged"`; the same file with a broken tree falls back to layout order with `extract-struct-tree-ignored`
- [ ] Headings, list items (with depth) and paragraphs match the expected kinds on the heading and list fixture
- [ ] Running headers, footers and page numbers on a 5-page fixture are absent from `blocks` and listed in `dropped` with their reasons; artifact-marked headers are dropped as `artifact`
- [ ] Clipped and white-on-white text is in no block and is in `dropped` as `clipped` and `low-contrast`

## Post-merge checks

None.

## Tests

Unit tests per module; integration tests on the fixtures above; the `extract` fuzz target keeps running.

## Docs to update

- [ ] ADR 0030 §2 (tuned thresholds) and §9
- [ ] `laminapdf-extract`'s README

## Definition of done

- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
