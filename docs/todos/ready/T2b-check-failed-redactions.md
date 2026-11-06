# T2b — `laminapdf check` warns about failed redactions

| Field | Value |
| --- | --- |
| ID | `T2b` |
| Status | `ready` |
| Plan | [trust and proof §T2](../../plans/trust-and-proof.md#t2-find-what-files-hide) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T2a](./T2a-covered-text-flag.md) |
| ADRs | Follows [0031](../../decisions/0031-cli-and-mcp-contract.md) §1 (the command reads through the façade), [§5](../../decisions/0031-cli-and-mcp-contract.md#5-exit-codes) (warnings exit 3), [§8](../../decisions/0031-cli-and-mcp-contract.md#8-errors-and-warnings) and [§9](../../decisions/0031-cli-and-mcp-contract.md#9-mcp-defaults-for-l4c); [0026](../../decisions/0026-error-and-diagnostic-contract.md) (new codes, each with a page; no document text in `message`, `hint` or `data`); [0030](../../decisions/0030-extraction-output-and-model-plugins.md#4-hidden-text-is-never-in-text) §4 (counts, never text); [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) F1, F5 and F10 (owned results, a `CancelToken`); [0003](../../decisions/0003-reader-tolerance-and-limits.md) §2 (these are findings, not repairs, so strict mode leaves them warnings). Adds an implementation note to ADR 0031 §8 |
| Branch | `feat/t2b-check-failed-redactions` |
| PR title | `feat(cli): laminapdf check warns about covered text, unapplied redactions and text left in earlier revisions` |
| Size | L (≈ 400–550 lines of code with unit tests; codes, error pages and fixtures don't count) |
| PR | — |

## Goal

Make `laminapdf check` find the three ways a redaction fails and leaves the text in the file: text painted over by a box (T2a's `covered` flag), redaction marks that were never applied, and an earlier revision that still holds what the latest one removed. Each becomes a warning with a code, so `check` exits 3, a script can branch on the code, and the MCP server's `laminapdf_check` reports it to an agent. Journalists can run it on released files, and publishers can run it before release.

Talk of redaction on Hacker News rose from 2.8 to 8.0 per 1,000 editor comments between 2020–22 and 2023–26, driven by the Epstein releases. X-ray, a library that finds bad redactions, drew 709 points ([46369923](https://news.ycombinator.com/item?id=46369923)). The talk moved from how to draw a box to what stays behind it, including earlier revisions: "the previous revision of the file is available inside of the PDF itself" ([46370872](https://news.ycombinator.com/item?id=46370872), on a `/Prev` pointer in an Epstein file). The report's gap for R2 is exactly this check, served by the CLI and the MCP server ([report R2](../../research/hacker-news/pdf-users-on-hacker-news.md#r2-ship-real-redaction-and-catch-fake-redaction-in-any-file)).

## Context

- [Report R2](../../research/hacker-news/pdf-users-on-hacker-news.md#r2-ship-real-redaction-and-catch-fake-redaction-in-any-file); the [editors note](../../research/hacker-news/notes/editors.md) (want 7, complaint 7 and recommendation 3: "a `laminapdf check` rule that flags boxes over live text and kept revisions") and the [format and security note](../../research/hacker-news/notes/format-and-security.md) (complaint 6; the forensics threads).
- [T2a](./T2a-covered-text-flag.md): the `covered` flag and `DropReason::Covered`, which this todo counts per page.
- Research: [04 §10](../../research/pdf/04-interactive-security.md#101-drawing-a-black-box-is-not-redaction) (a box is not redaction; Redact annotations express intent, §12.5.6.23, and only applying them removes anything), [01 §7](../../research/pdf/01-file-structure.md#7-incremental-updates-756) (incremental updates keep every older object) and [§8](../../research/pdf/01-file-structure.md#8-linearization-fast-web-view-annex-f) (a linearized file has two cross-reference sections and one revision).
- **What exists for revisions.** `laminapdf_cos::Document::revisions()` counts the sections reached through `/Prev` (1 for a file never updated; for a repaired file, the trailers its scan found). `xref_section_offsets()` lists those sections' offsets, newest first; it is empty for a repaired file. `whole_input()` returns the bytes. Nothing opens an earlier revision yet: [E14b](./E14b-revision-browser.md) plans `Document::open_revision(n)` and [E10h](./E10h-revision-diff-and-attacks.md) a revision index, both far down the editor rounds. **What is feasible now:** a revision's bytes are the file up to the `%%EOF` that ends its section, and `Document::open` reads that prefix like any file (01 §7). `fixtures/corpus/hello-incremental.pdf` is the case in miniature: its first revision draws `(Hello, World) Tj`, and the update replaces the content with `(Hello, Update) Tj`. Linearized files and repaired files need care (Scope).
- The code this changes:
  - `crates/laminapdf-cli/src/commands/check.rs`: `check()` opens the input, calls `read_every_object()`, `page_count()` and `page_texts(&TextOptions::default())` (which interprets every content stream and font), then lists the document's warnings. `CheckReport` (`laminapdf.check/1`) and `CheckStatus` (`ok`, `warnings`).
  - `crates/laminapdf-cli/src/commands/mod.rs`: `open()` builds the `OpenOptions` from `Settings`; `warnings()`. `src/main.rs`'s `check()` turns the report into an `Outcome`, whose warnings set exit 3 and print as `warning[code]` lines (ADR 0031 §8).
  - `crates/laminapdf-cli/src/mcp/tools.rs`: `laminapdf_check`'s fixed description and its schemas from `CheckArgs` and `CheckReport`. `src/mcp/mod.rs`'s `run_tool` calls `commands::check`.
  - `crates/laminapdf/src/warning.rs`: the façade's `Warning` enum (`Document`, `Font`), `warning_code()`, `diagnostic()`; `src/codes.rs` maps codes to the generated `WarningCode`; `src/error.rs` checks `facadeErrors` against `fixtures/codes.json` in a unit test.
  - `crates/laminapdf/src/extract.rs`: the façade's `extract`, `extract_with`, `extract_job`; `laminapdf-extract`'s `job.rs` and `page.rs` hold the page pass that flags runs.
  - `fixtures/codes.json` and `tools/xtask/src/codegen.rs`'s `GROUPS` (every group needs a title there); `docs/errors/<code>.md` pages and their headings, checked by `crates/laminapdf-cos/tests/codes.rs`.
  - `laminapdf_save::test_support::incremental_update(&base, &[(ObjRef, Object)], XrefEol)` appends a revision in tests; `laminapdf_test_fixtures::{classic, stream_body}` builds raw files.
- Earlier todos: [L4a](../done/L4a-lamina-cli.md) (`check`, exit codes), [L4b](../done/L4b-cli-batch-render-schema.md) (batch, schemas), [L4c](../done/L4c-mcp-mode.md) (the MCP tools), [L1b](../done/L1b-error-contract-codegen.md) (the code generator).

## Scope

**In scope**

- **Three warning codes** in a new `codes.json` group, `facadeWarnings`, each of kind `security`:

  | Code | When | `location` | `data` |
  | --- | --- | --- | --- |
  | `covered-text` | a page has text T2a flags `covered` | `pageIndex` | `chars` (characters covered), `regions` (dropped entries) |
  | `redaction-not-applied` | a page's `/Annots` holds `/Subtype /Redact` annotations | `pageIndex` | `annotations` (how many) |
  | `text-in-earlier-revision` | an earlier revision holds words that the latest revision holds nowhere | none | `revision` (its index, 0 for the original file), `words`, `pages` (the earlier revision's page indices where those words sit) |

  One warning per page or per revision, never per word. No warning carries text from the document.
- **The page pass alone.** `laminapdf_extract::hidden_text(doc, options, cancel)` (new) runs the page pass and ADR 0030 §4's flags, without lines, blocks, tables, order or furniture, and returns the `Dropped` entries of hidden runs (artifacts aren't hidden text). It interprets every content stream and font, so `check` reads as much as before.
- **Earlier revisions.**
  - `laminapdf_cos::Document::revision_ends() -> Result<Vec<u64>>` (new): each revision's byte length, oldest first; the last is the newest revision. A revision ends just past the first `%%EOF` line after its section and before the next newer section; one with no such `%%EOF` merges into the next.
  - A linearized file's first-page section (it follows the linearization dictionary, and its `/Prev` leads to a section later in the file; Annex F) is part of the first revision, not one of its own.
  - A repaired file has no chain to follow: one revision, and no comparison.
  - The façade adds `Document::revision_count() -> Result<u32>` and `Document::open_revision(index: u32, options: OpenOptions) -> Result<Document>`. The latter opens the prefix as a new document; past the last index it fails with `revision-index-out-of-range` (new, in `facadeErrors`). E14b's `open_revision` is this one.
- **The comparison.** Each earlier revision, newest first, is opened tolerantly (strict off, the caller's password and limits), and its warnings are dropped. Its page text (`page_texts`, invisible glyphs included) is split into words of at least two characters. Those words are counted against the latest revision's words, also counted over all its text, visible or not. A word the earlier revision has more often counts as removed. Text that the latest revision still holds in any form is never reported here; `covered-text` reports what it hides. The `CancelToken` is checked between pages and between revisions.
- **The façade call.** `Document::redaction_warnings(&self, options: &OpenOptions, cancel: &CancelToken) -> Result<Vec<Warning>>` (feature `extract`) returns the three kinds as a new `Warning::Redaction(RedactionWarning)` variant, with `code`, `kind`, `location`, `hint` and `diagnostic` like the others. `Document::hidden_text(&self, cancel) -> Result<Vec<Dropped>>` wraps the extract entry point. Both are Rust-only for now.
- **`check`.** It calls `redaction_warnings` in place of `page_texts` and lists the results after the reader's warnings. `status` becomes `warnings`, the exit code 3 (`--warnings-ok` makes it 0), and text mode prints `warning[covered-text]: …` lines. `--strict` still fails only on repairs: these warnings aren't repairs.
- **MCP.** `laminapdf_check` returns the same report: the warnings in `structuredContent.warnings`, `isError` false. Its fixed description says it also reports what failed redactions leave behind.
- **ADR 0031 §8 implementation note** (Tasks).

**Out of scope** (and which todo covers it)

- The full report of what a file discloses, the revision index with changed objects per revision, and a flag that fails CI on findings: [T2c](./T2c-reveal-command.md).
- Text in annotations, form values and metadata across revisions; `check` compares page text only.
- Applying redactions and dropping earlier revisions: [E12c](./E12c-apply-content-removal.md), [E12d](./E12d-scrub-and-rewrite.md). Running this check over E12's redacted fixtures: proposed for [E12f](./E12f-leak-tests.md).
- An editor warning when someone draws a filled box over text ("Every PDF editor ought to have a big flashing warning…", [38549034](https://news.ycombinator.com/item?id=38549034)): not planned yet; it belongs with [E6b](./E6b-place-images-and-shapes.md)'s shapes or [E12b](./E12b-mark-and-review.md)'s redact mode.
- WASM and TypeScript bindings of these calls, and a banner in the web reader: later, when a reader todo asks for them. ADR 0028 F10's job form comes with the bindings.

## Tasks

- [ ] Claim via the work PR; a fresh worktree off `main`; load `/rust-best-practices`, and `/writing` for the error pages and the README.
- [ ] **Codes.** Add the group to `fixtures/codes.json` after `facadeErrors`, with `since` set to the workspace version without its pre-release tag:
  ```json
  "facadeWarnings": [
    { "code": "covered-text", "kind": "security", "summary": "a page draws text and then paints a shape, an image or an annotation over it, so the text is hidden but still in the file", "hint": "remove the text itself with a tool that deletes what a redaction covers, then save a full rewrite", "since": "…" },
    { "code": "redaction-not-applied", "kind": "security", "summary": "a page has redaction marks that were never applied, so the text under them is still in the file", "hint": "apply the redactions in a tool that removes what they cover, then save a full rewrite", "since": "…" },
    { "code": "text-in-earlier-revision", "kind": "security", "summary": "an earlier revision of the file holds text that the latest revision no longer has", "hint": "save the file as a full rewrite, which keeps only the latest revision", "since": "…" }
  ]
  ```
  and `revision-index-out-of-range` (kind `usage`) to `facadeErrors`. Add ``("facadeWarnings", "Façade warnings (`laminapdf`)")`` to `GROUPS` in `tools/xtask/src/codegen.rs`, then run `cargo xtask gen codes`. Write the four `docs/errors/<code>.md` pages with the five headings and "For contributors" linking this todo; the warning pages' "Example" sections describe the black box, an unapplied Acrobat redaction mark and `hello-incremental.pdf`.
- [ ] **`laminapdf-extract`.** Add `hidden_text` in a new `src/hidden.rs`, sharing the per-page code with `ExtractJob::run` (one page pass, so the two can't drift). Export it from `src/lib.rs`; a line in the README.
- [ ] **`laminapdf-cos`.** Add `revision_ends` in a new `src/reader/revisions.rs`, with the rules in Scope. Unit tests: a file never updated, `hello-incremental.pdf`, the linearized fixture, a missing `%%EOF`, a repaired file, a hybrid `/XRefStm` file (`fixtures/corpus/hello-xrefstm.pdf`).
- [ ] **Façade** (`crates/laminapdf/src/redaction.rs`, new; feature `extract` except `revision_count` and `open_revision`). Add `revision_count`, `open_revision`, `hidden_text`, `redaction_warnings`, `RedactionWarning` and the `Warning::Redaction` variant. `covered-text`: group `hidden_text`'s `Covered` entries by page. `redaction-not-applied`: each page's `Page::annots()` resolved, counting `/Subtype /Redact`. `text-in-earlier-revision`: the comparison in Scope, in a `pub(crate)` function that returns `(revision, words, pages)` per revision, so [T2c](./T2c-reveal-command.md) reuses it. Unit-test `RedactionWarning::CODES` against `facadeWarnings`, as `src/error.rs` does for `facadeErrors`; extend `tests/data_fields.rs` with the three codes' keys.
- [ ] **CLI.** Split `open_options(settings) -> OpenOptions` out of `commands::open()`, and pass it to `redaction_warnings` in `check()`. Update the docs of `check()`, `CheckReport` and `CheckStatus::Warnings` ("…or found what a failed redaction leaves behind"), then regenerate `crates/laminapdf-cli/schemas/check.schema.json` (`UPDATE_GOLDEN=1 cargo test -p laminapdf-cli --test json_schemas`). The MCP schemas follow from the types.
- [ ] **MCP.** Change `laminapdf_check`'s description in `src/mcp/tools.rs` to: "Read a whole PDF (every object, page and content stream) and report what a tolerant read had to repair and what a failed redaction left in the file: covered text, unapplied redaction marks and text kept in earlier revisions."
- [ ] **ADR 0031 §8**, an implementation note:
  > **Implementation note (T2b).** `check` also reports what a failed redaction leaves behind: `covered-text`, `redaction-not-applied` and `text-in-earlier-revision` (`facadeWarnings` in `fixtures/codes.json`). They are findings about the content, not repairs, so `--strict` doesn't turn them into errors; like any warning they make the exit code 3, and `--warnings-ok` makes it 0. Their `data` holds counts and page indices, never the hidden text.
- [ ] **Fixture.** `fixtures/revisions/hello-linearized.pdf`: `qpdf --linearize fixtures/corpus/hello.pdf` with the qpdf pinned in `tools/xtask/src/doctor.rs`, listed with its provenance in `fixtures/README.md`. Build the other cases in the tests.
- [ ] **Docs.** The `check` bullet of `crates/laminapdf-cli/README.md` (what it now finds, and that `--warnings-ok` hides it, so a CI gate should read the codes or wait for T2c's `reveal --fail-on`); CLAUDE.md's CLI and façade paragraphs (`revision_count`, `open_revision`, `hidden_text`, `redaction_warnings`; the `facadeWarnings` group).

## Acceptance criteria

- [ ] `laminapdf check` on a built page that draws "Pay John Smith now." under a black box over the name exits 3. It prints `warning[covered-text]` on stderr, and with `--json` it lists `covered-text` with `location.pageIndex` 0 and `data.chars` 9. `--warnings-ok` exits 0, and `--strict` still exits 3.
- [ ] A page with two Redact annotations gives one `redaction-not-applied` with `data.annotations` 2; the same file after a test removes them gives none.
- [ ] `fixtures/corpus/hello-incremental.pdf` gives one `text-in-earlier-revision` with `data.revision` 0, `data.words` 1 (`World`) and `data.pages` `[0]`.
- [ ] `fixtures/revisions/hello-linearized.pdf` has `revision_count()` 1 and gives no warning. The same file with an appended update that replaces its page content (built with `incremental_update`) has `revision_count()` 2 and gives one `text-in-earlier-revision` for revision 0.
- [ ] No façade warning on `fixtures/corpus/hello.pdf`, `signed.pdf`, every file of `fixtures/signed/`, `fixtures/golden/incremental-set-info.pdf` or `fixtures/golden/cli-rotate-hello.pdf` (signatures, metadata and rotation add revisions but remove no text). A test over every file of `fixtures/corpus/` and `fixtures/golden/` asserts the warnings each gives, empty for all but the files it names.
- [ ] `open_revision(revision_count())` fails with `revision-index-out-of-range`; a token cancelled before the comparison stops `redaction_warnings` with `cancelled`.
- [ ] No message, hint or `data` of the new warnings holds document text: `crates/laminapdf-cli/tests/injection.rs` covers an injection string with a box, and keeps one only in an earlier revision; neither appears in `check --json`'s output.
- [ ] `laminapdf mcp`'s `laminapdf_check` on the covered fixture answers `isError: false` with `covered-text` in `structuredContent.warnings`, and `structuredContent` validates against the tool's `outputSchema` (`crates/laminapdf-cli/tests/mcp.rs`).
- [ ] `cargo xtask gen codes --check` passes; the four pages exist and `crates/laminapdf-cos/tests/codes.rs` passes; both copies of `error.schema.json` list the three warning codes; `crates/laminapdf/tests/error_contract.rs` passes.
- [ ] `check` still reads every content stream: the existing `crates/laminapdf-cli/tests/check.rs` passes unchanged, a broken font still warns `font-invalid`.

## Post-merge checks

None.

## Tests

- **Unit:** `revision_ends` in `laminapdf-cos` (the cases in Tasks); the word counting (multiplicity, two-character minimum, text the latest revision hides still counts as present); `RedactionWarning`'s codes, kinds and data against `codes.json`.
- **Integration:** `crates/laminapdf/tests/redaction.rs` (new) for the façade calls over built fixtures, `hello-incremental.pdf`, the linearized fixture and the corpus table; `crates/laminapdf-cli/tests/check.rs` (exit codes, JSON, text, `--strict`, `--warnings-ok`), `tests/injection.rs`, `tests/mcp.rs`, `tests/json_schemas.rs`, `tests/batch.rs` (a batch with one covered file reports `warnings` for it).
- **Fixtures:** build covered pages with `laminapdf-build`, as in T2a's tests; Redact annotations as raw objects with `laminapdf_test_fixtures::classic`; updated revisions with `laminapdf_save::test_support::incremental_update`. Commit only the linearized file.

## Docs to update

- [ ] `docs/errors/covered-text.md`, `redaction-not-applied.md`, `text-in-earlier-revision.md`, `revision-index-out-of-range.md`; `docs/errors/README.md` (regenerated)
- [ ] ADR 0031 §8 (the note above)
- [ ] `crates/laminapdf-cli/README.md`, `crates/laminapdf-extract/README.md`, `fixtures/README.md`, CLAUDE.md
- [ ] [Trust and proof §T2](../../plans/trust-and-proof.md#t2-find-what-files-hide): tick T2b

## Risks and open questions

- **Exit code 3 for findings.** ADR 0031 gives warnings exit 3, and this todo follows it. But 3 also means "repaired", and `--warnings-ok` turns both into 0, so a CI gate can't tell a fake redaction from a fixed cross-reference table by exit code alone. [T2c](./T2c-reveal-command.md)'s `reveal --fail-on` gives that gate its own code. The owner confirms that `check` stays on 3.
- **Strict mode.** ADR 0003 §2 says strict mode turns every warning into an error. These warnings come from the façade after the read, not from the reader, so strict mode doesn't touch them. The ADR 0031 note records that; if the owner wants ADR 0003 to say so too, add a line there.
- **Legitimate edits.** A revision that fixes a typo or renumbers pages removes words too, and `text-in-earlier-revision` reports it. The warning states a fact (the old text is still in the file), and its page says how to tell an edit from a leak.
- **Cost.** `check` now opens every earlier revision and reads its text. A signed file with many timestamp revisions costs one text pass per revision. The token stops it, and the MCP server's `--timeout` bounds a call. Measure on `fixtures/signed/` and say in the PR what it cost.
- **Odd revision chains.** Files whose sections don't end with `%%EOF`, linearized files updated many times, and repaired files are handled by the rules in Scope; a corpus file that breaks them gets a fixture.
- **Kind.** `security` is the closest of the nine kinds ("…or look tampered with"). If the owner wants a tenth kind for disclosures, that is an ADR 0026 amendment, not this PR.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
