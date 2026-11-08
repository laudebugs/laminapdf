# T4d — Markdown in, flowing pages out: `laminapdf build`, an MCP tool and the SDK

| Field | Value |
| --- | --- |
| ID | `T4d` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4c](./T4c-flow-tables-notes-contents.md) |
| ADRs | Implements ADR 0032 (written by [T4a](../ongoing/T4a-adr-0032-layout-engine.md)): the Markdown dialect, where the front end lives, the CLI verb and the MCP tool. Follows [0031](../../decisions/0031-cli-and-mcp-contract.md) as T4a amends it (`build`, `laminapdf.build/1`, a write tool under `--allow-write`), [0016 §6](../../decisions/0016-package-design.md#6-budgets-per-module) (module budgets), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md#rust-cargo-deny) (the parser's licence) and [0026 §4](../../decisions/0026-error-and-diagnostic-contract.md#4-no-document-text-in-message-hint-or-didyoumean) (no document text in messages) |
| Branch | `feat/t4d-markdown-in` |
| PR title | `feat: Markdown in, flowing pages out, in Rust, the CLI, MCP and the SDK` |
| Size | XL (≈ 1,100–1,400 lines with the CLI, MCP and bindings): [T4a](../ongoing/T4a-adr-0032-layout-engine.md) splits it into todos of at most L once ADR 0032 decides the design |
| PR | — |

> **Split.** JSON documents and templates moved to [T4e](./T4e-json-documents-and-templates.md) so that each PR stays reviewable. The file name keeps "json-templates" because the research report links to it. Like T4b and T4c, this todo assumes ADR 0032 chooses an engine of our own; T4a rewrites it if the ADR decides otherwise.

## Goal

Turn Markdown into flowing, tagged pages: from Rust, from `laminapdf build report.md -o report.pdf`, from an MCP tool that a coding agent can call with the Markdown inline, and from `@laminapdf/sdk`. Writing in Markdown or another plain markup is the largest generation request on Hacker News, about 1,140 comments, and people say what they would do with it: "if you have a markdown->PDF function included, where I can send in markdown and get PDF, that would solve quite many needs" ([46325132](https://news.ycombinator.com/item?id=46325132)). Today they chain tools for it ("MD-->(Pandoc-->(typst))--> PDF", [43232394](https://news.ycombinator.com/item?id=43232394)). Talk of AI near generation rose from 6 to 263 comments, and Markdown is the format agents already write.

## Context

- [T4b](./T4b-flow-layout-core.md) and [T4c](./T4c-flow-tables-notes-contents.md): the flow model, keep rules, templates, tables, footnotes, contents, named destinations per heading. ADR 0032: the dialect and its extensions, what is dropped, placement (a `markdown` feature of `laminapdf-build` or a crate of its own), the WASM module and its budget, the CLI verb and the MCP tool.
- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.1 (1,424 matches, 8 of 10 on topic), §5 (AI near generation) and §6 point 8; the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json).
- The parser: [`pulldown-cmark`](https://github.com/pulldown-cmark/pulldown-cmark) (MIT), a CommonMark parser with options for tables, footnotes, strikethrough, task lists and metadata blocks, and no `std::fs`. Its licence and its dependencies go through `cargo xtask deny`.
- The CLI ([L4a](../done/L4a-lamina-cli.md), [L4b](../done/L4b-cli-batch-render-schema.md)): one function per command in `crates/laminapdf-cli/src/commands/` over `Settings` and `&mut dyn Io`, the clap `Command` enum in `src/main.rs`, `schema::all()` and `COMMANDS` in `src/schema.rs` (schemas rewritten by `UPDATE_GOLDEN=1 cargo test -p laminapdf-cli --test json_schemas`), and `Io::read`, which reads paths and `-`.
- MCP ([L4c](../done/L4c-mcp-mode.md)): `tools(allow_write, allow_in_place)` in `src/mcp/tools.rs` (in name order; `laminapdf_pages_rotate` is the only write tool today) and `RootedIo` in `src/mcp/roots.rs`, which refuses paths outside every root (`path-outside-roots`).
- WASM and TypeScript: `WasmBuilder` (`crates/laminapdf-wasm/src/builder/document.rs`), records as JSON with `unknown-option` ([ADR 0016 §7](../../decisions/0016-package-design.md#7-api-naming-and-shape-at-the-wasm-boundary)), the SDK's `Builder` (`packages/sdk/src/builder.ts`), and the runtime suite (`packages/sdk/test/suite/examples.ts`), which [B6c](../done/B6c-runtime-matrix.md) runs in every runtime.
- Examples: the [examples guideline](../../guidelines/examples.md) and the TypeScript–Rust pairs in `tools/xtask/docs-snippets.toml`.
- `TagKind` has `Code` but no `BlockQuote` (PDF 1.7's block quote, ISO 32000-1 §14.8.4).
- No message or hint carries document text: `crates/laminapdf/tests/error_contract.rs` checks the façade's against a marker string, and the same rule covers Markdown (a warning about raw HTML gives its position, never the HTML).

## Scope

**In scope**

- **The Markdown front end,** placed as ADR 0032 decides (expected: a `markdown` feature of `laminapdf-build`, in `src/markdown/`, over `pulldown-cmark`). CommonMark plus the extensions ADR 0032 lists (expected: tables, footnotes, strikethrough and task lists), and front matter with `title`, `lang`, `author` and `subject` as plain `key: value` lines, so no YAML parser is needed.
- **The mapping,** one construct to one flow block and structure type:

  | Markdown | Flow (T4b, T4c, this todo) | Tag |
  | --- | --- | --- |
  | `#` to `######` | `Heading` | `H1`–`H6` (a skipped level stays an error under PDF/UA-1: `heading-level-skipped`) |
  | paragraph, hard break | `Paragraph`, line break | `P` |
  | `*em*`, `**strong**`, `` `code` ``, `~~strike~~` | spans in the italic, bold and mono fonts; a span struck through | `Span`, `Code` |
  | link | a link span: a URL, or `#slug` to that heading's destination | `Link` |
  | image | `Figure`, its text as alt text, its bytes from the caller | `Figure` with `/Alt` |
  | list, task list | `List` (an ordered list's start number; ☐ and ☑ markers) | `L`, `LI`, `Lbl`, `LBody` |
  | block quote | `Quote` (new block) | `BlockQuote` (new `TagKind`) |
  | code block | `CodeBlock` (new block: mono, wrapped at the body's width) | `P` holding `Code` |
  | thematic break | `Rule` (new block) | `Layout` artifact |
  | table | `Table`, column alignment from the delimiter row, the first row as header | `Table`, `THead`, `TBody`, `TR`, `TH`, `TD` |
  | footnote | a note | `Reference`, `Note` |
  | front matter | metadata and language | `/Title`, `/Lang` |

- **What is dropped,** as warnings in T4b's `buildWarnings` group, each with its `docs/errors/` page and a location (a byte offset into the Markdown, in its `data`) instead of the text: `markdown-html-ignored` (raw HTML) and `markdown-image-missing` (an image the caller didn't supply, or a remote URL: nothing is fetched).
- **Options** (`MarkdownOptions`, new): fonts by role (body, bold, italic, bold italic, mono, headings), sizes, the page template (by default A4, its margins, and a "Page {page} of {pages}" footer), and images as a map from the link target to an `Image`. Without fonts, the front end uses the standard 14 Helvetica family and Courier, which set Latin text only, unshaped, and which PDF/UA-1 refuses (`font-not-embedded`). ADR 0032 may choose another default.
- **The façade:** a `markdown` feature with `laminapdf::markdown` and `Builder::markdown(text, &MarkdownOptions)` (new), returning what the flow added and its warnings.
- **The CLI:** `laminapdf build <input.md | -> -o OUT`, with `--font ROLE=FILE` (repeatable), `--page-size`, `--profile`, `--lang` and `--title`.
  - Images are read through `Io` from paths relative to the input's folder.
  - `--json` prints `laminapdf.build/1` (output, page count, bytes, tagged, profiles, warnings), committed as `crates/laminapdf-cli/schemas/build.schema.json`.
  - `--dry-run` builds and reports without writing; `laminapdf schema build` prints the schema.
  - Exit codes and overwrites follow ADR 0031 §4–§5. For a `missing-glyph` or `font-not-embedded` error, the hint names `--font`, as ADR 0031 §8 asks.
  - Output is deterministic: no creation date unless `--creation-date` is given.
- **MCP:** `laminapdf_build`, listed with `--allow-write` only, as ADR 0031 is amended by T4a. It takes the Markdown inline (`markdown`) or from a path (`input`), writes `output` inside the roots, reads images through `RootedIo`, never overwrites and never fetches.
- **WASM and TypeScript:** `WasmBuilder.markdown(text, options)` in the module ADR 0032 names (expected: the text module), and `builder.markdown(text, options)` in `@laminapdf/sdk`, with fonts as `Font`s and images as `Image`s by name. `cargo xtask size` passes, or the PR carries a budget amendment for ADR 0016 §6 with the measurement, for the owner to approve.
- **Examples:** `crates/laminapdf/examples/markdown_to_pdf.rs` and `packages/sdk/examples/markdown-to-pdf.ts` (a pair in `tools/xtask/docs-snippets.toml`) over `fixtures/markdown/report.md` (new: every construct in the table, an Arabic and a Devanagari paragraph, a table across pages, footnotes, a contents page and an image), with byte and render goldens, veraPDF under PDF/UA-1, and a place in the runtime suite.
- **A `markdown` fuzz target:** any text in, a document or a coded error out, never a panic.

**Out of scope** (and which todo covers it)

- JSON documents and templates: [T4e](./T4e-json-documents-and-templates.md).
- Maths (`$…$` stays text), raw HTML, syntax highlighting, styling beyond `MarkdownOptions` (no CSS), remote images, and Markdown through Pandoc or Typst.
- Fonts bundled with the CLI or the SDK: [pdf-library §9](../../plans/pdf-library.md#9-open-questions) question 2, unless ADR 0032 answers it.
- Several inputs in one run (ADR 0031's batch mode): later, if users ask.

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`, and `/writing` for the docs pages
- [ ] Read ADR 0032 and T4b's and T4c's flow modules; check this todo against the decision
- [ ] Add `pulldown-cmark` to `[workspace.dependencies]` (default features off, as the crate needs), behind the `markdown` feature, and run `cargo xtask deny`
- [ ] The new blocks `Quote`, `CodeBlock` and `Rule` in the flow model, with their layout, pagination and tags; `TagKind::BlockQuote` with `laminapdf-wasm`'s name table and `cargo xtask gen boundary`
- [ ] `src/markdown/` (or the crate ADR 0032 names): front matter, the event-to-block mapping in the table, slugs for `#` links, `MarkdownOptions` with its defaults, the two warnings with their `codes.json` entries and `docs/errors/` pages, then `cargo xtask gen codes`
- [ ] The façade's `markdown` feature and `Builder::markdown`
- [ ] The CLI: `commands/build.rs` (new; `build(args, &Settings, &mut dyn Io)`, which never prints), `BuildArgs` and `BuildReport` with `JsonSchema`, the `Build` variant in `src/main.rs`, `schema::all()` and `COMMANDS`, the committed schema, CLI hints for the font errors, and the CLI README
- [ ] MCP: `laminapdf_build` in `src/mcp/tools.rs` behind `allow_write`, with annotations that say it writes and isn't destructive; update `the_tools_should_be_in_name_order_and_read_only_by_default` for the new order
- [ ] WASM: `WasmBuilder.markdown` with a tsify options record; TypeScript: `Builder.markdown` and its types; `cargo xtask wasm` and `cargo xtask size`
- [ ] The fixture `fixtures/markdown/report.md` with its provenance row in `fixtures/README.md`, the example pair, the goldens (`cargo xtask golden --update`, `cargo xtask render-golden --update`), and the runtime suite entry
- [ ] The fuzz target: `fuzz/fuzz_targets/markdown.rs`, its `[[bin]]` in `fuzz/Cargo.toml`, `FUZZ_TARGETS` in `tools/xtask/src/harness.rs`, seeds from the fixture
- [ ] The docs (Docs to update)

## Acceptance criteria

- [ ] Each row of the mapping table has a test in `crates/laminapdf-build/tests/markdown.rs` (new) that builds the construct and reads back its block and structure type (`laminapdf_model::Document::structure()`)
- [ ] `fixtures/markdown/report.md` builds the golden's bytes in Rust and in every runtime (`cargo xtask pdf-runtimes`), passes `cargo xtask verapdf` under PDF/UA-1 and `qpdf --check`, and extracts its Arabic and Devanagari paragraphs as written
- [ ] `laminapdf build fixtures/markdown/report.md` with the fixture fonts writes the golden's bytes; its `--json` report validates against `build.schema.json`; an existing output without `--force` exits 4; raw HTML exits 3 with `markdown-html-ignored` (0 with `--warnings-ok`)
- [ ] The CLI reads an image from a path relative to the input's folder; under MCP, an image or output path outside every root fails with `path-outside-roots`
- [ ] `crates/laminapdf-cli/tests/mcp.rs`: `laminapdf_build` builds from inline Markdown inside a root, and is absent without `--allow-write`
- [ ] Markdown whose raw HTML and image targets hold a marker string gives warnings whose messages and hints lack it
- [ ] `pulldown-cmark` and its dependencies pass `cargo xtask deny`
- [ ] The `markdown` fuzz target runs in the full lane (`cargo xtask fuzz --all`) without a crash
- [ ] `cargo xtask size` passes, with a budget amendment if needed
- [ ] `cargo xtask check docs` passes with the new example pair shown on a docs page

## Post-merge checks

- [ ] The nightly fuzz job runs the `markdown` target for its 10 minutes without a crash

## Tests

- `crates/laminapdf-build/tests/markdown.rs` (new): the mapping table, front matter, slugs, the two warnings, the defaults without fonts.
- `crates/laminapdf-cli/tests/`: `build` end to end (golden bytes, `--json` against the schema, `--dry-run`, exit codes) and `mcp.rs` for the tool.
- `crates/laminapdf/tests/examples.rs` for the Rust example; `packages/sdk/test/suite/examples.ts` for the TypeScript one, in every runtime.
- `crates/laminapdf/tests/error_contract.rs`: the marker check over Markdown warnings.
- The `markdown` fuzz target.

## Docs to update

- [ ] A recipe on the docs site, "Turn Markdown into a PDF" (`apps/docs/src/content/docs/recipes/markdown-to-pdf.mdx`, new), with both examples and the CLI command
- [ ] The "Pages that flow" guide: a section on Markdown
- [ ] `crates/laminapdf-cli/README.md` (the `build` command); `packages/sdk/examples/AGENTS.md`, so coding agents know the format
- [ ] `CLAUDE.md`: the Building API and CLI paragraphs, the fuzz target list, the commands list
- [ ] `fixtures/README.md`: the Markdown fixture and the new goldens
- [ ] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **Size.** If the PR passes about 1,400 lines, split it here: the Rust front end with the façade first, then the CLI, MCP and bindings.
- **WASM size.** The parser and the mapping add to a module with little headroom (the text module had 5.1 % on 2026-11-05). The owner decides any increase, as ADR 0016 §6 requires.
- **Fonts.** Without `--font`, only Latin text sets, and PDF/UA-1 fails. The CLI's hints must say so plainly; a bundled default font is the owner's call (pdf-library §9 question 2).
- **Markdown dialects differ.** Users will bring GitHub's and Pandoc's extras. The ADR's list is the contract, and anything else is plain text or a coded warning, never a silent guess.
- **Untrusted Markdown from agents.** Images are read only through `Io` (inside the roots under MCP) and nothing is fetched. The fuzz target checks that odd input ends in a document or a coded error, and MCP's `--timeout` stops a run that takes too long.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
