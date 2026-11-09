# T4i — Markdown from the command line, MCP and TypeScript: `laminapdf build`, `laminapdf_build` and `builder.markdown`

| Field | Value |
| --- | --- |
| ID | `T4i` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4d](../done/T4d-markdown-documents.md) |
| ADRs | Implements [ADR 0032 §7](../../decisions/0032-flowing-layout-and-document-inputs.md#7-surfaces) (the CLI verb, the MCP tool and the TypeScript call) and [§9](../../decisions/0032-flowing-layout-and-document-inputs.md#9-budgets) (the text module's 1,280,000 B gzip budget). Follows [0031](../../decisions/0031-cli-and-mcp-contract.md) as ADR 0032 amends it: [§2](../../decisions/0031-cli-and-mcp-contract.md#2-grammar) (`build`), [§6](../../decisions/0031-cli-and-mcp-contract.md#6-json-output) (`laminapdf.build/1`) and [§9](../../decisions/0031-cli-and-mcp-contract.md#9-mcp-defaults-for-l4c) (a write tool under `--allow-write`); [0016 §6–§7](../../decisions/0016-package-design.md#7-api-naming-and-shape-at-the-wasm-boundary) (module budgets; records as JSON with `unknown-option`) and [0026 §4](../../decisions/0026-error-and-diagnostic-contract.md#4-no-document-text-in-message-hint-or-didyoumean) (no document text in messages) |
| Branch | `feat/t4i-build-command-and-bindings` |
| PR title | `feat(cli): laminapdf build for Markdown, its MCP tool and the TypeScript markdown call` |
| Size | L (≈ 550–700 lines: the command and its report, the MCP tool, the WebAssembly and TypeScript calls, the TypeScript example; schemas and goldens don't count) |
| PR | — |

## Goal

Bring [T4d](../done/T4d-markdown-documents.md)'s Markdown front end to the places people and agents call it from: `laminapdf build report.md -o report.pdf`, an MCP tool that a coding agent can call with the Markdown inline, and `builder.markdown(text, options)` in `@laminapdf/sdk`. Markdown is the largest generation request on Hacker News, about 1,140 comments, and one commenter asked for exactly this: "if you have a markdown->PDF function included, where I can send in markdown and get PDF, that would solve quite many needs" ([46325132](https://news.ycombinator.com/item?id=46325132)). Talk of AI near generation rose from 6 to 263 comments, and an MCP tool is how an agent reaches the library without writing code.

## Context

- [ADR 0032 §7](../../decisions/0032-flowing-layout-and-document-inputs.md#7-surfaces) fixes the surfaces: `laminapdf build <input.md | doc.json | -> -o OUT` with `--font ROLE=FILE`, `--data FILE`, `--page-size`, `--profile`, `--lang` and `--title`; `--json` prints `laminapdf.build/1` (output, page count, bytes, tagged, profiles, warnings); `laminapdf_build`, listed only with `--allow-write`, takes the input inline or as a path and writes inside the roots; `builder.markdown(text, options)` takes fonts as `Font`s and images as `Image`s by name. This todo builds the Markdown half; [T4e](./T4e-json-documents-and-templates.md) adds `doc.json` and `--data`.
- [ADR 0032 §5](../../decisions/0032-flowing-layout-and-document-inputs.md#5-placement) puts the front end in the text module, and [§9](../../decisions/0032-flowing-layout-and-document-inputs.md#9-budgets) raised that module's budget from 1,200,000 to 1,280,000 B gzip to cover the flow layer, tables, notes, Markdown and templates together. [§8](../../decisions/0032-flowing-layout-and-document-inputs.md#8-fonts): without fonts the output is Latin only and PDF/UA-1 refuses it, which the CLI's hint names with `--font`.
- The CLI ([L4a](../done/L4a-lamina-cli.md), [L4b](../done/L4b-cli-batch-render-schema.md)): one function per command in `crates/laminapdf-cli/src/commands/` over `Settings` and `&mut dyn Io`, the clap `Command` enum in `src/main.rs`, `schema::all()` and `COMMANDS` in `src/schema.rs` (schemas rewritten by `UPDATE_GOLDEN=1 cargo test -p laminapdf-cli --test json_schemas`), and `Io::read`, which reads paths and `-`.
- MCP ([L4c](../done/L4c-mcp-mode.md)): `tools(allow_write, allow_in_place)` in `src/mcp/tools.rs` (in name order; `laminapdf_pages_rotate` and `laminapdf_render` are the write tools today) and `RootedIo` in `src/mcp/roots.rs`, which refuses paths outside every root (`path-outside-roots`). This tool answers [trust and proof §9](../../plans/trust-and-proof.md#9-open-questions) question 2.
- WebAssembly and TypeScript: `WasmBuilder` (`crates/laminapdf-wasm/src/builder/document.rs`), records as JSON with `unknown-option` ([ADR 0016 §7](../../decisions/0016-package-design.md#7-api-naming-and-shape-at-the-wasm-boundary)), the SDK's `Builder` (`packages/sdk/src/builder.ts`), and the runtime suite (`packages/sdk/test/suite/examples.ts`), which [B6c](../done/B6c-runtime-matrix.md) runs in every runtime.
- Examples: the [examples guideline](../../guidelines/examples.md) and the TypeScript–Rust pairs in `tools/xtask/docs-snippets.toml`. T4d's `markdown_to_pdf.rs` is the Rust twin.

## Scope

**In scope**

- **The CLI:** `laminapdf build <input.md | -> -o OUT`, with `--font ROLE=FILE` (repeatable), `--page-size`, `--profile`, `--lang` and `--title`.
  - Images are read through `Io` from paths relative to the input's folder.
  - `--json` prints `laminapdf.build/1` (output, page count, bytes, tagged, profiles, warnings), committed as `crates/laminapdf-cli/schemas/build.schema.json`.
  - `--dry-run` builds and reports without writing; `laminapdf schema build` prints the schema.
  - Exit codes and overwrites follow ADR 0031 §4–§5. For a `missing-glyph` or `font-not-embedded` error, the hint names `--font`, as ADR 0031 §8 asks.
  - Output is deterministic: no creation date unless `--creation-date` is given.
- **MCP:** `laminapdf_build`, listed with `--allow-write` only. It takes the Markdown inline (`markdown`) or from a path (`input`), writes `output` inside the roots, reads images through `RootedIo`, never overwrites and never fetches.
- **WebAssembly and TypeScript:** `WasmBuilder.markdown(text, options)` in the text module, with a tsify options record, and `builder.markdown(text, options)` in `@laminapdf/sdk`, with fonts as `Font`s and images as `Image`s by name. `cargo xtask size` passes within ADR 0032 §9's 1,280,000 B.
- **The TypeScript example:** `packages/sdk/examples/markdown-to-pdf.ts`, paired with T4d's `markdown_to_pdf.rs` in `tools/xtask/docs-snippets.toml`, writes the same golden bytes and joins the runtime suite.

**Out of scope** (and which todo covers it)

- `doc.json` inputs, `--data`, `--image NAME=FILE`, and the tool's `document` and `data` arguments: [T4e](./T4e-json-documents-and-templates.md).
- `builder.flow` and typed documents in TypeScript: [T4j](./T4j-documents-in-typescript.md).
- Several inputs in one run (ADR 0031's batch mode): later, if users ask.
- Fonts bundled with the CLI or the SDK: [pdf-library §9](../../plans/pdf-library.md#9-open-questions) question 2, which ADR 0032 §8 leaves open.

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`, and `/writing` for the docs pages
- [ ] Read ADR 0032 §7–§9 and T4d's `MarkdownOptions`
- [ ] The CLI: `commands/build.rs` (new; `build(args, &Settings, &mut dyn Io)`, which never prints), `BuildArgs` and `BuildReport` with `JsonSchema`, the `Build` variant in `src/main.rs`, `schema::all()` and `COMMANDS`, the committed schema, CLI hints for the font errors, and the CLI README
- [ ] MCP: `laminapdf_build` in `src/mcp/tools.rs` behind `allow_write`, with annotations that say it writes and isn't destructive; update `the_tools_should_be_in_name_order_and_read_only_by_default` for the new order
- [ ] WebAssembly: `WasmBuilder.markdown` with its options record, the `markdown` feature turned on in the text module's build; TypeScript: `Builder.markdown` and its types; `cargo xtask wasm` and `cargo xtask size`
- [ ] The TypeScript example, its pair entry and the runtime suite entry
- [ ] The docs (Docs to update)

## Acceptance criteria

- [ ] `laminapdf build fixtures/markdown/report.md` with the fixture fonts writes T4d's golden bytes; its `--json` report validates against `build.schema.json`; an existing output without `--force` exits 4; raw HTML exits 3 with `markdown-html-ignored` (0 with `--warnings-ok`)
- [ ] Without `--font`, PDF/UA-1 fails with `font-not-embedded`, and its hint names `--font`
- [ ] The CLI reads an image from a path relative to the input's folder; under MCP, an image or output path outside every root fails with `path-outside-roots`
- [ ] `crates/laminapdf-cli/tests/mcp.rs`: `laminapdf_build` builds from inline Markdown inside a root, and is absent without `--allow-write`
- [ ] `markdown-to-pdf.ts` writes the golden's bytes in every runtime (`cargo xtask pdf-runtimes`)
- [ ] `cargo xtask size` passes within the text module's 1,280,000 B gzip budget
- [ ] `cargo xtask check docs` passes with the example pair shown on a docs page

## Post-merge checks

- [ ] The nightly `pdf-runtimes` job runs the Markdown example in every runtime

## Tests

- `crates/laminapdf-cli/tests/`: `build` end to end (golden bytes, `--json` against the schema, `--dry-run`, exit codes, the font hint) and `mcp.rs` for the tool.
- `packages/sdk/test/suite/examples.ts` for the TypeScript example, in every runtime.
- A `laminapdf-wasm` test for an unknown key in the options record (`unknown-option`).

## Docs to update

- [ ] A recipe on the docs site, "Turn Markdown into a PDF" (`apps/docs/src/content/docs/recipes/markdown-to-pdf.mdx`, new), with both examples and the CLI command
- [ ] The "Pages that flow" guide: the TypeScript tab and the CLI command in T4d's Markdown section
- [ ] `crates/laminapdf-cli/README.md` (the `build` command); `packages/sdk/examples/AGENTS.md`, so coding agents know the format and the tool
- [ ] `CLAUDE.md`: the CLI, WASM and TypeScript paragraphs and the commands list
- [ ] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **Size.** If the PR passes about 800 lines, split it here: the CLI and MCP first, then the WebAssembly and TypeScript calls.
- **WASM size.** The budget covers the whole flow layer and both inputs. If Markdown alone leaves too little room for T4e's and T4j's additions, record the figure in the PR so T4j can plan around it.
- **Fonts.** Without `--font`, only Latin text sets, and PDF/UA-1 fails. The CLI's hints must say so plainly.
- **Untrusted Markdown from agents.** Images are read only through `Io` (inside the roots under MCP) and nothing is fetched. MCP's `--timeout` stops a run that takes too long.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
