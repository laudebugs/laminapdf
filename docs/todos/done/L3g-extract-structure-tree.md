# L3g — Extraction: structure-tree order, tagged elements, clipped and low-contrast text

| Field | Value |
| --- | --- |
| ID | `L3g` |
| Status | `done` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction) |
| Round / milestone | Round L3 (shareable libraries): AI-ready extraction |
| Depends on | [L3d](./L3d-extract-structure-and-furniture.md) |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) §2–§4 (the rest of what its §9 lists as L3d's); [0023](../../decisions/0023-content-stream-reading.md) for the clip and fill colour |
| Branch | `feat/l3g-extract-structure-tree` |
| PR title | `feat(extract): structure-tree order, tagged headings, lists and tables, clipped and low-contrast text` |
| Size | L |
| PR | [#265](https://github.com/laudebugs/laminapdf/pull/265) |

## Goal

Add the rest of ADR 0030 §2–§4: reading order from a plausible structure tree, tagged headings, lists and tables that win over the layout rules, and the `clipped` and `low-contrast` flags. [L3d](./L3d-extract-structure-and-furniture.md), which this todo was split from, finds headings and lists from the layout and drops running headers, footers, page numbers and artifacts.

## Context

- [L3d's todo](./L3d-extract-structure-and-furniture.md) and ADR 0030 §2–§4, §9.
- ISO 32000-2 §14.7 (logical structure, `/K`, `/ParentTree`, `RoleMap`), §14.8.4 (standard structure types), §14.8.4.8.3 (tables), §8.5.4 (clipping), §8.6.8 (current colour).
- `laminapdf-model`'s `StructTree` view (B8a), the MCID each `laminapdf-text` run carries, and `laminapdf-content`'s graphics state (no clip tracked yet).

## Scope

**In scope**

- **Structure-tree order** (`structSource: "tagged"`): the plausibility checks and thresholds of ADR 0030 §2, `RoleMap`, `(page, MCID)` to runs, `structPath`, `structRef`, confidence 1. Untagged content outside artifacts goes after the tree's on its page, with `extract-untagged-content`; an implausible tree is ignored with `extract-struct-tree-ignored`. Each code gets its `codes.json` entry and page. `ReadingOrder::Tagged` forces the tree.
- **Tagged headings and lists:** `H`/`H1`–`H6` and `L`/`LI`/`Lbl`/`LBody` win over L3d's layout rules when the tree is used.
- **Tagged tables:** a `Table` element with `TR`/`TH`/`TD` (and `/RowSpan`, `/ColSpan`) gives a table block's rows, cells, spans and header flags, ahead of the ruled grid L3b detects; `detector: "tagged"` and confidence 1.
- **`clipped` and `low-contrast`:** `laminapdf-content` tracks the clip's bounding box, and `laminapdf-text`'s runs carry it and the colour the glyphs are painted in (small changes in those crates, in their own commits). The heuristics follow ADR 0030 §4 and lean towards keeping text.
- Tests on files built with `laminapdf-build` (`tests/tagged.rs`): a tree whose order differs from the layout's, one that covers too little, untagged text beside a tree, tagged lists and a tagged table; clipped and white-on-white text added to the hidden-text tests.

**Out of scope** (and which todo covers it)

- Layout headings and lists, running headers, footers, page numbers and artifacts: [L3d](./L3d-extract-structure-and-furniture.md).
- Model plug-ins: [L3c](../ready/L3c-model-plugins.md).

## Tasks

- [x] Claim via its work PR; a fresh worktree off `main`
- [x] The clip box in `laminapdf-content`, then the run fields in `laminapdf-text` (clip box, paint colour)
- [x] `order/tagged.rs` and the tagged tables in `laminapdf-extract`; the new warning codes
- [x] Fixtures and tests; the thresholds checked on the corpus; kept, recorded in the L3g note
- [x] README: what the rules handle now

## Acceptance criteria

- [x] A tagged fixture whose content order differs from its structure order comes out in structure order with `structSource: "tagged"`; the same file with a broken tree falls back to layout order with `extract-struct-tree-ignored`
- [x] Tagged headings, list items and a tagged table come out with the kinds, levels, depths and cells the tree gives
- [x] Clipped and white-on-white text is in no block and is in `dropped` as `clipped` and `low-contrast`

## Post-merge checks

None.

## Tests

Unit tests per module (`tagged.rs`'s checks, cell placement and kinds; `visibility.rs`'s flags; the clip box in `laminapdf-content`; colour and clip on runs in `laminapdf-text`); integration tests in `crates/laminapdf-extract/tests/tagged.rs` and `tests/extract.rs`; the `extract` fuzz target keeps running.

## Docs to update

- [x] ADR 0030 §9 and the L3g note (thresholds kept)
- [x] `laminapdf-extract`'s README

## Definition of done

- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ready/` → `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
