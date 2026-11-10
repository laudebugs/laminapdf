# T2c — `laminapdf reveal`: what a file discloses, in the CLI and MCP

| Field | Value |
| --- | --- |
| ID | `T2c` |
| Status | `done` |
| Plan | [trust and proof §T2](../../plans/trust-and-proof.md#t2-find-what-files-hide) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T2b](./T2b-check-failed-redactions.md) |
| ADRs | Amends [0031](../../decisions/0031-cli-and-mcp-contract.md) [§5](../../decisions/0031-cli-and-mcp-contract.md#5-exit-codes) (exit code 6, only from `--fail-on`), [§6](../../decisions/0031-cli-and-mcp-contract.md#6-json-output) (`laminapdf.reveal/1` joins the schema list) and its batch amendment (a `found` count). Follows 0031 §1, §2, §8 and [§9](../../decisions/0031-cli-and-mcp-contract.md#9-mcp-defaults-for-l4c) (a read-only tool); [0026](../../decisions/0026-error-and-diagnostic-contract.md) (document text only in data fields); [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) §6 (pages carry `index` and `number`); [0030](../../decisions/0030-extraction-output-and-model-plugins.md#4-hidden-text-is-never-in-text) §4 (hidden text counted, shown only on request); [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (owned records, a `CancelToken`); [0003](../../decisions/0003-reader-tolerance-and-limits.md) (every object read within `Limits`) |
| Branch | `feat/t2c-reveal-command` |
| PR title | `feat(cli): laminapdf reveal and laminapdf_reveal list what a file discloses beyond its pages` |
| Size | L (≈ 650–800 lines of code with unit tests; schemas and fixtures don't count). If it passes 800, ship history, metadata, attachments, hidden content, the command, the tool and exit code 6 first, and move active content, annotations and forms into a second PR |
| PR | [#316](https://github.com/laudebugs/laminapdf/pull/316) |

## Goal

A new read-only command, `laminapdf reveal`, and its MCP tool, `laminapdf_reveal`, list what a PDF holds beyond its visible pages. That covers earlier revisions and what changed in each, Info and XMP metadata, attachments, JavaScript and actions, annotations and their comments, form values, layers that are off, and hidden text counted by flag. The report is versioned JSON (`laminapdf.reveal/1`) with a committed schema, or text a person can read. With `--fail-on`, the command exits 6 when the file holds anything in the named categories, so a publisher can stop a release in CI. The categories are the ones [E11d](../ready/E11d-sanitiser.md)'s sanitiser removes, so whatever `reveal` lists, the sanitiser can take out.

On Hacker News, the forensics and leak threads (Manafort, the Snowden and Epstein PDFs, Elsevier's per-download identifier, X-ray and others) hold 1,414 comments. A study of the Snowden PDFs' metadata versions drew 324 points ([46564762](https://news.ycombinator.com/item?id=46564762)), and Elsevier's hash, unique to each download, drew 343 ([30082138](https://news.ycombinator.com/item?id=30082138)). Users ask for the tool directly: "There needs to be better tooling for inspecting PDF documents" ([46565528](https://news.ycombinator.com/item?id=46565528)). The report's gap for R8 is "a 'what this file reveals' report in `laminapdf check` and the read-only MCP tools" ([report R8](../../research/hacker-news/pdf-users-on-hacker-news.md#r8-sanitise-without-flattening-and-report-what-a-file-reveals)).

## Context

- [Report R8](../../research/hacker-news/pdf-users-on-hacker-news.md#r8-sanitise-without-flattening-and-report-what-a-file-reveals) and [R2](../../research/hacker-news/pdf-users-on-hacker-news.md#r2-ship-real-redaction-and-catch-fake-redaction-in-any-file); the [format and security note](../../research/hacker-news/notes/format-and-security.md) (§3's forensics finds, the smaller complaints about hidden data and tracking, and its recommendation 3, "Ship a 'what does this file reveal?' report").
- [T2b](../done/T2b-check-failed-redactions.md), which this builds on: `laminapdf_cos::Document::revision_ends`, the façade's `revision_count`, `open_revision`, `hidden_text` and `redaction_warnings`, and the `pub(crate)` comparison that counts the words each earlier revision holds and the latest doesn't. [T2a](../done/T2a-covered-text-flag.md)'s `covered` flag.
- [ADR 0031](../../decisions/0031-cli-and-mcp-contract.md): one function per command over an `Io`, verb first, `--json` with a `schema` id, a committed schema per report, exit codes and their priority (§5), the batch amendment (NDJSON lines, `BatchSummary`), and the MCP defaults (§9). Adding a command needs no amendment (§1 and "Consequences" plan for new commands and new schema ids), but **a new exit code does**: §5's table and priority are the contract.
- Research: [01 §7](../../research/pdf/01-file-structure.md#7-incremental-updates-756) and [§8](../../research/pdf/01-file-structure.md#8-linearization-fast-web-view-annex-f); [04 §1.3](../../research/pdf/04-interactive-security.md#13-actions-126) (actions), [§4](../../research/pdf/04-interactive-security.md#4-javascript-in-pdf) (JavaScript), [§5](../../research/pdf/04-interactive-security.md#5-metadata) (Info, XMP, IDs), [§6](../../research/pdf/04-interactive-security.md#6-embedded-files-associated-files-portfolios) (embedded and associated files), [§3](../../research/pdf/04-interactive-security.md#3-forms) (fields, XFA) and [§10.2](../../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do) (its table lists every layer a leak hides in); [02 §11](../../research/pdf/02-imaging-model.md#11-optional-content-layers). ISO 32000-2 §7.5.6, Annex F, §14.3 (metadata), §14.5 (`/PieceInfo`), §12.3.4 (thumbnails), §7.11.4 and §14.13 (embedded and associated files), §12.6.3–12.6.4 (triggers and actions), §12.7 (forms), §8.11 (optional content), §12.5.6.23 (Redact annotations).
- The code this builds on:
  - `crates/laminapdf-cli/src/commands/` (one module per command, re-exported in `mod.rs`), `src/main.rs` (`enum Command`, `each()` for commands over many inputs, `execute()`), `src/many.rs` (`Many<A>`), `src/batch.rs` (`Status`, `BatchSummary::exit_code`), `src/exit.rs` (`ExitCode`, `ALL`, `priority`, `meaning`), `src/render.rs` (text output, `keep_fields` for `--fields`), `src/schema.rs` (`all()`, `COMMANDS`, `command()`), `src/mcp/tools.rs` (`tools()`, `text_only()`, which strips arguments a tool doesn't take) and `src/mcp/mod.rs` (`run_tool`, whose last arm is the write tool).
  - `laminapdf_cos::Document`: `object_numbers()` and `object(n)` (as `read_every_object` uses them), `trailer()`, `catalog()`, `revisions()`, `xref_section_offsets()`, `was_repaired()`. `src/reader/xref.rs` reads each section of the `/Prev` chain (`table.sections`, `table.section_offsets`) and merges their entries, newest first, without keeping which section listed which object.
  - `laminapdf-model`: `Document::info()` (`Info::text(key)`, no key listing yet), `name_tree(key)` (`EmbeddedFiles`, `JavaScript`), `pages()` and `Page::annots()`, and `ActionKind` (`src/outline.rs`: `GoTo`, `GoToR`, `GoToE`, `Launch`, `Thread`, `Uri`, `Named`, `JavaScript`, `SubmitForm`, `ResetForm`, `ImportData`, `Other`), today read only for outline items.
- Typed models planned for the same objects, each far down the editor rounds: XMP ([E1b](../done/E1b-xmp-packet-model.md)), attachments ([E3f](../ready/E3f-attachments-and-associated-files.md)), the object and revision inspectors ([E3g](../ready/E3g-inspectors.md)), annotations ([E4b](../ready/E4b-annotation-model-and-overlay.md)), form fields ([E8c](../ready/E8c-acroform-field-model.md)), the revision diff ([E10h](../ready/E10h-revision-diff-and-attacks.md)), the revision browser ([E14b](../ready/E14b-revision-browser.md)) and layers ([E17c](../ready/E17c-layers.md)). This todo reads the objects directly, in one scan that those models can take over later without changing the report.
- Earlier todos: [L4a](../done/L4a-lamina-cli.md), [L4b](../done/L4b-cli-batch-render-schema.md) (batch, `schema`, `--fields`), [L4c](../done/L4c-mcp-mode.md) (the server, truncation, the injection tests).

## Scope

**In scope**

- **The command.** `laminapdf reveal <inputs…> [--fail-on CATEGORY,…|all] [--fields a,b] [--include-hidden-text]`, over one input or a batch like `info` and `check`. It reads every object of the latest revision within the `Limits`, as `check` does, so it also finds objects nothing references any more.
- **Seven categories**, which [E11d](../ready/E11d-sanitiser.md)'s `Sanitise { categories }` takes as its own (all but `forms`, whose values [E8g](../ready/E8g-form-data-and-flatten.md) clears or flattens):

  | Category | What the report lists | Counts as found when |
  | --- | --- | --- |
  | `history` | each revision, oldest first: its byte length, the objects its section adds, changes and frees, the pages whose objects it rewrote, and the words it holds that the latest revision doesn't (T2b's comparison); whether the file is linearized or was repaired; objects nothing references | there is an earlier revision or an unreferenced object |
  | `metadata` | every Info entry (custom keys included) with its value; the XMP packet's size and the namespaces it declares; object-level `/Metadata` streams, `/PieceInfo` dictionaries and `/Thumb` images (counts); the trailer's `/ID` | any of these but `/ID` |
  | `attachments` | each embedded file: name, size, MIME type, object, and what holds it (`/EmbeddedFiles`, an `/AF` array, a `FileAttachment` annotation) | any |
  | `active-content` | JavaScript (scripts, bytes, where); every action, once, with its kind (`ActionKind`), the trigger that fires it (`/OpenAction`, an `/AA` key) and its owner (document, page, annotation, field); XFA | any script, any `Launch`, `SubmitForm` or `ImportData`, any action a trigger fires, or XFA. Plain links (`URI`, `GoToR`, `GoToE`) are listed but don't count |
  | `annotations` | annotations by subtype; for each markup annotation, its page, author (`/T`) and comment (`/Contents`) | any annotation but links, widgets and popups |
  | `forms` | each field: full name, type, value, and whether its widgets are hidden | any field with a value |
  | `hidden-content` | hidden text by flag (T2a's six; characters and pages); layers that are off by default (their names); unapplied Redact annotations | any |

- **`laminapdf.reveal/1`.** A `RevealReport` with `schema`, `input`, `found` (the categories with findings, in the table's order), one key per category (`history`, `metadata`, `attachments`, `activeContent`, `annotations`, `forms`, `hiddenContent`) and `warnings` (the reader's, ADR 0026). Keys are camel case, values kebab case, and pages are `{ "index", "number" }`. The schema is committed as `crates/laminapdf-cli/schemas/reveal.schema.json`. Values from the document (Info values, names, comments, field values) sit in data fields only.
- **Hidden text stays hidden** unless asked for. The report counts it, as ADR 0030's `dropped` does. `--include-hidden-text` (command line only) adds each hidden run's page, box, flag and text, so a journalist can read what a black box hides. The MCP tool never offers it, since hidden text is a prompt-injection channel.
- **Exit codes.** Without `--fail-on`, `reveal` exits as other reading commands do: 0, or 3 with reader warnings. With `--fail-on`, it exits **6** when `found` shares a category with the list, and the report prints as usual. Exit 6 ranks between 1 and 3: **2 > 5 > 4 > 1 > 6 > 3 > 0**. A batch exits 6 when some input has findings and none failed, and its summary line gains `found`, the number of inputs with findings. An unknown category is a usage error (exit 2) before any work.
- **Text output**, one line per category with a finding and a short list, for example:
  ```console
  $ laminapdf reveal release.pdf
  release.pdf: history, metadata, hidden-content
    history         2 revisions; the first holds 12 words the latest doesn't
    metadata        Info: Author, Creator, Producer, CreationDate, ModDate, SourceModified; XMP (3.1 KB, 5 namespaces)
    hidden-content  140 covered characters on page 4; layer "Draft notes" off
  ```
- **The MCP tool** `laminapdf_reveal`: read-only (`readOnlyHint: true`, `openWorldHint: false`), listed in name order, with the command's schemas; its input has neither `failOn` nor `includeHiddenText`.
- **Lower layers.**
  - `laminapdf-cos` keeps, per section of the chain, the objects it lists in use and free, and adds `Document::revision_index()`: one entry per revision, oldest first, with its section offset, its end (`revision_ends`) and those lists. A linearized file's two sections are one revision, as in T2b. This is the index E10h and E14b plan.
  - `laminapdf-model` gets the scan: `Document::disclosure() -> Result<Disclosure>` (new, `src/disclosure.rs`), which reads every object once and classifies what it finds (metadata streams, `/PieceInfo`, `/Thumb`, embedded files and file specifications, actions and scripts, annotations, fields, `/OCProperties`), each item with its object number so a sanitiser can find it. It also finds unreferenced objects, by walking from the trailer (`/Root`, `/Info`, `/Encrypt`) and leaving out object and cross-reference streams. `Info::keys()` (new) lists every key.
  - The façade adds `Document::reveal(&self, options: &OpenOptions, cancel: &CancelToken) -> Result<Reveal>` (feature `extract`), owned records that combine the revision index, T2b's comparison, `Disclosure` and `hidden_text`. It maps changed objects to pages through each page's dictionary and content streams.
  - T2b's `redaction-not-applied` then reads its Redact annotations from `Disclosure`, so one scan finds them.
- **ADR 0031's amendment** (Tasks).

**Out of scope** (and which todo covers it)

- Removing anything: [E11d](../ready/E11d-sanitiser.md) (the sanitiser, by these categories) and [E12d](../ready/E12d-scrub-and-rewrite.md) (redaction scrubbing). `reveal` only reads.
- XMP properties beyond their namespaces: [E1b](../done/E1b-xmp-packet-model.md)'s packet model; port the scan to it when it lands.
- Text drawn in layers that are off: no ADR 0030 flag finds it yet, so the report names those layers without counting their text.
- Classifying what each revision changed under a signature's rules, and the panels: [E10h](../ready/E10h-revision-diff-and-attacks.md), [E14b](../ready/E14b-revision-browser.md), [E3g](../ready/E3g-inspectors.md).
- WASM and TypeScript bindings, and a "what this file reveals" panel in the web reader: later, in the reader's rounds.
- Showing hidden text through MCP: never (Scope).

## Tasks

- [x] Claim via the work PR; a fresh worktree off `main`; load `/rust-best-practices`, and `/writing` for the README and the ADR text.
- [x] **`laminapdf-cos`.** Keep each section's in-use and free object numbers as `src/reader/xref.rs` reads the chain (hybrid `/XRefStm` sections count with the section that points to them, and object 0, the head of the free list, is never counted as freed). Add `Revision` (new: `section`, `end`, `in_use`, `freed`) and `Document::revision_index() -> Result<Vec<Revision>>` in T2b's `src/reader/revisions.rs`, empty for a repaired file. Unit tests over `hello-incremental.pdf`, `signed.pdf`, the linearized fixture and a three-revision file built in the test.
- [x] **`laminapdf-model`.** Add `Info::keys()`. Add `src/disclosure.rs` with `Disclosure` and its item types (names are yours; keep each module under 800 lines, splitting into `disclosure/` if needed). Reuse `ActionKind` for actions, following `/Next` with a visited set. Read JavaScript from `/JS` strings and streams; a stream that fails to decode within `max_stream_decoded` counts its stored length. Read field names through `/Parent` (`a.b.c`), the off layers from `/OCProperties /D /OFF`, and Redact annotations with their pages. Decode strings as text strings (PDFDocEncoding or UTF-16); keep names and numbers as written.
- [x] **Façade** (`crates/laminapdf/src/reveal.rs`, new). `Reveal` and its records (plain owned data, counts as `u32`, ADR 0028 F5), built from `revision_index`, T2b's comparison, `disclosure()`, `hidden_text` and `info()`. `found` follows the table. Check the token between categories and between revisions. Point T2b's Redact count at `Disclosure`.
- [x] **The command** (`crates/laminapdf-cli/src/commands/reveal.rs`, new). `RevealArgs` (`input`, `fields`, `include_hidden_text`), `RevealReport` and its parts with `JsonSchema`, `RevealReport::SCHEMA = "laminapdf.reveal/1"`, `reveal(args, settings, io)`. Re-export from `commands/mod.rs`.
- [x] **The front end.** `Command::Reveal` in `src/main.rs`, with `--fail-on` (a `clap::ValueEnum` list with `all`) beside `Many<RevealArgs>`: the command never sees it, and the front end maps `found` to exit 6. `render::reveal_text`, and `render::keep_fields` for `--fields` as `info` applies it. `ExitCode::Found` in `src/exit.rs`: number 6, the priority in Scope, `meaning` "the file holds something `--fail-on` named"; `ALL` gains it, so `laminapdf help exit-codes` lists it. `BatchSummary` gains `found`, and `exit_code` returns 6 as Scope says.
- [x] **Schemas.** Add `("reveal.schema.json", report::<RevealReport>(…))` to `schema::all()`, `"reveal"` to `COMMANDS` and an arm to `command()`, with the `result` and `summary` lines as for `check`. Run `UPDATE_GOLDEN=1 cargo test -p laminapdf-cli --test json_schemas` to write `reveal.schema.json` and the updated `summary-line.schema.json`.
- [x] **MCP.** In `src/mcp/tools.rs`, add `laminapdf_reveal` with the description "List what a PDF holds beyond its visible pages: earlier revisions, metadata, attachments, scripts and actions, annotations, form values, hidden layers and counts of hidden text." Give the tool its own argument type, `RevealToolArgs` (`input` and `fields`, with `deny_unknown_fields`), for its input schema and for parsing, and keep `tools/list` in name order (`laminapdf_reveal` after `laminapdf_pages_rotate` when that is listed). In `src/mcp/mod.rs`, add a `run_tool` arm before the write tool's catch-all that builds `RevealArgs` from it with `include_hidden_text` off.
- [x] **ADR 0031.** Add `laminapdf.reveal/1` to §6's list, and this amendment after the batch one:
  > **Amendment (T2c, <date of merge>): `reveal` and exit code 6.** `laminapdf reveal` lists what a file holds beyond its visible pages, as `laminapdf.reveal/1`, and `laminapdf mcp` serves it as the read-only `laminapdf_reveal`. With `--fail-on CATEGORY,…` (or `all`), it exits `6` when the file holds anything in those categories, and prints its report as usual. `6` ranks between `1` and `3` (**2 > 5 > 4 > 1 > 6 > 3 > 0**), and only `--fail-on` returns it, so no other command's exit codes change. A batch exits 6 when some input holds what `--fail-on` names and none failed, and its summary line counts those inputs as `found`. Exit 3 wasn't reused: it also means "repaired", and `--warnings-ok`, which many wrappers pass, turns it into 0.
- [x] **E11d's categories.** Check that [E11d](../ready/E11d-sanitiser.md)'s scope uses these category names, so its sanitiser removes what `reveal` lists; if it doesn't yet, edit it in this PR.
- [x] **Capability matrix.** Tag the revision-index test `/// capability: M.12 core read` and regenerate `docs/capabilities/README.md` with `cargo xtask capabilities`.
- [x] **Docs.** `crates/laminapdf-cli/README.md`: `reveal` in Commands with the categories, exit code 6 in Exit codes, a CI example (`laminapdf reveal dist/*.pdf --fail-on history,hidden-content,active-content,attachments`) and the MCP tool. CLAUDE.md: the CLI paragraph (commands, exit codes 0–6, the MCP tools), the façade's `reveal`, `laminapdf-model`'s `disclosure`.

## Acceptance criteria

- [x] A fixture built in the test has two revisions (the second removes a sentence), an Info dictionary with a custom key, an XMP packet, an attached file, a JavaScript `/OpenAction`, a `Launch` link action, a `Text` annotation with a comment, a filled text field, a layer off by default and a covered word. `laminapdf reveal --json` lists each in its category, and `found` holds all seven. The report validates against the committed `reveal.schema.json`.
- [x] `fixtures/corpus/hello.pdf` (one revision, no Info dictionary) reports an empty `found`, and `--fail-on all` exits 0.
- [x] `--fail-on hidden-content` exits 6 on the covered fixture and prints the report; `--fail-on all` exits 6 on the full fixture; `--fail-on bogus` exits 2 before reading. A batch of `hello.pdf` and the covered fixture exits 6, with `found: 1` in its summary line; adding an unreadable input makes it exit 1.
- [x] `fixtures/corpus/hello-incremental.pdf` lists two revisions: the second changes object 5, page 1's content stream, and nothing else; the first holds 1 word the latest doesn't. `fixtures/revisions/hello-linearized.pdf` lists one revision and `linearized: true`. A file with an object nothing references counts it.
- [x] Without `--include-hidden-text`, the covered word appears nowhere in the output; with it, it appears once, under `hiddenContent`. `crates/laminapdf-cli/tests/injection.rs` shows that Info values, comments and field values from its injection fixture appear only in data fields, never in a message, hint or tool description.
- [x] `laminapdf mcp` lists `laminapdf_reveal` with `readOnlyHint: true`, in name order, with and without `--allow-write`. Its input schema has no `failOn` or `includeHiddenText`, and a call that passes `includeHiddenText` is a tool error. Its `structuredContent` validates against its `outputSchema`, and `tools/list` is byte-identical across two runs.
- [x] `laminapdf help exit-codes` lists 6; `crates/laminapdf-cli/tests/exit_codes.rs` covers `ExitCode::ALL` and the new priority; `laminapdf schema reveal` validates against the 2020-12 meta-schema (`tests/schema.rs`).
- [x] `reveal` runs without error on every file of `fixtures/corpus/`, `fixtures/golden/` and `fixtures/signed/`. A JavaScript stream past `max_stream_decoded` is counted by its stored length, and the run doesn't fail.
- [x] ADR 0031 carries the amendment and the §6 entry; `docs/capabilities/README.md` shows `M.12` read in the core column, and `cargo xtask capabilities --check` passes.

## As built

- **Size.** About 1,765 lines of code with unit tests (schemas, fixtures and integration tests aside), more than twice the budget. The owner chose to keep all seven categories in one PR rather than split active content, annotations and forms out: they share one scan, one report and one set of tests.
- **Exit code 6** as recommended; ADR 0031 carries the amendment.
- **`laminapdf-cos`.** `XrefTable::section_objects` keeps each section's in-use and free numbers (`SectionObjects::of`, a hybrid section's `/XRefStm` entries with it, object 0 never freed); `Document::revision_index()` and `Revision` sit in `reader/revisions.rs`. The tests use `fixtures/signed/approval.pdf` (there is no `signed.pdf`) and append a third revision that frees object 4.
- **`laminapdf-model`.** `src/disclosure/` (`mod.rs` types and entry point, `objects.rs` the pass over every object and the unreferenced walk, `actions.rs`, `forms.rs` for annotations, fields and layers). Actions are listed at most 10,000 per file. `Info::keys()`.
- **Fixtures.** `laminapdf_test_fixtures::disclosing()` builds the full fixture and `update()` appends an incremental section; the CLI's `with_update` now calls it.
- **Façade.** `laminapdf::reveal` (`Reveal`, `RevisionRecord`, `Category`) re-exports the disclosure types. Hidden runs keep their text (`keep_spans`) and drop `Artifact` entries, so only ADR 0030's six flags count. `redaction-not-applied` reads `Disclosure::redactions()`.
- **A T2b bug, fixed here.** `laminapdf check` failed with `unsupported-filter` on `fixtures/corpus/reportlab.pdf`, which it passed before T2b, because the hidden-text pass and the word comparison read every page's text. Both now skip a page whose text can't be read (kinds `input` and `unsupported`); T2b's fixtures-wide test no longer skips files whose check fails.
- **CLI.** The text summary is in `src/reveal_text.rs` (`render.rs`'s `count` and `size` are shared). `laminapdf_reveal` sorts after `laminapdf_render`, last in name order. Like `laminapdf_info`, it takes `fields` but answers with the whole report, which its `outputSchema` describes. After the security review on the PR, the owner asked for its result to withhold every string the document wrote (Info values, comments, field names and values, file and layer names), so the report has `documentText`, `forms[].filled` and `layersOff[].name`, and the CLI alone shows the strings.

## Post-merge checks

None.

## Tests

- **Unit:** `revision_index` (sections to revisions, linearized merge, hybrid sections); `Disclosure`'s classifiers, one per kind of object, on raw dictionaries; the unreferenced-object walk (cycles, object streams); `found` per category; `ExitCode` priority; `BatchSummary::exit_code` with findings.
- **Integration:** `crates/laminapdf/tests/reveal.rs` (new) for the façade over the fixtures above; `crates/laminapdf-cli/tests/reveal.rs` (new) for text, JSON, `--fields`, `--fail-on`, `--include-hidden-text` and batches; `tests/mcp.rs`, `tests/injection.rs`, `tests/exit_codes.rs`, `tests/json_schemas.rs`, `tests/schema.rs`.
- **Fixtures:** build the full fixture in the test from raw objects (`laminapdf_test_fixtures::classic`) plus an update (`laminapdf_save::test_support::incremental_update`); reuse T2b's linearized file. Commit no new PDF unless a case can't be built.

## Docs to update

- [x] ADR 0031 (the amendment and §6)
- [x] `crates/laminapdf-cli/README.md`, CLAUDE.md, `docs/capabilities/README.md` (regenerated)
- [x] [E11d](../ready/E11d-sanitiser.md)'s scope, if it doesn't use these category names yet
- [x] [Trust and proof §T2](../../plans/trust-and-proof.md#t2-find-what-files-hide): tick T2c

## Risks and open questions

- **Exit code 6** is a change to ADR 0031's contract, so the owner decides it in this PR. The alternative is exit 3, with each finding raised as a warning. It needs no new code, but a gate then fails on harmless repairs and passes under `--warnings-ok`. Recommendation: 6.
- **"Found" rules.** Nearly every file has an Info dictionary, so `metadata` is nearly always found, and generators leave unreferenced objects behind (unused fonts, old copies), so `history` often is too. The rules state facts; users pick the categories their gate needs, and the README's CI example leaves `metadata` out.
- **One scan, several later models.** `Disclosure` reads attachments, actions, annotations, fields and layers before their typed models exist (E3f, E4b, E8c, E17c). When each lands, `Disclosure` reads through it and the report stays the same; the risk is two readers of one structure in the meantime, which ADR 0004's two-parsers rule warns about for rendering. Keep `Disclosure`'s readers small and tested on the same fixtures those todos will use.
- **Document strings in agent output.** The report puts Info values, comments and field values in front of an agent. ADR 0031 §9 allows document text in data fields only, and the injection test holds that; hidden text never reaches MCP.
- **Big files.** `reveal` reads every object, every page's text and every earlier revision. The token, the `Limits` and the MCP server's `--timeout` bound it; `check` already reads every object.
- **Size.** If the PR passes 800 lines, split it as the Size field says, and add the second todo to the plan's T2 section.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
