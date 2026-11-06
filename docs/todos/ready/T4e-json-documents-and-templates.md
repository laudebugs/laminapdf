# T4e — JSON documents and templates in, flowing pages out

| Field | Value |
| --- | --- |
| ID | `T4e` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4d](./T4d-markdown-documents.md) |
| ADRs | Implements ADR 0032 (written by [T4a](./T4a-adr-0032-layout-engine.md)): the document format and the template language. Follows [0031](../../decisions/0031-cli-and-mcp-contract.md) as T4a amends it (`laminapdf build`, its MCP tool) and its [§6](../../decisions/0031-cli-and-mcp-contract.md#6-json-output) versioning rule, [0016 §6–§7](../../decisions/0016-package-design.md#7-api-naming-and-shape-at-the-wasm-boundary) (budgets; records as JSON with `unknown-option`) and [0026 §4](../../decisions/0026-error-and-diagnostic-contract.md#4-no-document-text-in-message-hint-or-didyoumean) (no document text in messages) |
| Branch | `feat/t4e-json-documents-and-templates` |
| PR title | `feat: JSON documents and templates in, flowing pages out` |
| Size | XL (≈ 900–1,200 lines with the bindings and the generator): [T4a](./T4a-adr-0032-layout-engine.md) splits it into todos of at most L once ADR 0032 decides the design |
| PR | — |

> **Split from [T4d](./T4d-markdown-documents.md),** which keeps Markdown, so that each PR stays reviewable. Like T4b–T4d, this todo assumes ADR 0032 chooses an engine of our own, a JSON document format and placeholders inside it for data; T4a rewrites it if the ADR decides otherwise.

## Goal

Let a program or an LLM write a flowing document as JSON, check it against a committed schema, and fill it from data: an invoice template plus its line items gives a paginated, tagged invoice. In TypeScript the same document is a typed object, which brings the whole flow layer of T4b and T4c to `@laminapdf/sdk`. Filling a template from data is the second-largest generation request on Hacker News, about 560 comments: "I make invoices by having a Typst template read line items from a JSON file." ([39672973](https://news.ycombinator.com/item?id=39672973)) and "It's absolutely trivial for example to build a Lambda function that parses some json input into a PDF." ([45394409](https://news.ycombinator.com/item?id=45394409)) Talk of AI near generation rose from 6 to 263 comments ("Frontier LLMs work great with Typst.", [48548712](https://news.ycombinator.com/item?id=48548712)); a format with a schema lets an agent write it and a program check it before anything is drawn.

## Context

- ADR 0032: the format's name and shape, the template language, and the decision on what agents are offered. [T4b](./T4b-flow-layout-core.md), [T4c](./T4c-flow-tables-notes-contents.md) and [T4d](./T4d-markdown-documents.md): the flow model this format serialises, and the `laminapdf build` command, MCP tool and WASM plumbing this todo extends.
- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.2 (799 matches, 7 of 10 on topic), §1.9 (Typst run in production from data: 94 comments in 2023–26), §5 and §6 point 8; the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json).
- The precedent for a JSON contract: `laminapdf-extract` has `serde` and `schema` features, `schemas/extract.schema.json` is checked by `tests/output.rs` (`UPDATE_GOLDEN=1` rewrites it), and `cargo xtask gen boundary` turns it into `packages/sdk/src/generated/extract.ts` (`tools/xtask/src/codegen/boundary.rs` and `schema_ts.rs`, which fails on a schema keyword it doesn't know). See [L3e](../done/L3e-extract-bindings-and-benchmark.md).
- Unknown keys: `laminapdf::did_you_mean` (pinned by `fixtures/did-you-mean.json`) and the binding's `unknown-option` ([ADR 0016 §7](../../decisions/0016-package-design.md#7-api-naming-and-shape-at-the-wasm-boundary)).
- RFC 6901 (JSON Pointer), for paths into the data.
- The trust and proof plan's [§3](../../plans/trust-and-proof.md#3-rules-for-this-round) rule 5: within a schema version, keys are only added.
- Agent docs ([L5a](../done/L5a-agent-docs.md)): `packages/sdk/examples/AGENTS.md`, `llms.txt`, and the docs folder the packages ship.
- The runtime suite's `measure()` (`packages/sdk/test/suite/examples.ts`) records timings per runtime. [T1c](./T1c-publish-performance-numbers.md) adds a one-page `invoice` case there, publishes generation timings, and leaves flowing-layout timings and a two-page invoice to this round. A Typst user reports a two-page invoice in under 100 ms ([45697377](https://news.ycombinator.com/item?id=45697377)).

## Scope

**In scope**

- **The document format,** `laminapdf.document/1` (or the name ADR 0032 gives): `serde` and `schema` features on `laminapdf-build` with derives on the flow model (camelCase names, block kinds tagged by `type`). Fonts and images are named in the document and supplied beside it as bytes, which keeps the core bytes-in, bytes-out. The page template, metadata (title, language) and profiles are part of it. The schema is committed as `crates/laminapdf-build/schemas/document.schema.json` and checked by a test; within `/1`, keys are only added.
- **Validation:** an unknown key or a wrong value fails with `invalid-document` (new, `buildErrors`). The error carries the value's JSON Pointer in its `data`, never in its message, and a misspelt key gets a `didYouMean` from the format's own key names (`laminapdf::did_you_mean`).
- **Templates,** in the language ADR 0032 picks (expected: placeholders inside the document JSON, with no expressions and no code):
  - `{ "$data": "/invoice/number" }` in place of a text or number: a JSON Pointer into the data;
  - `{ "$each": "/items", "do": … }` repeats a block or a table row per array item, and `{ "$item": "/name" }` points into the current item;
  - `{ "$if": "/paid", "do": … }` keeps a block when the value is present and not `false` or `null`.

  Values are written as given: the data arrives formatted. Errors (new, `buildErrors`): `template-data-missing` (a pointer with no value) and `template-data-type` (`$each` over something that isn't an array, or text from an object). Both carry the pointer in their `data`.
- **Rust:** `laminapdf::flow::fill(template, &data)` (new) gives a `Flow`; `Builder::flow` (T4b) builds it.
- **The CLI:** `laminapdf build doc.json [--data FILE|-] -o OUT`. A `.json` input whose `schema` is `laminapdf.document/1` takes `--font NAME=FILE` and `--image NAME=FILE`, and resolves names it isn't given from files next to the document. The `laminapdf.build/1` report is unchanged apart from added keys.
- **MCP:** `laminapdf_build` takes `document` and `data` inline, beside T4d's `markdown` and `input`. Its `inputSchema` inlines `laminapdf.document/1` under `$defs`, as `laminapdf.error/1` is inlined today.
- **WASM and TypeScript:** `WasmBuilder.flow(document, data)` and `builder.flow(document, { data })` in `@laminapdf/sdk`, with fonts and images as `Font` and `Image` handles by name. The `FlowDocument` types are generated by `cargo xtask gen boundary` into `packages/sdk/src/generated/document.ts` from the committed schema, extending `schema_ts.rs` for any keyword the schema adds. TypeScript users then have the whole flow layer as typed objects, as L3e brought extraction to TypeScript after L3a–L3d built it in Rust.
- **Examples:** `crates/laminapdf/examples/invoice_from_json.rs` and `packages/sdk/examples/invoice-from-json.ts` (a pair in `tools/xtask/docs-snippets.toml`) over `fixtures/documents/invoice.json` (the template) and `fixtures/documents/invoice-data.json` (40 line items). The output runs to two pages, with the table header on both and the totals with the last rows, under PDF/UA-1. It has byte and render goldens and a place in the runtime suite.
- **Timing:** the two-page invoice joins `measure()` as its own case, `invoice-from-json` (beside T1c's one-page `invoice`), so `cargo xtask pdf-runtimes` records it in each runtime as `pdf/<runtime>/invoice-from-json`, for T1c's page to show.
- **A `document` fuzz target:** any JSON in as document and data, a document or a coded error out, never a panic.

**Out of scope** (and which todo covers it)

- Typst templates, unless ADR 0032 adopts them.
- Number, currency and date formatting, expressions, and loops other than `$each`: the data arrives formatted. A later todo can add formatting if users ask.
- A JSX layer over the typed objects ([pdf-library §9](../../plans/pdf-library.md#9-open-questions) question 3, if ADR 0032 leaves it for later), and a visual template designer.
- Publishing the timings: [T1c](./T1c-publish-performance-numbers.md).

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`, and `/writing` for the docs pages
- [ ] Read ADR 0032, the flow model and T4d's `build` command; check this todo against the decision
- [ ] `laminapdf-build`: the `serde` and `schema` features (optional `serde`, `serde_json` and `schemars` from `[workspace.dependencies]`, as `laminapdf-extract` has them), the derives, the `schema` key, and fonts and images by name
- [ ] The schema test in `crates/laminapdf-build/tests/document_schema.rs` (new): writes the schema with `UPDATE_GOLDEN=1`, fails when it is stale, and validates every fixture document against it with `boon`, as the façade's tests do
- [ ] `flow/template.rs` (or the module ADR 0032 names): `fill` with `$data`, `$each`, `$item` and `$if`; the three new codes with their `codes.json` entries and `docs/errors/` pages, then `cargo xtask gen codes`
- [ ] The façade: `laminapdf::flow::fill`, behind the façade feature that turns on `laminapdf-build`'s `serde` (decide in review whether `text` implies it)
- [ ] The CLI: `.json` inputs in `commands/build.rs`, `--data` (standard input once, as ADR 0031 §4 allows), `--font NAME=FILE` and `--image NAME=FILE`; the schema in `schemas/build.schema.json` regenerated
- [ ] MCP: `document` and `data` on `laminapdf_build`; the inlined schema under `$defs`, checked by `every_reference_should_resolve_inside_its_schema`
- [ ] WASM: `WasmBuilder.flow`; TypeScript: `Builder.flow`, the generated `document.ts` (`cargo xtask gen boundary`), and a `@ts-expect-error` test for a misspelt key; `cargo xtask wasm` and `cargo xtask size`
- [ ] The fixtures with their provenance rows in `fixtures/README.md`, the example pair, the goldens, the runtime suite entry and the timing in `measure()`
- [ ] The fuzz target: `fuzz/fuzz_targets/document.rs`, its `[[bin]]` in `fuzz/Cargo.toml`, `FUZZ_TARGETS` in `tools/xtask/src/harness.rs`, seeds from the fixtures
- [ ] The docs (Docs to update)

## Acceptance criteria

- [ ] `document.schema.json` is committed and current, and every fixture document validates against it
- [ ] `invoice_from_json` writes the golden's bytes in Rust and in every runtime (`cargo xtask pdf-runtimes`): two pages, the table header on both, the totals on the same page as the last body row; it passes `cargo xtask verapdf` under PDF/UA-1 and `qpdf --check`
- [ ] `laminapdf build fixtures/documents/invoice.json --data fixtures/documents/invoice-data.json` writes the same bytes, and so does `--data -` with the data on standard input
- [ ] A misspelt key fails with `invalid-document` and a `didYouMean` in Rust and the CLI, with `unknown-option` and a `didYouMean` in the binding, and fails to type-check in TypeScript
- [ ] A pointer with no value fails with `template-data-missing` and the pointer in its `data`; with a marker string in the data and the template, no message or hint contains it
- [ ] `crates/laminapdf-cli/tests/mcp.rs`: `laminapdf_build` builds the invoice from inline `document` and `data` inside a root
- [ ] The runtime suite's measurements include the `invoice-from-json` case in each runtime
- [ ] `cargo xtask gen boundary --check` and `cargo xtask size` pass (with a budget amendment if needed); the `document` fuzz target runs in the full lane without a crash

## Post-merge checks

- [ ] The nightly `pdf-runtimes` job records `pdf/<runtime>/invoice-from-json` in `history.jsonl`, and the nightly fuzz job runs the `document` target for its 10 minutes without a crash

## Tests

- `crates/laminapdf-build/tests/document_schema.rs` (new): the schema, round trips (a `Flow` written as JSON and read back is the same `Flow`), unknown keys.
- Unit tests in the template module for each placeholder, nesting `$each` in `$each`, and each error.
- `crates/laminapdf-cli/tests/`: `build` with `.json` inputs and `--data`, and `mcp.rs` for the tool's new arguments.
- `crates/laminapdf/tests/examples.rs` and `packages/sdk/test/suite/examples.ts` for the example pair; a TypeScript type test for the generated types.
- The `document` fuzz target.

## Docs to update

- [ ] A recipe on the docs site, "Fill an invoice from JSON" (`apps/docs/src/content/docs/recipes/invoice-from-json.mdx`, new), with both examples and the CLI command
- [ ] The "Pages that flow" guide: the TypeScript tab for every section, and the JSON form
- [ ] `packages/sdk/examples/AGENTS.md` and the agent docs: the schema's URL, so coding agents can check what they write
- [ ] `crates/laminapdf-cli/README.md`; `CLAUDE.md`'s Building API, WASM and TypeScript paragraphs and the fuzz target list
- [ ] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **The schema becomes a public contract.** Every flow option it exposes is hard to rename later. Keep `/1` to what T4b–T4d ship, and mark nothing as experimental inside it.
- **A mini template language.** Placeholders without expressions are safe and simple, but users will ask for sums, dates and currency. The ADR's answer (formatted data in) has to be stated plainly in the recipe.
- **The schema converter.** `schema_ts.rs` knows only the keywords the extract schema uses, and fails on any other. It handles `oneOf` and `const`, which tagged unions need; check what schemars writes for the flow model before relying on that.
- **MCP tokens.** Inlining the document schema makes the tool's `inputSchema` large. Measure it; if it is too large for clients, reference the schema by its `$id` instead.
- **WASM size,** as in T4d: the owner decides any budget increase.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
