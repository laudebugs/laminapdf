# L4e — CLI `render` to PNG, output templates, and MCP tools for `render` and the extraction formats

| Field | Value |
| --- | --- |
| ID | `L4e` |
| Status | `ongoing` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L4b](../done/L4b-cli-batch-render-schema.md), [C2b](../done/C2b-lamina-render-crate.md) |
| Blocks | None |
| ADRs | Follows 0031 (its batch amendment, from [L4b](../done/L4b-cli-batch-render-schema.md)), [0004](../../decisions/0004-render-backend.md) (backends), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (new dependencies) |
| Branch | `feat/l4e-cli-render` |
| PR title | `feat(cli): render pages to PNG, output templates, MCP tools for render and extraction` |
| Size | M |
| PR | — |

> **Split from [L4b](../done/L4b-cli-batch-render-schema.md)**, which shipped batch mode, the extraction formats, `schema`, completions and man pages; this todo takes the rest of its scope.

## Goal

Add `laminapdf render`, which turns pages into PNG files through `laminapdf-render`, with the output templates that let one command write many files. Then give `laminapdf mcp` tools for `render` and for `extract`'s Markdown, JSON and chunk formats.

## Context

- [L4b's todo](../done/L4b-cli-batch-render-schema.md): the original scope, acceptance criteria and tests for `render`, and the batch runner it built (`crates/laminapdf-cli/src/batch.rs`).
- [C2b](../done/C2b-lamina-render-crate.md): the `RenderBackend` trait, the hayro adapter (pure Rust, feature `hayro`) and the PDFium adapter (a native library at run time).
- ADR 0031's batch amendment: NDJSON lines, exit codes. ADR 0004: which backend ships where.

## Scope

**In scope**

- **`render <inputs…> [--pages RANGE] [--dpi N | --scale F] [--backend NAME] -o TEMPLATE`**: one PNG per page; `-o -` only for a single page; `--json` reports `laminapdf.render/1` (pages, pixel sizes, output paths) with a committed schema; the backend by name from what C2b provides (hayro in every build; PDFium when `LAMINA_PDF_PDFIUM_DIR` names a library).
- **Output templates:** `-o 'out/{stem}-{n:03}.png'` with `{stem}`, `{name}`, `{index}`, `{n}` (page number) and a width; two outputs with the same path is a usage error before any file is written. Recorded in ADR 0031.
- **PNG encoding** with the workspace's `png` crate (MIT OR Apache-2.0).
- **MCP:** a `laminapdf_render` tool (behind `--allow-write`, writing inside the roots) and the extraction formats in `laminapdf_extract` (today the tool answers only `format: text`), with their `outputSchema`s.
- `render` in `laminapdf schema` and in the batch runner.

**Out of scope** (and which todo covers it)

- SVG output: later, when `laminapdf-render` supports it.
- Distribution: [L4d](../ready/L4d-cli-distribution.md).

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`
- [ ] The template parser and the duplicate-output check, with unit tests
- [ ] `render` over `laminapdf-render` and PNG encoding; its report and schema
- [ ] The MCP tools; `tests/mcp.rs` cases
- [ ] ADR 0031 (templates), the CLI README, CLAUDE.md

## Acceptance criteria

- [ ] `render` output for the B5c examples matches the render goldens within the C2b thresholds
- [ ] A duplicate output path exits 2 before any file is written; templates expand as documented
- [ ] `laminapdf_extract` answers every format, and `laminapdf_render` writes only inside the roots
- [ ] `cargo xtask check` passes, including cargo-deny

## Post-merge checks

None.

## Tests

`crates/laminapdf-cli/tests/render.rs` (PNG goldens in `fixtures/render-golden/cli/`, low DPI, templates, batch), unit tests for the template parser, `tests/mcp.rs` for the new tools.

## Docs to update

- [ ] ADR 0031 (output templates)
- [ ] `crates/laminapdf-cli/README.md`; `CLAUDE.md`
- [ ] [Shareable libraries plan §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1)

## Definition of done

- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ready/` → `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
