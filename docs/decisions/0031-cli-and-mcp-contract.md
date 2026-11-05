# 0031 — CLI and MCP contract

- Status: Accepted (2026-11-04, when L4a's PR merges)
- Date: 2026-11-04

## Context

Lamina PDF needs one command-line tool for three readers: people at a shell, CI scripts, and AI coding agents. The [shareable-libraries report](../research/libraries/shareable-libraries.md#the-cli-is-the-primary-agent-interface-and-mcp-is-a-thin-adapter-over-it) concludes that the CLI, not MCP, is the primary agent interface. Playwright tells users that coding agents "increasingly favor CLI–based workflows exposed as SKILLs over MCP because CLI invocations are more token-efficient", so an MCP server should be a thin adapter over the same commands. Scripts and agents parse what a CLI prints and branch on its exit code, so both are a contract: once published, changing them breaks callers. This ADR fixes that contract before the first commands ship ([L4a](../todos/done/L4a-lamina-cli.md)) and more pile on ([L4b](../todos/done/L4b-cli-batch-render-schema.md), [L4c](../todos/done/L4c-mcp-mode.md)).

The [CLI spec note](../research/libraries/notes/cli-spec.md) surveyed the existing tools:
- **qpdf** has the best machine contract: exit codes 0, 2 and 3 (warnings only), a page-range grammar with `z`, `r3`, reverse ranges, `x` exclusions and `:odd`/`:even`, and versioned JSON whose consumers must ignore unknown keys ([§1](../research/libraries/notes/cli-spec.md#1-command-grammars-of-existing-pdf-clis-qpdf-mutool-pdftk-cpdf-poppler-ghostscript)). It can't read stdin.
- **mutool** has the verb subcommands (`info`, `pages`, `extract`, `show`). **pdftk** has handle-based page algebra (`A=fronts.pdf B=backs.pdf shuffle A Bend-1`). Both grammars can be borrowed; MuPDF's AGPL code can't.
- **Ghostscript** is the warning: interactive unless passed two flags, and safe only after sandbox escapes.
- **clig.dev** gives the conventions: data on stdout and everything else on stderr, `-` for stdin and stdout, no colour or animation without a TTY or with `NO_COLOR`, no prompts without a TTY, and `--json`, `--force`, `--dry-run` and `--no-input` ([§2](../research/libraries/notes/cli-spec.md#2-modern-cli-conventions-cligdev-12-factor-posix-exit-codes-env-vars-json)).

Three accepted ADRs already constrain the CLI. [ADR 0026](0026-error-and-diagnostic-contract.md) fixes the error object and its text form on the CLI (§7). [ADR 0027](0027-coordinates-units-and-page-indexing.md) §6 makes CLI page ranges 1-based and JSON pages carry `index` and `number`. [ADR 0003](0003-reader-tolerance-and-limits.md) defines the warnings, `Limits` and strict mode the CLI exposes. The owner named the binary `laminapdf` on 2026-11-01.

## Decision

### 1. One binary over one command layer

- The binary is **`laminapdf`**, built from `crates/laminapdf-cli` (layer `app`: it may use the file system, stdin and stdout).
- **The commands are a library**, `laminapdf_cli::commands`: one function per command that takes its arguments, the `Settings` (open flags, password, `--force`, `--dry-run`) and an `Io`, and returns a report or a `CliError`. Commands never print and hold no clap types in their reports.
- **`main.rs` is only the front end**: it parses with clap, calls the command, renders text or JSON, and maps the result to an exit code. `laminapdf mcp` (L4c) calls the same functions, so the CLI and MCP can't drift.
- Every command reads documents through the `laminapdf` façade ([ADR 0028](0028-facade-rules-jobs-and-cancellation.md)); a façade gap is a missing key in the report, never a reach into a lower crate.

### 2. Grammar

- **Verb first:** `laminapdf <command> [<subcommand>] <inputs…> [-o OUT | --in-place] [--json]`. Page operations are subcommands of `pages` (`pages rotate`, later `pages select`, `merge`, `split`); flat aliases are added only if users ask (no usability evidence either way, cli-spec note §6 Gaps).
- **Global flags:** `--json`, `--quiet`, `--no-input`, `--force`, `--dry-run`, `--color auto|always|never`, `--password-file FILE`, `--strict`, `--warnings-ok`, and one flag per limit `fixtures/codes.json` lists, named after it (`--max-stream-decoded`, …).
- **`--help` lists only commands that exist.** `laminapdf help exit-codes` prints §5's table.

### 3. Page ranges

Recommendation from the cli-spec note §1, adopted: **qpdf's grammar is the base**.
- `n`; `a-b` inclusive, counting down when `a > b`; `z` for the last page and `r<n>` for the n-th from the end; `x<n>` and `x<a-b>` exclude from the groups before them, or from every page when they come first (`x1` is every page but the first); `:odd` and `:even` select alternate pages **of the resulting set**; groups are joined by commas.
- `end` is an alias of `z`, for pdftk users. Uppercase `N` (mutool's last page) is **not** an alias, because uppercase letters are handles.
- **Handles** follow pdftk: `A=fronts.pdf B=backs.pdf`, then `A1-5`, `Bz-1`; a handle is one or more uppercase ASCII letters, so it never collides with a range token. An inline range is `file.pdf:1-z:odd`.
- **Ranges are 1-based page numbers**, as users count (ADR 0027 §6). A page outside the document is a usage error that points at the character.
- The whole grammar, handles included, is parsed now, even though merge and shuffle land later, so those commands only add wiring.

### 4. Inputs and outputs

- **`-` as an input reads stdin**, at most once per command; **`-o -` writes stdout**. When stdin is a terminal, `-` is a usage error rather than a silent wait (clig.dev: nobody types a PDF).
- **Binary output never goes to a TTY.** Writing a PDF to a stdout that is a terminal is a usage error (exit 2).
- **Nothing is overwritten without `--force`:** an existing `-o` target, and every `--in-place`. A refusal is exit 4.
- **Writes are atomic:** to a temporary file in the target's directory, then renamed over the target, so a failed write never leaves half a file.
- **Passwords come from `--password-file` or `LAMINA_PDF_PASSWORD`, never from argv**, where other users can read them in the process list. The CLI never prints a password.

### 5. Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Failure: the document could not be read, written or processed |
| `2` | Usage error: a bad flag, range or argument, or binary output to a terminal |
| `3` | Success with warnings; `--warnings-ok` turns it into `0` |
| `4` | Refused a destructive action that needs `--force` |
| `5` | Password required or wrong |

When several apply, the highest-priority one wins: **2 > 5 > 4 > 1 > 3 > 0**. The scale follows the report's recommendation. It keeps qpdf's `3` for warnings, so wrappers that already handle qpdf's warning exit keep working (Stirling-PDF had to add that handling, cli-spec note §1). Unlike qpdf, which reserves `1`, it uses `1` for every failure. It deliberately differs from gh, where `2` means cancelled and `4` authentication. Every error code in `fixtures/codes.json` maps to an exit code in one table, and a test checks that each code is mapped.

### 6. JSON output

- **`--json` prints one JSON document on stdout with a `schema` id**: `laminapdf.info/1`, `laminapdf.check/1`, `laminapdf.pages/1`, `laminapdf.text/1`, `laminapdf.explain/1`. Keys are camelCase, as in ADR 0026.
- **qpdf's compatibility rule:** within a version new keys may appear and consumers ignore unknown keys; the number goes up only on a breaking change (a key removed or its meaning changed).
- **Each schema is JSON Schema 2020-12, generated from the Rust report types** and committed under `crates/laminapdf-cli/schemas/`; a test fails when a committed schema is stale. L4c reuses them as MCP `outputSchema`s; the argument types derive `JsonSchema` too, for `inputSchema`.
- **Pages carry `index` and `number`** (ADR 0027 §6).
- **When stdout carries the PDF** (`-o -`), the `--json` report goes to stderr instead.
- **`--fields a,b`** keeps only the named top-level keys (gh's pattern, cli-spec note §4), to save an agent's tokens.
- **Batch output is NDJSON**, one document per line with a `type` field (`result` per input, in argument order, then one `summary`), after ripgrep's `--json` (cli-spec note §3). L4b builds it.

### 7. Streams, colour and prompts

- **Stdout holds data only:** a report, extracted text, or a PDF. Warnings, errors, progress and human diagnostics go to stderr.
- **Colour** only when the stream is a TTY and `NO_COLOR` is unset or empty; `--color` overrides. **Progress** only when stderr is a TTY, at most ten redraws a second (ADR 0028).
- **No prompts without a TTY.** When stdin is not a terminal, or with `--no-input`, anything that would prompt fails instead, with exit 4 or 5 and a hint naming the flag that answers it.

### 8. Errors and warnings

ADR 0026 §7 fixes the text form, and this ADR keeps it: `error[wrong-password]: wrong password`, then `= hint: …` and `= see: <docsUrl>`, on stderr; warnings the same with `warning[code]`. It amends §7's JSON form, which wrote bare §1 objects on stderr:

- **With `--json`, a failure is one line on stderr**: `{"schema":"laminapdf.error/1","error":{…}}`. The inner object is ADR 0026's §1 object, which also carries `input` (the path as given, or `-`) and `exitCode`; its schema already lets new keys appear.
- **Warnings are written once.** Every report lists its warnings, so under `--json` they are in the report. In text mode they go to stderr as `warning[code]` lines, which `--quiet` hides; the exit code still says 3.
- **Errors go to stderr even with `--json`**, as the research recommends, so stdout stays empty on failure. An agent that captures only stdout sees nothing; `--help` says where errors go. Revisit after the L5 agent evaluation.
- **The CLI's own codes** (a refused overwrite, a bad range, a failed read of an input) are listed in `fixtures/codes.json` as the `cliErrors` group, each with its `docs/errors/` page, like every other code.
- **Hints name flags.** Where the façade's hint names an API (`OpenOptions` for a password, `Limits` for a limit), the CLI gives the flag that answers it instead: `--password-file` or `LAMINA_PDF_PASSWORD`, `--max-stream-decoded`.
- **No document text in `message` or `hint`** (ADR 0026 §4). Extracted text, titles and other document strings appear only in data fields of a report.

### 9. MCP defaults, for L4c

The cli-spec note §4 and the filesystem MCP server's symlink CVEs set these, so `laminapdf mcp` is safe without opt-in flags:
- **stdio only**; `--root DIR` is required and repeatable; every path is canonicalised and a symlink that leaves the roots is rejected.
- **Read-only by default:** `--allow-write` lists the write tools, and `--allow-in-place` on top lets them overwrite inputs.
- **One tool per command**, with the command's schemas, in a fixed order, with honest annotations (`readOnlyHint` for reads; `destructiveHint` true only for in-place writes).
- **Document text only in data fields**; argument errors are tool errors (`isError: true`) the model can correct.

> **Amendment (L4b, 2026-11-05): batch.** `info`, `check` and `extract` take several inputs, from argv and `--files-from FILE|-` (newline-separated, or NUL-separated with `-0`). Standard input can be read once: `-` may appear once across argv and the list, and not at all when the list is read from it. `--jobs N` (by default the machine's parallelism, at most 8) runs them in parallel, and results are printed in argument order. One input named on argv keeps the single-document output; several, or any `--files-from` list, are a batch:
>
> - **Output:** with `--json`, stdout is NDJSON: one `{"type":"result","schema",…,"index","input","status"}` line per input (`status` is `ok`, `warnings` or `error`), carrying its report's keys or, for a failure, `error` (the §8 error object with `input` and `exitCode`, on stdout as data), then `{"type":"summary","ok","warnings","failed","skipped","elapsedMs"}`. In text mode each report comes under `==> input <==`, and errors and the counts go to stderr.
> - **Exit codes:** `0` all ok; `3` warnings and no failure (`0` with `--warnings-ok`); `1` any failure; `5` when every failure is a missing or wrong password; `2` for a usage error found before any work (no input, `-` twice). `--fail-fast` starts no input after a failure, and `skipped` counts those.
> - **Schemas:** the lines' are `result-line.schema.json` and `summary-line.schema.json`. `laminapdf schema [command]` prints a 2020-12 document whose `input`, `output`, `error`, `result` and `summary` properties embed those schemas; the arguments object is the shape `laminapdf mcp` takes too.
>
> Output templates for commands that write many files come with `render` ([L4e](../todos/ready/L4e-cli-render.md)).

## Consequences

- Scripts and agents get a contract that can grow without breaking them: new keys within a schema version, new commands, new codes.
- The CLI and MCP share one implementation and one set of schemas.
- Exit code 3 surprises callers who expect 0 after a repaired file; `--warnings-ok` is the escape hatch, as qpdf's `--warning-exit-0`.
- The CLI's codes join every generated table, the Rust `ErrorCode` and the TypeScript unions included, as the build and render codes do; the WebAssembly modules carry their summaries and hints too.
- The range grammar is larger than L4a needs; its parser and tests land now, so later page commands only add wiring.
- Errors on stderr under `--json` cost agents that read only stdout; the L5 evaluation decides whether that changes.

## Alternatives considered

- **MCP as the primary agent interface:** rejected. Agents increasingly prefer CLIs for token cost (Playwright), and MCP's 2026-07-28 revision deprecates Roots, so the server needs a CLI-like configuration anyway (cli-spec note §4).
- **mutool's `N` for the last page:** rejected, because uppercase letters are pdftk handles; `z` and `end` cover it.
- **qpdf's exit codes as they are** (`1` reserved, `2` for errors): rejected. `2` is clap's usage-error code, and one code for every failure is simpler to branch on.
- **`sysexits.h` codes (64–78):** rejected. They spread one failure over a dozen codes that few scripts check, and clap already exits 2 on a usage error.
- **JSON by default when stdout is not a TTY** (Arcjet's agent CLI): rejected. `extract` piped into `rg` is the common case, and its output should stay plain text.
- **Errors on stdout under `--json`:** rejected for now (§8); stdout is for data.
- **Flat page verbs** (`laminapdf merge`): not chosen without evidence; aliases can be added later without breaking the `pages` form.
