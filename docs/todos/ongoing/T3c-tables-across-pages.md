# T3c — Join tables that continue across pages

| Field | Value |
| --- | --- |
| ID | `T3c` |
| Status | `ongoing` |
| Plan | [trust and proof §T3](../../plans/trust-and-proof.md#t3-extraction-users-can-check) |
| Round / milestone | Round T (trust and proof) |
| Depends on | None |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) §1 (blocks) and [§7](../../decisions/0030-extraction-output-and-model-plugins.md#7-versioning-and-determinism) (new keys within `/1`; records an implementation note), [0031](../../decisions/0031-cli-and-mcp-contract.md) (`--max-chars`) |
| Branch | `feat/t3c-tables-across-pages` |
| PR title | `feat(extract): join tables that continue across a page break` |
| Size | M (≈ 300–450 lines: the linking pass, the Markdown, text and chunk changes, tests) |
| PR | — |

## Goal

When a table runs over a page break, link its two parts, write them as one table in Markdown and plain text, and keep them in one chunk. Today each part is a table of its own: the second loses its header or repeats it, and a chunk boundary can fall between them. Tables are the steadiest want in ten years of extraction comments (810 comments) and the top complaint (246). The named failures include "Merged cells, misplaced headers, and so forth." ([41107213](https://news.ycombinator.com/item?id=41107213)) and tables split over pages.

## Context

- [Report R4: make extraction checkable](../../research/hacker-news/pdf-users-on-hacker-news.md#r4-make-extraction-checkable) ("Third, tables that continue across pages") and the [extraction note](../../research/hacker-news/notes/extraction.md): want 1, complaint 1, §6 priority 3. Long tables are also where models give up: "they'd often insert \"...rest of table follows\"" ([42953492](https://news.ycombinator.com/item?id=42953492)).
- [ADR 0030](../../decisions/0030-extraction-output-and-model-plugins.md) §1 and L3b's implementation note: a table block has `rows`, `cols`, `detector` and `cells` (`row`, `col`, `rowSpan`, `colSpan`, `header`, `text`, `bbox`), and its `page` and `bbox` are one page's. §7 lets new keys appear within `/1`.
- [L3b](../done/L3b-extract-tables-and-serialisers.md) (tables, Markdown, chunks) and [L3d](../done/L3d-extract-structure-and-furniture.md) (the pass over every page):
  - `crates/laminapdf-extract/src/job.rs`'s `finish()` runs `furniture::remove`, `headings::assign` and `lists::assign`, then renumbers each page's block IDs.
  - `crates/laminapdf-extract/src/tables/`: `lattice.rs` (ruled grids, `SNAP` 3 pt), `stream.rs` (unruled, `TableMode::LatticeAndStream` only), `header_rows()` in `mod.rs` (a bold or shaded first row).
  - `crates/laminapdf-extract/src/output/markdown.rs`: `render()`, `block_markdown()`, `table()` (a pipe table, its first row as the header line) and `html_table()` (when a cell spans).
  - `crates/laminapdf-extract/src/output/mod.rs`: `Writer` and `Extraction::to_text()`.
  - `crates/laminapdf-extract/src/output/chunks.rs`: `split()` (boundaries at headings or pages, the budget, the overlap).
- `crates/laminapdf-extract/tests/tables.rs`: `a_table_split_across_a_page_break_is_two_tables` draws a whole table at the top of each of two pages, and its helpers (`pages()`, `grid()`, `text()`) draw ruled grids by hand.
- `crates/laminapdf-cli/src/commands/extract.rs`'s `truncate()` (`--max-chars`) keeps whole blocks.
- [L3g](../done/L3g-extract-structure-tree.md) adds tagged tables, whose structure tree already says where a table goes on.

## Scope

**In scope**

- **A linking pass**, `crates/laminapdf-extract/src/tables/across.rs` (new), called last in `finish()`, once furniture is gone and the IDs are final. Two table blocks are parts of one table when all of these hold:
  - they are on pages next to each other, both in the document (`index` and `index + 1`) and in the extraction's page order;
  - the first is its page's last block and the second its page's first, once running headers, footers and page numbers have left the blocks. One short block between them still allows a join when its text says the table goes on (`continued`, `cont.`, `cont'd`, in any case; English only): that block stays a paragraph;
  - the first reaches the bottom of its page (its bottom edge at or below 75% of the page's height) and the second starts near the top of its own (its top edge at or above 25%);
  - they have the same `detector` and the same `cols`, and their columns line up: for ruled tables, each column edge, measured from the table's left edge, within 3 pt (`lattice.rs`'s `SNAP`); for unruled ones, each column overlapping the same column of the other part and no other. A column with no cell starting in it gives no join;
  - the header fits: the second part's first row repeats the first part's header row (the same cell texts once white space is collapsed), or neither part has a header row, or only the first has one and the second's first row is different. A second part whose header differs from the first's starts a new table.
- **The link:** `Block::continues` (the ID of the part before) and `Block::continued_by` (the part after), `continues` and `continuedBy` in the JSON, absent when unset. A table over three or more pages is a chain. Each part keeps its own `page`, `bbox`, `rows` and `cells`, so every box still lies on one page. A repeated header row's cells get `header: true`, so a reader of the JSON can drop them when it joins the parts.
- **Markdown writes a chain as one table.** The first part writes the header line and the separator; each later part writes its rows without its header rows and without a separator, after a single line break instead of a blank line. When any part has a spanning cell, the chain is one HTML `<table>`, opened in the first part and closed in the last. Each part's `TextRange` covers its own rows. With page markers, a page whose first block continues a table gets its `<!-- page N -->` after that block, so the marker never cuts the table.
- **Plain text** (`to_text`) does the same: later parts' rows without their header rows, after a single line break.
- **Chunks** never start at a part that continues a table, whatever `SplitOn` says, and the overlap never starts one there. A chain over the budget is one chunk, as a block over the budget is; its `pages` and `regions` name both pages.
- **The renderers decide by the neighbour**, joining a part to the block before it only when that block is the part its `continues` names, so a cut list of blocks still renders valid Markdown. `--max-chars` in the CLI clears the last kept part's `continued_by` when the next part was cut.

**Out of scope** (and which todo covers it)

- Tagged tables over pages: the structure tree's `Table` element joins them, [L3g](../done/L3g-extract-structure-tree.md)'s work.
- A caption or footnote between the parts other than a "continued" line, a part that isn't its page's last or first block (a footnote under it, a heading above it), and tables side by side in columns.
- A table continued in the next column on the same page.
- Merging the parts into one block in the JSON: a block's box lies on one page (ADR 0027), so the parts stay blocks of their own.
- An option to turn joining off (Risks).

## Tasks

- [x] Claim via the work PR (see Definition of done); a fresh worktree off `main`. Load `/rust-best-practices` and follow [the Rust guidelines](../../guidelines/rust.md); no todo ID goes in a code comment (`cargo xtask check maintainability`'s `todo-id` rule).
- [ ] `model.rs`: `continues: Option<String>` and `continued_by: Option<String>` on `Block` (serde camel case, skipped when `None`; schemars); `page.rs`'s `text_block()` and `table_block()` set them to `None`.
- [ ] `tables/across.rs`: the conditions above as named constants (`BOTTOM` 0.75, `TOP` 0.25, `SNAP` reused from `lattice.rs`) and small functions (column edges from cells, header rows, the "continued" line), each unit-tested; `link(&mut Extraction)`, called at the end of `job.rs`'s `finish()`.
- [ ] `output/markdown.rs`: chains in `render()` (the separator between parts, where a page marker goes, pipe or HTML for the whole chain) and in the table writers (a part's rows without the header line, `<table>` and `</table>` only at the chain's ends).
- [ ] `output/mod.rs`: the same in `to_text()`. `output/chunks.rs`: no boundary, page or budget, at a continuing part, and no overlap that starts at one.
- [ ] `crates/laminapdf-cli/src/commands/extract.rs`: `truncate()` clears a dangling `continued_by`.
- [ ] Regenerate: `UPDATE_GOLDEN=1 cargo test -p laminapdf-extract --all-features --test output` (the schema; the corpus's Markdown goldens don't change, since no corpus table crosses a page), `UPDATE_GOLDEN=1 cargo test -p laminapdf --test extract` (`fixtures/extract/json/`, which shouldn't change: the new keys are absent there), `cargo xtask gen boundary` (`packages/sdk/src/generated/extract.ts`).
- [ ] Docs (below).

## Acceptance criteria

- [ ] A ruled table at the foot of page 1 whose bold header repeats at the top of page 2 gives two linked table blocks (`continuedBy` on the first, `continues` on the second, the repeated cells marked `header`), and its Markdown is one pipe table with the header line once:

  ```
  | Item | Price |
  | --- | --- |
  | Tea | 2.50 |
  | Cake | 4.00 |
  | Bread | 3.00 |
  | Jam | 1.50 |
  ```

- [ ] The same with no header on page 2, with a "Table 1 (continued)" line above the second part, and over three pages, each joins; `to_text()` writes one table each time.
- [ ] Each of these stays two tables, unlinked: columns 20 pt narrower on page 2; a different header on page 2; a paragraph under the first part on page 1; and the existing case of a whole table at the top of each page (the first doesn't reach the foot of its page).
- [ ] A chain with a spanning cell is one HTML `<table>` in the Markdown. With page markers, page 2's marker comes after the continued part. Every block's and chunk's `TextRange` still slices the Markdown to its text (`tests/output.rs`'s check, run on these files too).
- [ ] With `SplitOn::Pages`, and with a budget smaller than either part, the chain is in one chunk whose `pages` are `[0, 1]`.
- [ ] `laminapdf extract --format md --max-chars N` cut between the parts writes a closed table, and its JSON has no `continuedBy` that names a missing block.
- [ ] The committed schema has the two optional keys, and the JSON of every test file validates against it.

## Post-merge checks

None.

## Tests

- `crates/laminapdf-extract/tests/tables.rs`: the joins and non-joins above, drawn with its own helpers. `grid()` gains a parameter for whether the first row is bold, and the parts sit at the foot of page 1 (for example rules from y = 160 down to y = 60 in PDF user space, on its 600 × 800 pt pages) and the top of page 2. Rename `a_table_split_across_a_page_break_is_two_tables` to say what it now checks, two whole tables at the tops of their pages staying apart, and assert that neither part is linked.
- A three-page case with a page number in each page's foot, so `furniture::remove` takes the numbers out before the pass looks for each page's last block.
- Unit tests in `tables/across.rs` for each condition, and in `output/markdown.rs` and `output/chunks.rs` for the chain rendering, the page marker's place and the chunk boundaries.
- `crates/laminapdf-cli/tests/extract.rs`: the `--max-chars` criterion.
- The `extract` fuzz target keeps running over the corpus (`tests/regressions.rs`).

## Docs to update

- [ ] ADR 0030: an implementation note (as L3b's and L3d's) recording `continues` and `continuedBy`, the join conditions and their cut-offs, and that the renderings write one table and chunks keep a chain whole.
- [ ] `crates/laminapdf-extract/README.md`: a paragraph in its Tables section, and its Tests list.
- [ ] `apps/docs/src/content/docs/recipes/extract-markdown.mdx`: under "How it works", a table that goes on over a page break comes out as one table, and the JSON keeps one block per page, linked ([docs writing](../../guidelines/docs-writing.md)).
- [ ] `CLAUDE.md`'s "Extraction for retrieval" paragraph: `tables/across.rs` and the two keys.

## Risks and open questions

- **False joins.** Two different tables with the same columns, one ending a page and the next starting the following page, with no header on the second or the same header, are joined. The header and position rules make that rare; a report of one should become a test.
- **Missed joins.** A caption, a footnote or a heading between the parts stops the join, and only English "continued" lines are allowed through. Those are common in financial reports; a later change can widen the rule once real files show which forms matter.
- **`SplitOn::Pages` changes meaning a little:** a chain stays in one chunk that spans two pages. A pipeline that indexes page by page sees that chunk's two pages in its `pages`.
- **Cut outputs.** A caller that drops blocks from the JSON itself can leave a `continuedBy` that names a missing block. The docs say to treat such a link as the end of the table.
- **For the owner: an option to turn joining off?** The JSON keeps every part as it was on its page, so only the renderings change. A switch beside `DropOptions::furniture` can follow if someone needs page-faithful Markdown.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
