# L3g — Extraction: structure-tree order, tagged elements, clipped and low-contrast text

| Field | Value |
| --- | --- |
| ID | `L3g` |
| Status | `ongoing` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction) |
| Round / milestone | Round L3 (shareable libraries): AI-ready extraction |
| Depends on | [L3d](../done/L3d-extract-structure-and-furniture.md) |
| Blocks | None |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) §2–§4 (the rest of what its §9 lists as L3d's); [0023](../../decisions/0023-content-stream-reading.md) for the clip and fill colour |
| Branch | `feat/l3g-extract-structure-tree` |
| PR title | `feat(extract): structure-tree order, tagged headings, lists and tables, clipped and low-contrast text` |
| Size | L |
| PR | — |

## Goal

Add the rest of ADR 0030 §2–§4: reading order from a plausible structure tree, tagged headings, lists and tables that win over the layout rules, and the `clipped` and `low-contrast` flags. [L3d](../done/L3d-extract-structure-and-furniture.md), which this todo was split from, finds headings and lists from the layout and drops running headers, footers, page numbers and artifacts.

## Context

- [L3d's todo](../done/L3d-extract-structure-and-furniture.md) and ADR 0030 §2–§4, §9.
- ISO 32000-2 §14.7 (logical structure, `/K`, `/ParentTree`, `RoleMap`), §14.8.4 (standard structure types), §14.8.4.8.3 (tables), §8.5.4 (clipping), §8.6.8 (current colour).
- `laminapdf-model`'s `StructTree` view (B8a), the MCID each `laminapdf-text` run carries, and `laminapdf-content`'s graphics state (no clip tracked yet).

## Scope

**In scope**

- **Structure-tree order** (`structSource: "tagged"`): the plausibility checks and thresholds of ADR 0030 §2, `RoleMap`, `(page, MCID)` to runs, `structPath`, `structRef`, confidence 1. Untagged content outside artifacts goes after the tree's on its page, with `extract-untagged-content`; an implausible tree is ignored with `extract-struct-tree-ignored`. Each code gets its `codes.json` entry and page. `ReadingOrder::Tagged` forces the tree.
- **Tagged headings and lists:** `H`/`H1`–`H6` and `L`/`LI`/`Lbl`/`LBody` win over L3d's layout rules when the tree is used.
- **Tagged tables:** a `Table` element with `TR`/`TH`/`TD` (and `/RowSpan`, `/ColSpan`) gives a table block's rows, cells, spans and header flags, ahead of the ruled grid L3b detects; `detector: "tagged"` and confidence 1.
- **`clipped` and `low-contrast`:** `laminapdf-content` tracks the clip's bounding box, and `laminapdf-text`'s runs carry it and the fill colour (small changes in those crates, in their own commits). The heuristics follow ADR 0030 §4 and lean towards keeping text.
- Fixtures built with `laminapdf-build`: `tagged-reordered`, `tagged-broken`, `tagged-table`, and clipped and white-on-white text added to the hidden-text test.

**Out of scope** (and which todo covers it)

- Layout headings and lists, running headers, footers, page numbers and artifacts: [L3d](../done/L3d-extract-structure-and-furniture.md).
- Model plug-ins: [L3c](../ready/L3c-model-plugins.md).

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`
- [ ] The clip box in `laminapdf-content`, then the run fields in `laminapdf-text` (clip box, fill colour)
- [ ] `order/tagged.rs` and the tagged tables in `laminapdf-extract`; the new warning codes
- [ ] Fixtures and tests; the thresholds tuned on the corpus and recorded in ADR 0030 §2
- [ ] README: what the rules handle now

## Acceptance criteria

- [ ] A tagged fixture whose content order differs from its structure order comes out in structure order with `structSource: "tagged"`; the same file with a broken tree falls back to layout order with `extract-struct-tree-ignored`
- [ ] Tagged headings, list items and a tagged table come out with the kinds, levels, depths and cells the tree gives
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
- [ ] This file moved `ready/` → `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
