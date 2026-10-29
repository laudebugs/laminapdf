# L4a — ADR 0031 and `crates/lamina-cli`: `info`, `check`, `pages rotate`, `extract`, `explain`

| Field | Value |
| --- | --- |
| ID | `L4a` |
| Status | `ready` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [B5c](./B5c-lamina-facade-and-examples.md), [B9c](./B9c-text-extraction-bindings-and-differential.md), [L1b](./L1b-error-contract-codegen.md) |
| Blocks | [L4b](./L4b-cli-batch-render-schema.md), [L4c](./L4c-mcp-mode.md), [L4d](./L4d-cli-distribution.md) |
| ADRs | Writes **0031** (CLI and MCP contract). Follows 0026 (error shape, from [L1a](./L1a-contract-adrs.md)), 0027 (coordinates and page indexing), [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits`, warnings, strict mode), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (new dependencies), [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (crate layout, xtask) |
| Branch | `feat/l4a-lamina-cli` |
| PR title | `feat(cli): ADR 0031 and the lamina CLI with info, check, pages rotate, extract and explain` |
| Size | L (≈ 600–900 lines: ADR, command layer, clap front end, tests; generated schemas and test fixtures don't count) |
| PR | — |

## Goal

Give shell users, CI pipelines and AI agents one binary, `lamina`, with a stable machine contract. This PR writes ADR 0031, which fixes the grammar, page ranges, stdin/stdout handling, exit codes, JSON versioning and the MCP security defaults. It then ships the first commands on top of the `lamina` façade: `info`, `check`, `pages rotate`, `extract text` and `explain`. Each command has `--json` output with a schema id and errors in the ADR 0026 shape. The command logic lives in a library layer separate from the clap front end, so `lamina mcp` ([L4c](./L4c-mcp-mode.md)) can call the same functions. The research says the CLI, not MCP, is the primary agent interface, so its contract must be right before more commands pile on.

## Context

- [Shareable libraries plan §4](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) (one error shape, explicit coordinates, versioned schemas) and [§7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) (deliverable and acceptance).
- [Report: the CLI is the primary agent interface](../../research/libraries/shareable-libraries.md#the-cli-is-the-primary-agent-interface-and-mcp-is-a-thin-adapter-over-it): exit-code recommendation, grammar, one command layer under clap and rmcp.
- [CLI spec note §1](../../research/libraries/notes/cli-spec.md#1-command-grammars-of-existing-pdf-clis-qpdf-mutool-pdftk-cpdf-poppler-ghostscript): qpdf page ranges and exit codes, mutool verbs, pdftk handles, Ghostscript's interactive-by-default anti-pattern.
- [CLI spec note §2](../../research/libraries/notes/cli-spec.md#2-modern-cli-conventions-cligdev-12-factor-posix-exit-codes-env-vars-json): stdout for data, stderr for the rest, `-`, TTY and `NO_COLOR`, no prompts without a TTY.
- [CLI spec note §6](../../research/libraries/notes/cli-spec.md#6-proposed-command-set-with-example-sessions-recommendations): the proposed command set, example sessions and the error JSON.
- ADR 0026 (from [L1a](./L1a-contract-adrs.md)) and the generated error types and `explain` text from [L1b](./L1b-error-contract-codegen.md): the CLI's errors are these, not a new shape.
- [ADR 0003 §2](../../decisions/0003-reader-tolerance-and-limits.md#2-warnings-stable-enumerated-codes) and [§3](../../decisions/0003-reader-tolerance-and-limits.md#3-limits): warning codes drive exit code 3; `Limits` are CLI flags.
- [B3a](./B3a-overlay-and-ops.md): today the only page op is `RotatePage`; that limits `pages` in this PR.
- [Code organisation §2](../../guidelines/code-organization.md#2-boundaries-and-dependency-direction): `app` crates may use the file system and threads; core crates may not.

## Scope

**In scope**

- **ADR 0031** (`docs/decisions/0031-cli-and-mcp-contract.md`), status Accepted on merge:
  - Grammar: verb-first, `lamina <command> [<subcommand>] <inputs…> [-o OUT | --in-place] [--json]`. Global flags: `--json`, `--quiet`, `--no-input`, `--force`, `--dry-run`, `--color auto|always|never`, `--password-file FILE`, `--strict`, limit flags.
  - Page ranges: qpdf's grammar as the base (`1-5`, `z` last page, `r3` third from the end, `5-1` reverse, `x3` exclusion, `:odd`/`:even` on the resulting set) with `end` accepted as an alias of `z`. Uppercase `N` (mutool's last page) is **not** an alias, because uppercase letters are pdftk-style file handles (`A=a.pdf … A1-z`); a handle name is one or more uppercase letters and never collides with a range token. Ranges are 1-based page numbers, as users count; JSON output gives both `index` (0-based) and `number` (1-based), per ADR 0027.
  - File handles: pdftk style `A=fronts.pdf B=backs.pdf`, then `A1-5`, `Bz-1`; `file.pdf:1-z:odd` for an inline range. The grammar is fixed now, even though merge and shuffle land later.
  - Stdin and stdout: `-` as an input reads stdin (at most one `-` per command); `-o -` writes stdout. Binary output is never written to a stdout that is a TTY (usage error instead).
  - Exit codes: `0` ok; `1` failure; `2` usage error; `3` completed with warnings only (`--warnings-ok` maps it to 0); `4` refused a destructive action that needs `--force` (overwrite an existing file, `--in-place`); `5` password required or wrong. When several apply, the highest-priority wins: `2` > `5` > `4` > `1` > `3` > `0`. `lamina help exit-codes` prints the table.
  - JSON: every document has a `schema` id (`lamina.info/1`, `lamina.check/1`, `lamina.pages/1`, `lamina.text/1`, `lamina.explain/1`, `lamina.error/1`). New keys may appear within a version; consumers must ignore unknown keys; the number is bumped only on a breaking change (qpdf's rule). NDJSON (one document per line, `type` field) is the batch format, specified here and built in [L4b](./L4b-cli-batch-render-schema.md). Key casing follows ADR 0026.
  - Streams: data on stdout; progress, warnings and human diagnostics on stderr. Colour only when the stream is a TTY and `NO_COLOR` is unset or empty (`--color` overrides). Progress only when stderr is a TTY.
  - No prompts when stdin is not a TTY or `--no-input` is set; anything that would prompt fails with exit 4 or 5 and a hint. Passwords come from `--password-file` or `LAMINA_PASSWORD`, never from argv.
  - Errors: the ADR 0026 shape wrapped as `lamina.error/1`, plus CLI fields `exitCode` and `input`. Document text never appears in `message` or `hint`.
  - MCP security defaults (implemented in [L4c](./L4c-mcp-mode.md)): stdio only; `--root DIR` required (repeatable), paths canonicalised and symlink escapes rejected; read-only by default (`--allow-write` to list write tools, `--allow-in-place` on top of that to overwrite inputs); tools mapped 1:1 to CLI commands with the same schemas; honest annotations; document text only in data fields; validation errors as tool errors.
- **`crates/lamina-cli`**, layer `app` (`[package.metadata.lamina] layer = "app"`), binary name `lamina`:
  - `src/lib.rs` with `pub mod commands`: one function per command, `fn info(args: &InfoArgs, io: &mut dyn Io) -> Result<InfoReport, CliError>` style. Args and reports derive `serde` and `schemars::JsonSchema`; args also derive `clap::Args`. No clap types in reports, no printing in commands.
  - `src/io.rs`: an `Io` trait for reading inputs (`-`, path) and writing outputs (atomic write through a temporary file and rename in the target directory; refuse to overwrite without `--force`). A test double for unit tests.
  - `src/main.rs`: the clap front end only (parse, call the command, render human or JSON output, map to exit code).
  - `src/ranges.rs`: the page-range and handle parser, with errors that point at the bad character.
  - `src/exit.rs`: `ExitCode` enum and the priority rule; `CliError` wraps `lamina::Error` (from L1b) plus usage and I/O errors, all with a code and `kind`.
  - `src/render.rs`: human output (anstream, `NO_COLOR`), JSON and the `lamina.error/1` document.
- Commands:
  - `info <input> [--fields a,b] [--json]`: version, page count, page sizes (top-left points per ADR 0027), encryption, Info dictionary, tagged or not, counts of form fields and signatures if the façade exposes them, warnings. `--fields` keeps only the named top-level keys (gh-style, to save agent tokens).
  - `check <input> [--strict] [--json]`: open, resolve every object and page, decode every content stream within `Limits`; report warnings and errors; exit 3 on warnings only.
  - `pages rotate <input> <angle>[:<range>] [-o OUT | --in-place]`: `+90`, `-90`, `180`, absolute `90`; uses `RotatePage` and the B3c save planner (incremental save).
  - `extract <input> [--format text] [--pages RANGE] [--json]`: plain text per page from the façade's `page.text()`; with `--json`, `lamina.text/1` with one entry per page (`index`, `number`, `text`). `--format` is part of the grammar now so scripts never change: this PR accepts only `text` (the default); [L4b](./L4b-cli-batch-render-schema.md) adds `md`, `json` and `chunks`. There is no `extract text` subcommand, so a file named `text` is never ambiguous.
  - `explain <code> [--json]`: the L1b text for a warning or error code (`lamina::explain`, with the façade's `explain` feature on); unknown code is exit 2 with the list of close matches.
  - `help exit-codes` and `--version` (`--version --json` gives the CLI, core and schema versions).
- Limit flags mapped onto `Limits` (`--max-stream-decoded`, `--max-decoded-total`, …, names from `fixtures/codes.json` `limits`), and `--strict` onto strict mode.
- Committed JSON Schemas for each report and `lamina.error/1` under `crates/lamina-cli/schemas/`, generated from the Rust types by a test helper; a test fails if they are stale.

**Out of scope** (and which todo covers it)

- Several inputs per command, `--jobs`, NDJSON, `render`, `extract --format md|json`, the `schema` command, completions and man pages: [L4b](./L4b-cli-batch-render-schema.md).
- `lamina mcp`: [L4c](./L4c-mcp-mode.md).
- Release binaries, Homebrew and npm: [L4d](./L4d-cli-distribution.md).
- `pages select|delete|reorder|reverse` (need [E2b](./E2b-page-structure-ops.md)), `pages merge|split|shuffle` (need [E2c](./E2c-cross-document-pages.md) and [E2d](./E2d-merge-and-split.md)): a follow-up CLI todo is added when E2b is claimed. This PR parses the full range and handle grammar so those commands only add wiring.
- `fill`, `sign`, `verify`, `encrypt`, `optimize`, `diff`, `inspect`: later rounds, each with its feature.

## Tasks

- [ ] Load `/rust-best-practices`; read [docs/guidelines/rust.md](../../guidelines/rust.md) and [code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer). The `app` layer may use the file system, but errors still use `thiserror` and the `commands` library still avoids `unwrap`/`expect`/`panic`/indexing.
- [ ] Write ADR 0031 from the Scope list above; add it to `docs/decisions/README.md`. Cite the report and the CLI spec note for each choice; mark recommendations.
- [ ] Create `crates/lamina-cli` (`Cargo.toml` with workspace fields, lints, `layer = "app"`, `[[bin]] name = "lamina"`); add it to the workspace members.
- [ ] Add dependencies through `[workspace.dependencies]`: `clap` 4 (derive), `anstream`, `serde`, `serde_json`, `schemars` 1; dev: `assert_cmd`, `predicates`, `jsonschema` (validate output against schemas), `tempfile`. Expected licences are MIT and/or Apache-2.0; confirm with `cargo xtask deny` (ADR 0019).
- [ ] Implement `ranges.rs` with the qpdf grammar, the `end` alias, handles (uppercase only) and inline `file:range`; property tests (proptest) for "parsed range is within 1..=n" and reverse/exclusion cases.
- [ ] Implement `io.rs` (stdin, stdout, atomic write, overwrite check, TTY check for binary stdout).
- [ ] Implement `exit.rs` and `CliError`; map every `lamina::Error` code to an exit code (password codes → 5, `limit-exceeded` → 1, …) in one table with a test that every code in `fixtures/codes.json` is mapped.
- [ ] Implement the commands in `commands/` and the clap front end in `main.rs`; human output kept under ~20 lines for `info`.
- [ ] Implement `--password-file` and `LAMINA_PASSWORD` (decryption from B2b via the façade); never echo the password.
- [ ] Generate and commit schemas; add the staleness test.
- [ ] Update docs (below) and `CLAUDE.md` (layout and commands).

### Example sessions (to copy into ADR 0031 and the crate README)

```console
$ lamina info report.pdf
report.pdf  PDF 1.7 · 42 pages · 1.8 MB · not encrypted · tagged
warning[xref-repaired]: cross-reference table was rebuilt
$ echo $?
3
$ lamina info report.pdf --json --fields pages,encrypted,warnings
{"schema":"lamina.info/1","input":"report.pdf","pages":42,"encrypted":false,"warnings":[{"code":"xref-repaired","message":"cross-reference table was rebuilt"}]}
$ cat scan.pdf | lamina pages rotate - +90:1-3,x2 -o - > rotated.pdf
$ lamina pages rotate scan.pdf 90:z --in-place < /dev/null
error[overwrite-refused]: refusing to modify scan.pdf in place without --force
  hint: pass --force, or write to a new file with -o
$ echo $?
4
$ curl -sL https://example.org/a.pdf | lamina extract - --pages 1-3 | rg -i invoice
$ lamina explain xref-repaired
```

### Error JSON example (stderr, `--json`, exit 5)

```json
{
  "schema": "lamina.error/1",
  "error": {
    "code": "password-required",
    "kind": "password",
    "message": "the document is encrypted (AES-256, revision 6) and needs a user password",
    "hint": "pass --password-file FILE or set LAMINA_PASSWORD",
    "location": { "object": "12 0 R" },
    "docsUrl": "https://lamina.dev/errors/password-required",
    "input": "b.pdf",
    "exitCode": 5
  }
}
```

The code name is whatever B2b and L1b define for a missing password; field names follow ADR 0026 as merged.

## Acceptance criteria

- [ ] ADR 0031 is Accepted and listed in `docs/decisions/README.md`.
- [ ] `cargo run -p lamina-cli -- info fixtures/…` works for every command above; `lamina --help` lists only implemented commands.
- [ ] Every command's `--json` output validates against its committed schema in tests, and each document carries its `schema` id.
- [ ] Each exit code 0–5 is produced by at least one integration test (warnings only → 3, `--warnings-ok` → 0, bad range → 2, overwrite without `--force` → 4, encrypted fixture without password → 5, corrupt file → 1).
- [ ] `-` input and `-o -` output work in a pipe; writing binary to a TTY stdout is refused with exit 2.
- [ ] With stdin not a TTY, no command waits for input (tests run with closed stdin and a timeout from nextest).
- [ ] `NO_COLOR=1` and non-TTY output contain no ANSI escapes (test).
- [ ] No `message` or `hint` contains text from the document (test with a fixture whose Info title is an injection string).
- [ ] The command layer compiles without the clap front end (`commands` has no dependency on `main.rs`); L4c can call `commands::info` directly.
- [ ] `cargo xtask check` passes, including cargo-deny with the new dependencies.

## Post-merge checks

None.

## Tests

- Unit tests in `crates/lamina-cli/src/ranges.rs` (grammar cases from qpdf's manual, aliases, handles, errors with positions) and proptest.
- Integration tests in `crates/lamina-cli/tests/` using `assert_cmd`: one file per command, plus `exit_codes.rs`, `pipes.rs`, `json_schemas.rs` (validate every `--json` output with `jsonschema`), `no_prompt.rs`, `injection.rs`.
- Fixtures from `fixtures/corpus/` and `fixtures/golden/`; add small ones (encrypted, warning-only, injection-string title) to `fixtures/cli/` with provenance in `fixtures/README.md`.
- Byte-compare `pages rotate` output against a golden in `fixtures/golden/cli/`.

## Docs to update

- [ ] [Shareable libraries plan §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1): tick "Write ADR 0031; create `crates/lamina-cli` …"
- [ ] `docs/decisions/README.md` (ADR 0031)
- [ ] `CLAUDE.md`: layout (`crates/lamina-cli`), commands (`cargo run -p lamina-cli -- …`), ADR list
- [ ] `crates/lamina-cli/README.md` with the example sessions; `fixtures/README.md` for new fixtures

## Risks and open questions

- **Error on stdout or stderr with `--json`?** The research puts the error document on stderr and leaves stdout empty. Agents that capture only stdout then see nothing. Recommendation: stderr, as researched, and say so in `--help`; revisit after the L5 agent eval.
- **Code names:** codes are kebab-case strings today (`fixtures/codes.json`); if L1b introduces another form, the examples follow it.
- **`pages` sub-verbs or flat verbs** (`lamina merge` vs `lamina pages merge`): no usability evidence either way. Recommendation: `pages <verb>` now, with flat aliases only if users ask.
- **Façade gaps:** `info` may want form-field and signature counts that the façade does not expose yet; leave those keys out rather than reaching into lower crates.
- **Binary name** `lamina` vs `lam`: owner decides before L4d; this PR uses `lamina`.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
