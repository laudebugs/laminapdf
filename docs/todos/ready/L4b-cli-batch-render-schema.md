# L4b — CLI batch mode, `render`, `extract --format md|json`, `schema`, completions and man pages

| Field | Value |
| --- | --- |
| ID | `L4b` |
| Status | `ready` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L3b](../ongoing/L3b-extract-tables-and-serialisers.md), [L4a](../done/L4a-lamina-cli.md), [C2b](../done/C2b-lamina-render-crate.md) |
| Blocks | None |
| ADRs | Implements 0031 (batch and NDJSON rules, from [L4a](../done/L4a-lamina-cli.md)). Follows 0026 (errors), 0027 (coordinates in output), 0030 (`laminapdf.extract/1`, from [L3a](../done/L3a-lamina-extract-reading-order.md)), [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits`), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (new dependencies) |
| Branch | `feat/l4b-cli-batch-render-schema` |
| PR title | `feat(cli): batch mode with --jobs and NDJSON, render, extract md/json, schema, completions and man pages` |
| Size | L (≈ 500–800 lines; generated schemas, completions, man pages and PNG goldens don't count) |
| PR | — |

## Goal

Make the CLI useful at scale and self-describing for agents. Commands accept many inputs and run them in parallel with `--jobs`, printing one NDJSON result per file in argument order and a summary line. `render` turns pages into PNG through `laminapdf-render`. `extract --format md|json` emits the `laminapdf.extract/1` output from `laminapdf-extract`. `schema [command]` prints the JSON Schema 2020-12 of each command's input and output, generated from the same Rust types the commands use, so agents and the MCP mode never drift from the CLI. Shell completions and man pages come from the clap definitions.

## Context

- [Shareable libraries plan §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp): deliverable and acceptance.
- [Report: grammar, pipes and batch](../../research/libraries/shareable-libraries.md#borrow-qpdfs-contracts-mutools-verbs-and-pdftks-page-algebra): ripgrep-style NDJSON with a final `summary`, argument order, `--fail-fast`, `--files-from` with `-0`.
- [CLI spec note §3](../../research/libraries/notes/cli-spec.md#3-batch-and-parallel-processing): batch and parallel processing precedents.
- [CLI spec note §4](../../research/libraries/notes/cli-spec.md#4-agent-readiness-schemas-mcp-security-what-agents-need): `schema` from `schemars`, reused as MCP `inputSchema`/`outputSchema`; token economy (`--fields`, `--max-chars`).
- [CLI spec note §5](../../research/libraries/notes/cli-spec.md#5-rust-implementation-and-distribution): clap_complete, clap_mangen, indicatif, schemars versions.
- ADR 0031 (from [L4a](../done/L4a-lamina-cli.md)): exit codes and JSON rules this PR must follow.
- [C2b](../done/C2b-lamina-render-crate.md): the `RenderBackend` trait and its backends. [L3b](../ongoing/L3b-extract-tables-and-serialisers.md): Markdown, JSON and chunk serialisers.

## Scope

**In scope**

- **Batch over many inputs** for `info`, `check`, `extract` and `render`:
  - Inputs from argv, `--files-from FILE|-` (newline-separated, or NUL-separated with `-0`).
  - `--jobs N` (default: available parallelism, capped at 8; `--jobs 1` is sequential). Work runs in parallel, but output is printed in argument order (a reorder buffer keyed by index).
  - With `--json` and more than one input, output is NDJSON: one `{"type":"result","schema":…,"index":…,"input":…,"status":"ok"|"warnings"|"error",…}` per input, then `{"type":"summary","ok":…,"warnings":…,"failed":…,"elapsed_ms":…}`. Per-input errors are `result` lines with an `error` object in the ADR 0026 shape (data, on stdout).
  - Exit code for a batch: `0` all ok; `3` some warnings, no failures; `1` at least one input failed (partial failure); `5` only if every failure was a password failure; `2` usage errors before any work. Recorded in ADR 0031 by this PR (amendment section).
  - `--fail-fast`: stop scheduling new inputs after the first failure; the summary counts skipped inputs.
  - Outputs for many inputs use a template: `-o 'out/{stem}-{n:03}.png'` (`{stem}`, `{name}`, `{index}`, `{n}` for page number). Two inputs writing the same path is a usage error before any work starts.
  - Progress on stderr with indicatif only when stderr is a TTY and `--quiet` is not set.
- **`render <inputs…> [--pages RANGE] [--dpi N | --scale F] [--backend NAME] -o TEMPLATE`**: PNG per page through `laminapdf-render`; `-o -` allowed only for a single page. `--json` reports `laminapdf.render/1` (pages, pixel sizes, output paths). Backend selection by name from what C2b provides.
- **`extract <inputs…> --format text|md|json|chunks [--pages RANGE] [--max-chars N] [--visible-only]`**: `md` and `json`/`chunks` from `laminapdf-extract` (L3b) in `laminapdf.extract/1`, with bboxes in top-left points (ADR 0027) (L4a's `extract` already takes `--format text`; this PR adds the other values, same grammar). Hidden text is flagged in JSON and never written in Markdown (per L3).
- **`schema [command]`**: prints a JSON Schema 2020-12 document for the command's input (flags as an object) and output (`--json` shape), plus `laminapdf.error/1` and the NDJSON `result`/`summary` lines. With no argument, an index of all schema ids. Generated at run time from the `schemars` derives in `commands`; the committed schemas from L4a become a check that `laminapdf schema` matches them.
- **Completions** with `clap_complete` (`laminapdf completions bash|zsh|fish|powershell|elvish`) and **man pages** with `clap_mangen` generated by `cargo xtask cli-docs` into `target/cli-docs/` (packaged by [L4d](./L4d-cli-distribution.md), not committed).

**Out of scope** (and which todo covers it)

- `laminapdf mcp` and its tools for these commands: [L4c](../done/L4c-mcp-mode.md) (adds `render` and `extract` tools when this merges).
- SVG render output: later, when `laminapdf-render` supports it.
- Batch over write commands (`pages rotate` on many files): add with the E2 page commands; this PR's batch runner is generic enough to take them.
- Server-side batch: [E14e](./E14e-server-batch.md).

## Tasks

- [ ] Load `/rust-best-practices`; follow [docs/guidelines/rust.md](../../guidelines/rust.md). Batch scheduling is `app`-layer code: threads are fine here, never in core crates.
- [ ] Add a `batch` module in `crates/laminapdf-cli`: input expansion (`--files-from`, `-0`), a worker pool (`std::thread::scope` with a channel, or `rayon`; recommendation: `std::thread::scope` to avoid a dependency), an ordered emitter, the summary and the batch exit-code rule.
- [ ] Make `info`, `check` and `extract` batch-capable through the runner; one input keeps L4a's single-document output.
- [ ] Add the output template parser and the duplicate-output check.
- [ ] Add `render` over `laminapdf-render` (C2b) with PNG encoding (use what C2b already depends on, or `png`, MIT/Apache).
- [ ] Add `extract --format md|json|chunks` over the façade's `Document::extract` (L3b, `extract` feature on); `--max-chars` truncates on block boundaries and says so in JSON (`"truncated": true`).
- [ ] Add `schema` from the `schemars` derives; make the L4a schema test compare against `laminapdf schema` output.
- [ ] Add `completions` (clap_complete) and `cargo xtask cli-docs` (clap_mangen); add `indicatif` for progress.
- [ ] New dependencies: `clap_complete`, `clap_mangen`, `indicatif` (and `png` if needed). Expected MIT and/or Apache-2.0; confirm with `cargo xtask deny` (ADR 0019).
- [ ] Amend ADR 0031 with the batch exit-code rule and the output template syntax.

### Example sessions

```console
$ laminapdf check docs/*.pdf --jobs 8 --json | jq -c 'select(.type=="result" and .status!="ok")'
{"type":"result","schema":"laminapdf.check/1","index":1,"input":"docs/b.pdf","status":"error","error":{"code":"password-required","kind":"password"}}
$ fd -e pdf -0 | laminapdf info --files-from - -0 --json --fields pages | tail -1
{"type":"summary","ok":41,"warnings":0,"failed":1,"elapsed_ms":412}
$ laminapdf render report.pdf --pages 1-3 --dpi 72 -o 'thumbs/{stem}-{n:03}.png'
$ cat invoice.pdf | laminapdf extract - --format md > invoice.md
$ laminapdf schema extract | jq '.properties.output'
$ laminapdf completions zsh > ~/.zfunc/_lamina
```

## Acceptance criteria

- [ ] A batch of 50 corpus files with `--jobs 8` prints results in argument order (test with inputs that finish out of order, e.g. a large file first).
- [ ] NDJSON lines validate against the `result` and `summary` schemas; every line is one JSON object.
- [ ] Batch exit codes are tested: all ok → 0, warnings only → 3, one failure among many → 1, all password failures → 5, duplicate output path → 2 before any file is written.
- [ ] `--fail-fast` stops after the first failure and the summary counts the skipped inputs.
- [ ] `render` output for the B5c examples matches render goldens within the C2b thresholds.
- [ ] `extract --format json` validates against `laminapdf.extract/1`; `--format md` contains no hidden-text fixture strings.
- [ ] `laminapdf schema <command>` output is valid JSON Schema 2020-12 (checked with a meta-schema validator) for every command.
- [ ] `laminapdf completions bash` output is accepted by `bash -n`; `cargo xtask cli-docs` writes one man page per command.
- [ ] `cargo xtask check` passes, including cargo-deny.

## Post-merge checks

None.

## Tests

- `crates/laminapdf-cli/tests/batch.rs` (ordering, NDJSON, exit codes, `--fail-fast`, `--files-from`, `-0`), `render.rs` (PNG goldens in `fixtures/render-golden/cli/`, low DPI), `extract.rs` (schema validation, hidden text), `schema.rs` (meta-schema validation, match with committed schemas), `completions.rs`.
- Unit tests for the output template parser and the ordered emitter.

## Docs to update

- [ ] [Shareable libraries plan §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1): tick "Batch and NDJSON, `--jobs`, `render`, …"
- [ ] ADR 0031 amendment (batch exit codes, templates)
- [ ] `crates/laminapdf-cli/README.md` (batch, render, extract, schema, completions); `CLAUDE.md` (`cargo xtask cli-docs`)

## Risks and open questions

- **Memory with `--jobs`:** eight large PDFs at once can exceed memory. Recommendation: cap concurrent bytes as well as jobs (e.g. `--max-inflight-bytes`, default 1 GiB) and say so in the summary when it throttled.
- **Render backend in the CLI binary:** PDFium needs a native library at run time; hayro is pure Rust. Which backend ships in release binaries is decided by ADR 0004 (C2a) and affects L4d's binary size.
- **`--jobs` default:** capped at 8 to stay polite in CI; revisit with measurements.
- **Schema for flags:** describing clap flags as a JSON object is our own convention; L4c must use exactly the same mapping for MCP `inputSchema`.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
