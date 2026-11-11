# T4k — JSON documents from the command line and MCP: `laminapdf build doc.json --data`

| Field | Value |
| --- | --- |
| ID | `T4k` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4e](../ongoing/T4e-json-documents-and-templates.md) |
| Dependency notes | T4e split its CLI and MCP inputs off into this todo, as its risk note allowed: the Rust format, templates, example and fuzz target came to about 1,600 lines on their own. |
| ADRs | Implements [ADR 0032 §7](../../decisions/0032-flowing-layout-and-document-inputs.md#7-surfaces) (`--data` and the MCP tool's document input); follows [0031](../../decisions/0031-cli-and-mcp-contract.md) as ADR 0032 amends it, and its [§6](../../decisions/0031-cli-and-mcp-contract.md#6-json-output) versioning rule |
| Branch | `feat/t4k-json-documents-cli-mcp` |
| PR title | `feat(cli): JSON documents and templates from laminapdf build and its MCP tool` |
| Size | M (≈ 300–450 lines: the CLI inputs, the MCP arguments and their inlined schema, tests, docs) |
| PR | — |

## Goal

Build a `laminapdf.document/1` document, or fill a template, without writing code: `laminapdf build invoice.json --data items.json -o invoice.pdf`, and the same from an agent through `laminapdf_build`. T4e made the format, its templates and `Builder::document` in Rust; this todo brings them to the command line and MCP, beside T4i's Markdown.

## Context

- [T4e](../ongoing/T4e-json-documents-and-templates.md): `laminapdf::flow::document::{FlowDocument, Resources, fill}`, `Builder::document`, `FlowDocument::profiles`, the schema `crates/laminapdf-build/schemas/document.schema.json` and `flow::document::json_schema()`, and the fixtures `fixtures/documents/invoice.json` and `invoice-data.json` with the golden `fixtures/golden/examples/invoice_from_json.pdf`.
- [T4i](../done/T4i-build-command-and-bindings.md): `crates/laminapdf-cli/src/commands/build.rs` (`BuildArgs`, `BuildToolArgs`, `build`, `build_tool`, images read through `Io` only from the input's folder or below it, inline Markdown reading no image file), `laminapdf.build/1` and the `laminapdf_build` tool.

## Scope

**In scope**

- **The CLI:** `laminapdf build doc.json [--data FILE|-] -o OUT`. A `.json` input, or `-` whose JSON has `schema` `laminapdf.document/1`, is a document; `--data` fills it as a template (standard input once, as ADR 0031 §4 allows). `--font NAME=FILE` and `--image NAME=FILE` give the files the document names (`FILE#N` for a face of a collection); a name it isn't given is read from a file of that name next to the document, inside its folder only, as T4i reads images. The document's `profiles` go to the builder; `--profile` adds to them. `laminapdf.build/1` gains keys only.
- **MCP:** `laminapdf_build` takes `document` and `data` inline, beside `markdown` and `input`, with `fonts` and `images` by name inside the roots; an inline document reads no file it isn't given by name. Its `inputSchema` inlines `laminapdf.document/1` under `$defs`, as `laminapdf.error/1` is inlined, checked by `every_reference_should_resolve_inside_its_schema`. Measure the tool's `inputSchema` size; if clients find it too large, reference the schema by its `$id` instead.
- **Hints:** `invalid-document`, `template-data-missing` and `template-data-type` keep their pointer in `data`; the CLI's text output names the pointer.

**Out of scope**

- The format, templates and Rust API: [T4e](../ongoing/T4e-json-documents-and-templates.md).
- TypeScript: [T4j](./T4j-documents-in-typescript.md).

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`, and `/writing` for the docs
- [ ] `commands/build.rs`: JSON inputs, `--data`, `--font NAME=FILE`, `--image NAME=FILE`, names read from the document's folder; `schemas/build.schema.json` regenerated
- [ ] MCP: `document`, `data`, `fonts`, `images` on `laminapdf_build`; the inlined schema
- [ ] Tests (below) and docs (below)

## Acceptance criteria

- [ ] `laminapdf build fixtures/documents/invoice.json --data fixtures/documents/invoice-data.json --font regular=fixtures/fonts/NotoSans-Regular.ttf --font bold=fixtures/fonts/NotoSans-RegularBold.ttc#1` writes the bytes of `invoice_from_json`'s golden with the example's options, or a CLI golden of its own when the options differ (as T4i's did); so does `--data -` with the data on standard input
- [ ] A misspelt key exits 2 with `invalid-document`, its pointer and a `didYouMean`
- [ ] A pointer with no value exits 2 with `template-data-missing` and the pointer
- [ ] `crates/laminapdf-cli/tests/mcp.rs`: `laminapdf_build` builds the invoice from inline `document` and `data` inside a root, and an inline document reads no file it isn't given

## Tests

`crates/laminapdf-cli/tests/build.rs` (JSON inputs, `--data` and `--data -`, the errors), `tests/mcp.rs`, the tool schema tests in `src/mcp/tools.rs`.

## Docs to update

- The "Fill an invoice from JSON" recipe: the CLI command
- `crates/laminapdf-cli/README.md`; `packages/sdk/examples/AGENTS.md` (the MCP tool's document input); `CLAUDE.md`'s CLI paragraph

## Risks and open questions

- **Untrusted documents from agents:** an inline document names fonts and images; it reads only the files it is given by name, never a path it writes itself.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
