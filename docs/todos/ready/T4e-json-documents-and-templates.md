# T4e — JSON documents and templates in, flowing pages out, in Rust and the CLI

| Field | Value |
| --- | --- |
| ID | `T4e` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4i](./T4i-build-command-and-bindings.md) |
| ADRs | Implements [ADR 0032 §6](../../decisions/0032-flowing-layout-and-document-inputs.md#6-inputs) (the `laminapdf.document/1` format and placeholder templates), [§5](../../decisions/0032-flowing-layout-and-document-inputs.md#5-placement) (`serde` and `schema` features on `laminapdf-build`) and [§7](../../decisions/0032-flowing-layout-and-document-inputs.md#7-surfaces) (`--data` and the MCP tool's document input). Follows [0031](../../decisions/0031-cli-and-mcp-contract.md) as ADR 0032 amends it (`laminapdf build`, its MCP tool) and its [§6](../../decisions/0031-cli-and-mcp-contract.md#6-json-output) versioning rule, and [0026 §4](../../decisions/0026-error-and-diagnostic-contract.md#4-no-document-text-in-message-hint-or-didyoumean) (no document text in messages) |
| Branch | `feat/t4e-json-documents-and-templates` |
| PR title | `feat(build): JSON documents and templates in Rust and the CLI` |
| Size | L (≈ 600–750 lines: the derives and schema test, validation, the template module, the CLI and MCP inputs, the example and the fuzz target; the schema, fixtures and goldens don't count) |
| PR | — |

## Goal

Let a program or an LLM write a flowing document as JSON, check it against a committed schema, and fill it from data: an invoice template plus its line items gives a paginated, tagged invoice, from Rust and from `laminapdf build invoice.json --data items.json`. Filling a template from data is the second-largest generation request on Hacker News, about 560 comments: "I make invoices by having a Typst template read line items from a JSON file." ([39672973](https://news.ycombinator.com/item?id=39672973)) and "It’s absolutely trivial for example to build a Lambda function that parses some json input into a PDF." ([45394409](https://news.ycombinator.com/item?id=45394409)) Talk of AI near generation rose from 6 to 263 comments ("Frontier LLMs work great with Typst.", [48548712](https://news.ycombinator.com/item?id=48548712)); a format with a schema lets an agent write it and a program check it before anything is drawn. The typed TypeScript form comes next, in [T4j](./T4j-documents-in-typescript.md).

## Context

- [ADR 0032 §6](../../decisions/0032-flowing-layout-and-document-inputs.md#6-inputs) fixes the format and the templates. `laminapdf.document/1` is the flow model as JSON (camelCase names, block kinds tagged by `type`), with its JSON Schema committed and versioned under ADR 0031 §6's rule; fonts and images are named in the document and supplied beside it as bytes. Templates are placeholders inside the document JSON: `{ "$data": "/pointer" }`, `{ "$each": "/array", "do": … }` and `{ "$if": "/pointer", "do": … }`, with RFC 6901 pointers, no expressions and no code, and a filled template is still a valid document. The ADR measured and rejected a text template engine: MiniJinja adds 170 KB gzip and can write broken JSON. The same section makes the document tree the declarative layer that pdf-library §9 question 3 asks for, and offers it to agents beside Markdown.
- The flow model this format serialises: [T4b](../ongoing/T4b-flow-layout-core.md), [T4g](../done/T4g-flow-styled-spans.md), [T4c](./T4c-flow-tables-notes-contents.md), [T4h](./T4h-flow-notes-contents-outline.md) and [T4d](./T4d-markdown-documents.md)'s three new blocks. [T4i](./T4i-build-command-and-bindings.md): the `laminapdf build` command and the `laminapdf_build` tool this todo extends.
- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.2 (799 matches, 7 of 10 on topic), §1.9 (Typst run in production from data: 94 comments in 2023–26), §5 and §6 point 8; the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json).
- The precedent for a JSON contract: `laminapdf-extract` has `serde` and `schema` features, and `schemas/extract.schema.json` is checked by `tests/output.rs` (`UPDATE_GOLDEN=1` rewrites it). See [L3e](../done/L3e-extract-bindings-and-benchmark.md).
- Unknown keys: `laminapdf::did_you_mean` (pinned by `fixtures/did-you-mean.json`).
- RFC 6901 (JSON Pointer), for paths into the data.
- The trust and proof plan's [§3](../../plans/trust-and-proof.md#3-rules-for-this-round) rule 5: within a schema version, keys are only added.
- A Typst user reports a two-page invoice in under 100 ms ([45697377](https://news.ycombinator.com/item?id=45697377)). ADR 0032 [§9](../../decisions/0032-flowing-layout-and-document-inputs.md#9-budgets) sets ≤ 10 ms native for the two-page invoice; T4j records it in every runtime.

## Scope

**In scope**

- **The document format,** `laminapdf.document/1`: `serde` and `schema` features on `laminapdf-build` with derives on the flow model (camelCase names, block kinds tagged by `type`). Fonts and images are named in the document and supplied beside it as bytes, which keeps the core bytes-in, bytes-out. The page template, metadata (title, language) and profiles are part of it. The schema is committed as `crates/laminapdf-build/schemas/document.schema.json` and checked by a test; within `/1`, keys are only added.
- **Validation:** an unknown key or a wrong value fails with `invalid-document` (new, `buildErrors`). The error carries the value's JSON Pointer in its `data`, never in its message, and a misspelt key gets a `didYouMean` from the format's own key names (`laminapdf::did_you_mean`).
- **Templates,** ADR 0032 §6's three placeholders:
  - `{ "$data": "/invoice/number" }` in place of a text or number: a JSON Pointer into the data;
  - `{ "$each": "/items", "do": … }` repeats a block or a table row per array item; inside `do`, `{ "$item": "/name" }` points into the current item;
  - `{ "$if": "/paid", "do": … }` keeps a block when the value is present and not `false` or `null`.

  Values are written as given: the data arrives formatted. Errors (new, `buildErrors`): `template-data-missing` (a pointer with no value) and `template-data-type` (`$each` over something that isn't an array, or text from an object). Both carry the pointer in their `data`.
- **Rust:** `laminapdf::flow::fill(template, &data)` (new) gives a `Flow`; `Builder::flow` (T4b) builds it.
- **The CLI:** `laminapdf build doc.json [--data FILE|-] -o OUT`. A `.json` input whose `schema` is `laminapdf.document/1` takes `--font NAME=FILE` and `--image NAME=FILE`, and resolves names it isn't given from files next to the document. The `laminapdf.build/1` report gains keys only.
- **MCP:** `laminapdf_build` takes `document` and `data` inline, beside T4i's `markdown` and `input`. Its `inputSchema` inlines `laminapdf.document/1` under `$defs`, as `laminapdf.error/1` is inlined today.
- **The example:** `crates/laminapdf/examples/invoice_from_json.rs` over `fixtures/documents/invoice.json` (the template) and `fixtures/documents/invoice-data.json` (40 line items). The output runs to two pages, with the table header on both and the totals with the last rows, under PDF/UA-1. It has byte and render goldens.
- **A `document` fuzz target:** any JSON in as document and data, a document or a coded error out, never a panic.

**Out of scope** (and which todo covers it)

- `WasmBuilder.flow`, `builder.flow(document, { data })`, the generated `FlowDocument` types, the TypeScript example and the `invoice-from-json` timing in `cargo xtask pdf-runtimes`: [T4j](./T4j-documents-in-typescript.md).
- Typst templates and text template engines: rejected by ADR 0032 §2 and §6.
- Number, currency and date formatting, expressions, and loops other than `$each`: the data arrives formatted. A later todo can add formatting if users ask.
- A JSX layer over the document tree (ADR 0032 §6 leaves it for later, mapped onto this format), and a visual template designer.

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`, and `/writing` for the docs pages
- [ ] Read ADR 0032 §6, the flow model and T4i's `build` command
- [ ] `laminapdf-build`: the `serde` and `schema` features (optional `serde`, `serde_json` and `schemars` from `[workspace.dependencies]`, as `laminapdf-extract` has them), the derives, the `schema` key, and fonts and images by name
- [ ] The schema test in `crates/laminapdf-build/tests/document_schema.rs` (new): writes the schema with `UPDATE_GOLDEN=1`, fails when it is stale, and validates every fixture document against it with `boon`, as the façade's tests do
- [ ] `flow/template.rs`: `fill` with `$data`, `$each`, `$item` and `$if`; the three new codes with their `codes.json` entries and `docs/errors/` pages, then `cargo xtask gen codes`
- [ ] The façade: `laminapdf::flow::fill`, behind the façade feature that turns on `laminapdf-build`'s `serde` (decide in review whether `text` implies it)
- [ ] The CLI: `.json` inputs in `commands/build.rs`, `--data` (standard input once, as ADR 0031 §4 allows), `--font NAME=FILE` and `--image NAME=FILE`; the schema in `schemas/build.schema.json` regenerated
- [ ] MCP: `document` and `data` on `laminapdf_build`; the inlined schema under `$defs`, checked by `every_reference_should_resolve_inside_its_schema`
- [ ] The fixtures with their provenance rows in `fixtures/README.md`, the Rust example and its goldens
- [ ] The fuzz target: `fuzz/fuzz_targets/document.rs`, its `[[bin]]` in `fuzz/Cargo.toml`, `FUZZ_TARGETS` in `tools/xtask/src/harness.rs`, seeds from the fixtures
- [ ] The docs (Docs to update)

## Acceptance criteria

- [ ] `document.schema.json` is committed and current, and every fixture document validates against it
- [ ] `invoice_from_json` writes the golden's bytes: two pages, the table header on both, the totals on the same page as the last body row; it passes `cargo xtask verapdf` under PDF/UA-1 and `qpdf --check`
- [ ] `laminapdf build fixtures/documents/invoice.json --data fixtures/documents/invoice-data.json` writes the same bytes, and so does `--data -` with the data on standard input
- [ ] A misspelt key fails with `invalid-document` and a `didYouMean` in Rust and the CLI
- [ ] A pointer with no value fails with `template-data-missing` and the pointer in its `data`; with a marker string in the data and the template, no message or hint contains it
- [ ] `crates/laminapdf-cli/tests/mcp.rs`: `laminapdf_build` builds the invoice from inline `document` and `data` inside a root
- [ ] The `document` fuzz target runs in the full lane without a crash

## Post-merge checks

- [ ] The nightly fuzz job runs the `document` target for its 10 minutes without a crash

## Tests

- `crates/laminapdf-build/tests/document_schema.rs` (new): the schema, round trips (a `Flow` written as JSON and read back is the same `Flow`), unknown keys.
- Unit tests in the template module for each placeholder, nesting `$each` in `$each`, and each error.
- `crates/laminapdf-cli/tests/`: `build` with `.json` inputs and `--data`, and `mcp.rs` for the tool's new arguments.
- `crates/laminapdf/tests/examples.rs` for the Rust example.
- The `document` fuzz target.

## Docs to update

- [ ] A recipe on the docs site, "Fill an invoice from JSON" (`apps/docs/src/content/docs/recipes/invoice-from-json.mdx`, new), with the Rust example and the CLI command (T4j adds the TypeScript tab)
- [ ] The "Pages that flow" guide: the JSON form
- [ ] `packages/sdk/examples/AGENTS.md` and the agent docs: the schema's URL, so coding agents can check what they write
- [ ] `crates/laminapdf-cli/README.md`; `CLAUDE.md`'s Building API and CLI paragraphs and the fuzz target list
- [ ] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **The schema becomes a public contract.** Every flow option it exposes is hard to rename later. Keep `/1` to what T4b, T4g, T4c, T4h and T4d ship, and mark nothing as experimental inside it.
- **A mini template language.** Placeholders without expressions are safe and simple, but users will ask for sums, dates and currency. The recipe must state ADR 0032's answer plainly: the data arrives formatted.
- **MCP tokens.** Inlining the document schema makes the tool's `inputSchema` large. Measure it; if it is too large for clients, reference the schema by its `$id` instead.
- **Size.** If the PR passes about 800 lines, move the CLI and MCP inputs into a PR of their own after the Rust format and templates.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
