# T6a — `laminapdf pages` merge, split, delete, extract, reorder and insert

| Field | Value |
| --- | --- |
| ID | `T6a` |
| Status | `ready` |
| Plan | [trust and proof §T6](../../plans/trust-and-proof.md#t6-chained-jobs-in-the-cli) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [L4e](./L4e-cli-render.md), [E2d](./E2d-merge-and-split.md) |
| ADRs | Follows and amends [0031](../../decisions/0031-cli-and-mcp-contract.md) (§2 grammar: the page subcommands; §6: new keys in `laminapdf.pages/1`; the batch amendment: write commands, `--output-dir`, split's template fields); follows [0007](../../decisions/0007-save-planner.md) (every save planned), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (errors), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) §6 (`index` and `number`), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (the façade only, `CancelToken`) |
| Branch | `feat/t6a-cli-page-commands` |
| PR title | `feat(cli): pages merge, split, delete, extract, reorder and insert` |
| Size | L (≈ 650 lines of code plus tests; split point under Risks) |
| PR | — |

## Goal

Developers already script page jobs with pdftk, qpdf and Ghostscript: 433 Hacker News comments name one of them, and 356 discuss merge, split, rotate, reorder and delete, the page jobs people expect for free. They also chain the jobs over folders: "First merge all files then depending on output size compress to fit the size" ([46675707](https://news.ycombinator.com/item?id=46675707)). The failures they report are specific, such as a merge that drops the links ([43888208](https://news.ycombinator.com/item?id=43888208)). This PR adds the page commands to `laminapdf` over the E2 ops, so links, outlines, form fields and tags survive every one: `pages delete`, `extract`, `reorder`, `insert`, `merge` and `split`. They use ADR 0031's grammar, run over many inputs or a folder, report in `laminapdf.pages/1`, and are served as MCP tools when the server is started with `--allow-write`.

## Context

- **Evidence:** the [editors note](../../research/hacker-news/notes/editors.md) (wants 5, scriptable command-line tools, and 6, page tools; §6, recommendations 1 and 6) and the report's [R10](../../research/hacker-news/pdf-users-on-hacker-news.md#r10-grow-the-cli-and-mcp-server-for-chained-jobs).
- **[ADR 0031](../../decisions/0031-cli-and-mcp-contract.md):** verb first, page operations as subcommands of `pages` (§2); qpdf's ranges with pdftk's handles and inline `file.pdf:1-3` (§3); `-`, `-o -`, `--force`, atomic writes (§4); exit codes (§5); `laminapdf.pages/1` and its compatibility rule (§6); MCP defaults (§9); the batch amendment (L4b). The [CLI spec note](../../research/libraries/notes/cli-spec.md) lists `pages merge/split/select/rotate/reorder/shuffle/delete` as the planned page algebra.
- **The model command:** `pages rotate` in `crates/laminapdf-cli/src/commands/pages.rs`: `RotateArgs`, `PagesReport`, `output_of` (`-o` or `--in-place` with `--force`), `open`, `pages_of`, `doc.apply(ops)`, `doc.save_with(&SaveOptions::default(), &settings.cancel)`, `io.check`/`io.write` under `--dry-run`. Its tests are in `crates/laminapdf-cli/tests/pages.rs`; its golden is `fixtures/golden/cli-rotate-hello.pdf`, from `crates/laminapdf-cli/examples/golden.rs`.
- **The range code:** `crates/laminapdf-cli/src/ranges.rs` (`PageRange::parse`/`resolve`, `inline_range`, `HandleRange`, `handle_definition`). **Batch:** `crates/laminapdf-cli/src/batch.rs` (`inputs`, the worker pool, the ordered emitter, the exit rule) and `src/many.rs` (`Many<A>`, `BatchArgs`). L4b left "batch over write commands" for the E2 page commands. **Templates:** [L4e](./L4e-cli-render.md) adds `-o 'out/{stem}-{n:03}.png'` (`{stem}`, `{name}`, `{index}`, `{n}`, a width) and the duplicate-output check before any write.
- **The ops**, all done once E2d is, since E2d depends on E2c and E2c on E2b: [E2b](./E2b-page-structure-ops.md) (`InsertBlankPages`, `DeletePages`, `MovePages`, `ReversePages`, reference cleanup with an `OpReport`, `PageLabels::remap`, the signed-widget check), [E2c](./E2c-cross-document-pages.md) (`InsertPagesFrom` with a `SourceDocId`, `ReplacePages`, `extract_pages`, `export_ranges`) and [E2d](./E2d-merge-and-split.md) (`merge(target, sources, options)` with outlines, names, AcroForm, struct trees and OCGs; `split(doc, SplitBy::Count | Ranges | Bookmarks { level })`; the merge report).
- **MCP:** `crates/laminapdf-cli/src/mcp/` (`tools.rs` lists `laminapdf_pages_rotate` only with `--allow-write`; `mod.rs` runs it through `roots::RootedIo`), from [L4c](../done/L4c-mcp-mode.md).
- **Not this todo:** recipes of several steps applied to many files are [E14d](./E14d-batch-runner.md); the organiser UI is [E2h](./E2h-page-organiser-ui.md).

## Scope

**In scope**

- **Six commands** in `crates/laminapdf-cli/src/commands/pages.rs` (split into `pages/` modules if it passes the 800-line budget). Each is one function `(args, &Settings, &mut dyn Io) -> Result<PagesReport, CliError>`, as `rotate` is. Pages are named by an inline range (`in.pdf:2-5`) or by `--pages RANGE` for every input without one; an inline range wins.

  | Command | Does | Over | Save |
  | --- | --- | --- | --- |
  | `pages delete <INPUT[:RANGE]>… [--pages R] (-o OUT \| --output-dir DIR \| --in-place)` | Removes the pages; E2b cleans up the outline items, links and named destinations that pointed at them | E2b `DeletePages` | Planned, incremental where allowed |
  | `pages extract <INPUT[:RANGE]>… [--pages R] (-o OUT \| --output-dir DIR)` | Writes a new document with those pages, in the order the range names them; a page named twice is an `invalid-page-range` | E2c `extract_pages` | A new document |
  | `pages reorder <INPUT> <ORDER> (-o OUT \| --in-place)` | Puts the pages in `ORDER`, a range that names every page once (`2,1,3-z`; `z-1` reverses) | E2b `MovePages`, or `ReversePages` for `z-1` | Planned, incremental where allowed |
  | `pages insert <INPUT> (--from FILE[:RANGE] \| --blank N [--size SIZE]) (--before PAGE \| --after PAGE) (-o OUT \| --in-place)` | Inserts pages from another file, or blank pages (`--size` takes E2b's page-size names or `WIDTHxHEIGHT` in points; by default the size of the page beside them) | E2c `InsertPagesFrom`, E2b `InsertBlankPages` | Planned, incremental where allowed |
  | `pages merge <INPUT[:RANGE]>… [--files-from FILE [-0]] [--outline-per-file] -o OUT` | Joins the documents in the order given, with their outlines, named destinations, fields and tags merged by E2d | E2d `merge`, into an empty document | A new document |
  | `pages split <INPUT>… (--every N \| --part RANGE… \| --bookmarks [LEVEL]) (-o TEMPLATE \| --output-dir DIR)` | Writes one document per part, each with its outline subset | E2d `split` | New documents |

  - `PAGE` in `--before`/`--after` is one page: a number, `z` or `r<n>`.
  - `merge` builds the output on an empty base, so it carries no earlier revision of its first input. An input with a range is merged as E2d's per-source page list where `merge` takes one; otherwise it is extracted first with E2c's `extract_pages`.
  - `--outline-per-file` maps E2d's option that puts each input's outline under one item titled with the input's title or file name.
  - `-` reads standard input once across a command's inputs; `-o -` works wherever there is one output.
- **Many inputs and folders.** `delete`, `extract` and `split` take several inputs and `--files-from FILE` (`-0`), and run on L4b's runner with `--jobs`, `--fail-fast`, NDJSON `result` lines and the summary; `merge` reads many inputs into one output. Two ways name the outputs:
  - `-o` with an L4e template (`-o 'trimmed/{name}'`);
  - `--output-dir DIR`, new, the short form of `-o 'DIR/{name}'`, or of `-o 'DIR/{stem}-{part:03}.pdf'` for `split`. It excludes `-o` and `--in-place`, and its folder is created by L4e's rule for a template's folders.
  The duplicate-output check runs over the whole batch before the first write, and an output that would replace an input needs `--force`, as `--in-place` does.
- **Split's template fields:** `{part}` (from 1), `{first}` and `{last}` (the part's first and last page numbers in the input), and, with `--bookmarks`, `{title}`. `{title}` is the outline item's title, which is document text, so it is made safe for a file name: path separators, control characters, bidi controls and `:*?"<>|` become `-`, leading dots are dropped, it is cut to 80 bytes at a character boundary, and an empty result becomes `part-{part}`.
- **`laminapdf.pages/1` grows; nothing breaks** (ADR 0031 §6: new keys within a version). New keys:
  - `command`: `rotate`, `delete`, `extract`, `reorder`, `insert`, `merge` or `split`;
  - `inputs`: `[{ input, pageCount }]`, every document read, in order;
  - `outputs`: `[{ output, saveMode, bytes, pageCount, pages: [{ index, number, from: { input, number } }] }]`, one per file written (a new document's `saveMode` is `fullRewrite`);
  - `removed`: `[{ index, number }]`, the pages `delete` took out;
  - `references`: what E2b's `OpReport` says it retargeted or removed (outline items, links, named destinations, the open action);
  - `merge`: E2d's merge report (renamed fields and destinations, dropped items, conformance claims lost).
  `input`, `output`, `saveMode` and `bytes` keep their meaning for the first input and the first output. `pages` keeps rotate's meaning ("the pages that turned") and is empty for the other commands. `rotate` fills the new keys too. The committed `crates/laminapdf-cli/schemas/pages.schema.json` is regenerated; `schema::COMMANDS` gains `pages-delete`, `pages-extract`, `pages-insert`, `pages-merge`, `pages-reorder` and `pages-split`.
- **Text output:** one line per output in rotate's style ("deleted 2 pages → out.pdf (incremental update)", "merged 3 files, 42 pages → all.pdf", "split into 3 files → chapters/").
- **Errors reuse existing codes.**
  - `invalid-page-range`: an order that misses or repeats a page (`data.missing` or `data.repeated`, plus the position), a range that would delete every page, a `PAGE` outside the document;
  - `invalid-command-line`: `--before` with `--after`, `--from` with `--blank`, `-o -` with several outputs;
  - `overwrite-refused`, `binary-output-to-terminal`, `would-invalidate-signatures`, and L4e's duplicate-output error.
  If E2b gives "every page deleted" a code of its own, map that instead. No new code is expected; if one is needed, it goes in `cliErrors` with its `docs/errors/<code>.md` page.
- **The façade only** (ADR 0031 §1). If E2c's and E2d's functions (`extract_pages`, `merge`, `split`, the source documents behind `InsertPagesFrom`) aren't on `laminapdf::Document` when this starts, add thin façade methods here with rustdoc, rather than reaching into `laminapdf-model`.
- **MCP** (ADR 0031 §9). With `--allow-write`, the server also lists `laminapdf_pages_delete`, `_extract`, `_insert`, `_merge`, `_reorder` and `_split`, in name order beside `laminapdf_pages_rotate`. The tools have:
  - `readOnlyHint: false`;
  - `destructiveHint: true` only for `delete`, `insert` and `reorder` when the server also has `--allow-in-place`;
  - static descriptions;
  - every input and output path, each of `split`'s outputs included, resolved through `RootedIo` and refused outside the roots (`path-outside-roots`).

**Out of scope** (and which todo covers it)

- `pages select` and `shuffle` (pdftk's `cat` and `shuffle`, qpdf's `--collate`: interleaving pages of several files by handle): later, if users ask; the handles in `ranges.rs` are ready for them.
- `pages rotate` over many inputs: its `<input> <angle>[:<range>]` grammar takes one input; a batch form waits for a request.
- `optimize` and `--target-size`: [T6b](./T6b-cli-optimize-target-size.md).
- Recipes of several steps per file: [E14d](./E14d-batch-runner.md). Server batches: [E14e](./E14e-server-batch.md).
- Release binaries and installers: [L4d](./L4d-cli-distribution.md).
- An override that invalidates signatures (`--invalidate-signatures`): not here; signed files are refused as `pages rotate` refuses them.

## Tasks

- [ ] Claim via the work PR (see Definition of done); a fresh worktree off `main`. Load `/rust-best-practices` and follow [docs/guidelines/rust.md](../../guidelines/rust.md).
- [ ] Read what E2b–E2d and L4e merged: the op names, `OpReport`, the merge report, `split`'s outline subsets, the template parser. Add any missing façade methods first, with their tests.
- [ ] Shared helpers in `commands/pages.rs`: page selection from an inline range or `--pages`, the output resolution (`-o`, a template, `--output-dir`, `--in-place`), and one save-and-write step that honours `--dry-run`. Move `rotate` onto them.
- [ ] The six commands with their args types (`clap::Args`, `Deserialize`, `JsonSchema`, `deny_unknown_fields`, as `RotateArgs`); `PagesCommand` in `main.rs`; the batch wiring for `delete`, `extract` and `split`.
- [ ] Split's template fields and the `{title}` rules, with unit tests (`../../etc/passwd`, a NUL, U+202E, an empty title, a title of 300 bytes).
- [ ] The report's new keys; regenerate the schemas (`UPDATE_GOLDEN=1 cargo test -p laminapdf-cli --test json_schemas`); `schema::COMMANDS`.
- [ ] The MCP tools in `mcp/tools.rs` and `mcp/mod.rs`, with `tests/mcp.rs` cases.
- [ ] Goldens from `crates/laminapdf-cli/examples/golden.rs`: `fixtures/golden/cli-merge-links-and-outlines.pdf` (two copies of `golden/links-and-outlines.pdf`) and `fixtures/golden/cli-split-chapters-p1.pdf` (the first part of E2d's 3-chapter fixture split by bookmarks), regenerated by `cargo xtask golden --update`.
- [ ] ADR 0031's amendment, the CLI README and CLAUDE.md (below).

### Example session

```console
$ laminapdf pages merge scans/*.pdf -o all.pdf
merged 12 files, 87 pages → all.pdf
$ laminapdf pages delete scans/*.pdf --pages 1 --output-dir trimmed --json | jq -c 'select(.type=="summary")'
{"elapsedMs":180,"failed":0,"ok":12,"skipped":0,"type":"summary","warnings":0}
$ laminapdf pages reorder report.pdf 3,1,2,4-z -o report-reordered.pdf
reordered 24 pages → report-reordered.pdf (incremental update)
$ laminapdf pages split book.pdf --bookmarks -o 'chapters/{part:02}-{title}.pdf'
split into 9 files → chapters/
$ laminapdf pages insert contract.pdf --from cover.pdf:1 --before 1 -o contract-with-cover.pdf
```

## Acceptance criteria

- [ ] **Links and outlines survive** (`crates/laminapdf-cli/tests/pages.rs`): merging two copies of `fixtures/golden/links-and-outlines.pdf` gives an output whose outline holds both inputs' items. Each item and link annotation with an internal destination targets the output page that holds the page it targeted in its input, through E2d's renamed named destinations where the names clashed. The test reads the output with `laminapdf-model`, a path-only dev-dependency. The output equals `fixtures/golden/cli-merge-links-and-outlines.pdf` byte for byte.
- [ ] **Delete:** deleting a page that an outline item targets follows E2b's rule and lists the item in `references`. The output is an incremental update: the input's bytes come first. Deleting every page exits 2 with `invalid-page-range` and writes nothing.
- [ ] **Reorder:** `z-1` reverses the pages and `2,1,3-z` swaps the first two; page labels and link targets follow the pages; the output is an incremental update. `1,1,3-z` exits 2 with `invalid-page-range` and `data.repeated` 1.
- [ ] **Extract and insert:** `extract in.pdf:3,1` gives a two-page document with page 3, then page 1. `insert --from cover.pdf:1 --before 1` puts the cover first and leaves the target's outline pointing at the same pages. `--blank 2 --after z` appends two pages of the last page's size.
- [ ] **Split:** `--bookmarks` on E2d's 3-chapter fixture writes 3 files, each with its chapter's outline subtree, the first equal to `fixtures/golden/cli-split-chapters-p1.pdf`. `--every 2` on a 5-page file writes 3 files (2, 2 and 1 pages). On `fixtures/golden/links-and-outlines.pdf`, whose top level holds an item with no destination and a web link, the parts follow E2d's rule for such items. Two parts that expand to the same path exit 2 before any file is written.
- [ ] **Folders:** `delete` over 3 inputs with `--output-dir` writes 3 files and prints NDJSON `result` lines in argument order, then the summary. With one unreadable input, it exits 1, writes the other 2, and lists the failure as a `result` line with an `error`.
- [ ] **Signatures:** `delete` and `reorder` on `fixtures/signed/certified-p2.pdf` exit 1 with `would-invalidate-signatures` and write nothing. `reorder` on `fixtures/signed/approval.pdf` writes an incremental update, and the signature still covers its original byte range.
- [ ] **The contract:** every command honours `--dry-run` (writes nothing, refuses what the real run would), `--force`, `-` and `-o -`, and a PDF never goes to a terminal; `--json` reports validate against the committed `pages.schema.json`; the existing `rotate` tests and golden pass unchanged.
- [ ] **MCP:** `tools/list` with `--allow-write` lists the seven page tools in name order, and none without it. A `split` whose template leaves the roots is a tool error with `path-outside-roots`, and nothing is written outside the roots.
- [ ] The goldens pass `qpdf --check` (recorded in the PR), and `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

- `crates/laminapdf-cli/tests/pages.rs` (each command against `fixtures/golden/links-and-outlines.pdf`, `golden/tagged-report.pdf`, `golden/examples/invoice.pdf`, `signed/approval.pdf`, `signed/certified-p2.pdf` and an encrypted fixture from `fixtures/encrypted/`), `tests/batch.rs` (write batches, `--output-dir`, the duplicate check), `tests/pipes.rs` (`pages merge … -o - | laminapdf pages delete - --pages z -o out.pdf`), `tests/json_schemas.rs`, `tests/mcp.rs`.
- Unit tests in `commands/pages.rs` (or its modules): output resolution, order validation, `{title}` sanitising, the report's keys.
- The goldens above, through `crates/laminapdf-cli/examples/golden.rs`.

## Docs to update

- ADR 0031: an amendment section (the page subcommands, `--output-dir`, split's template fields, the new `laminapdf.pages/1` keys, write commands in a batch).
- `crates/laminapdf-cli/README.md` (Commands, Many inputs, Page ranges: drop "for the page commands to come"); CLAUDE.md (the `laminapdf-cli` layout line and the CLI paragraph: the commands and their MCP tools).
- `fixtures/README.md` (the two new goldens).
- [Trust and proof §10](../../plans/trust-and-proof.md#10-to-dos): tick T6a.

## Risks and open questions

- **Size and the split point.** Six commands, the report and the MCP tools come to about 650 lines of code before tests. If review finds it too large, move `merge` and `split` (the commands with many inputs or many outputs, the template fields and their goldens) into a follow-up todo and land the single-document commands first.
- **Signed and encrypted inputs to `merge`, `extract` and `split`.** A new document can't carry an input's signatures, and E2c writes encrypted sources unencrypted. The report must say both, using E2c's and E2d's warnings; if they raise none, add warning codes (with their pages) rather than staying silent.
- **One password for every input.** `--password-file` and `LAMINA_PDF_PASSWORD` give one password; a merge of files with different passwords waits for a request (handles could name one per input).
- **Document text in file names.** `{title}` puts outline text into a path; the sanitising rules and their tests are the guard, and MCP's `RootedIo` still checks every path.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
