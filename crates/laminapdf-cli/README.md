# laminapdf-cli

The `laminapdf` command-line tool ([ADR 0031](../../docs/decisions/0031-cli-and-mcp-contract.md), [L4a](../../docs/todos/done/L4a-lamina-cli.md)): read, check and change PDFs from a shell, a CI script or an AI agent. Every command prints a report a person can read, or versioned JSON a program can parse, and ends with an exit code that says what happened.

```console
$ laminapdf info report.pdf
report.pdf  PDF 1.7 · 4 pages · 41 KB · not encrypted · tagged
Title: Quarterly report
Pages: 595 × 842 pt
$ laminapdf info scan.pdf --json --fields pages,encrypted,warnings
{"encrypted":false,"input":"scan.pdf","pages":1,"schema":"laminapdf.info/1","warnings":[{"code":"xref-repaired",…}]}
$ echo $?
3
$ cat scan.pdf | laminapdf pages rotate - +90:1-3,x2 -o - > rotated.pdf
$ laminapdf pages rotate scan.pdf 90:z --in-place
error[overwrite-refused]: refusing to replace a file without --force
  = hint: pass --force, or write to a new file with -o
  = see: https://github.com/laudebugs/laminapdf/blob/main/docs/errors/overwrite-refused.md
$ echo $?
4
$ curl -sL https://example.org/a.pdf | laminapdf extract - --pages 1-3 | rg -i invoice
$ laminapdf explain xref-repaired
```

## Commands

- **`info <input>`**: the PDF version, page count and sizes, file size, encryption, whether the document is tagged, its Info dictionary and the warnings reading it raised. `--fields a,b` keeps only those keys of the JSON report.
- **`check <input>`**: reads every object, every page and every content stream within the limits, and reports what a tolerant read had to repair. It exits 3 when there were repairs, and 1 with `--strict` at the first one.
- **`pages rotate <input> <angle>[:<range>] (-o OUT | --in-place)`**: `+90` and `-90` turn pages from their current rotation, `90` sets it; the range names the pages. The result is an incremental update where the file allows one, so the input's bytes come first.
- **`extract <input> [--pages RANGE]`**: each page's text, pages separated by a form feed, as `pdftotext` does. `--format text` is the only format so far.
- **`explain <code>`**: the page of an error or warning code, as in [`docs/errors/`](../../docs/errors/README.md).
- **`help exit-codes`**, `help <command>` and `--version` (with `--json`, the versions and schema ids).

`-` reads standard input, `-o -` writes standard output, and a PDF never goes to a terminal. Passwords come from `--password-file FILE` or `LAMINA_PDF_PASSWORD`, never from the command line. Each limit `fixtures/codes.json` lists has a flag (`--max-stream-decoded N`, …), shown by `--help`.

## Page ranges

Pages count from 1. A range is groups joined by commas: `7`, `z` (the last page, also `end`), `r2` (the second from the end), or a span such as `3-7` or `z-1`. A group with `x` in front removes pages: `1-z,x3` is every page but 3, and so is `x3` alone. `:odd` or `:even` at the end keeps every other page of the result. The grammar is qpdf's; handles (`A=fronts.pdf`, then `A1-5`) are parsed already for the page commands to come.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | The document could not be read, written or processed |
| 2 | A bad flag, range or argument, or binary output to a terminal |
| 3 | Success with warnings; `--warnings-ok` makes it 0 |
| 4 | A destructive action refused without `--force` |
| 5 | A password is required, or the one given is wrong |

When several apply, the first in 2, 5, 4, 1, 3, 0 wins.

## JSON

With `--json`, the report goes to standard output with a `schema` id (`laminapdf.info/1`, `laminapdf.check/1`, `laminapdf.pages/1`, `laminapdf.text/1`, `laminapdf.explain/1`) and lists its warnings. A failure leaves standard output empty and writes one line to standard error: `{"schema":"laminapdf.error/1","error":{…}}`, the error object every Lamina PDF surface uses ([ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md)), with `input` and `exitCode`. Document text, such as a title, appears only in report fields, never in an error's message or hint. Within a schema version keys may be added, so read only the ones you need. The schemas are in [`schemas/`](schemas/), generated from the report types.

## MCP server

For clients that speak only the Model Context Protocol, `laminapdf mcp` serves the same commands as MCP tools on standard input and output. Only builds with the crate's `mcp` feature have it (`cargo build -p laminapdf-cli --features mcp`), so the plain CLI carries no async runtime.

```console
$ laminapdf mcp --root ./docs                  # read-only: laminapdf_check, laminapdf_explain, laminapdf_extract, laminapdf_info
$ laminapdf mcp --root ./docs --allow-write    # adds laminapdf_pages_rotate, which writes new files only
```

A client starts it as a command. In Claude Code, add it with `claude mcp add laminapdf -- laminapdf mcp --root /home/me/docs`; Claude Desktop and most other clients take the same command in their configuration file:

```json
{ "mcpServers": { "laminapdf": { "command": "laminapdf", "args": ["mcp", "--root", "/home/me/docs"] } } }
```

Each tool takes the command's arguments as JSON (`{"input": "report.pdf", "pages": "1-3"}`) and answers with the command's report as `structuredContent`, so its `outputSchema` is the report's schema. A failure is a tool error whose text is the same `laminapdf.error/1` line the CLI writes under `--json`, so the model can read the code and correct its call. The line goes in the text rather than in `structuredContent`, which has to match the report's schema.

The server is safe without flags, because a model may follow instructions planted in a document:

- **Roots.** Tools touch only files under the `--root` folders. A relative path starts at the first root. Before it checks a path, the server resolves it and follows its links. So `..`, an absolute path elsewhere and a link that leads out all get `path-outside-roots`. Outside the roots, a missing file gets the same answer as one that exists, so a call can't probe for files.
- **Read-only by default.** The server lists the write tool only with `--allow-write`. It writes new files only, unless the server also has `--allow-in-place`, which lets it replace its input.
- **Document text stays in data.** Titles and extracted text appear only in report fields. Error messages and hints never quote the document, and tool descriptions are fixed text.
- **Bounded calls.** At most one call per CPU runs at a time. The server stops a call that runs past `--timeout` (60 s by default) at its next step and answers `cancelled`. A step that can't stop, such as opening a large file, gets 5 more seconds. After that the server answers anyway, and the call keeps its CPU slot until the step ends.
- **Bounded output.** A report over `--max-output-bytes` (1 MiB by default, 1 KiB at least) loses items from the end of its lists, then text, and gets `"truncated": true`. The cut report still matches the tool's `outputSchema`.

The server opens documents with the command line's settings: `--strict`, the limit flags and the password (`--password-file` or `LAMINA_PDF_PASSWORD`). It refuses `--force`, because only `--allow-in-place` may let a tool replace a file.

## Code

`src/commands/` holds one function per command, which takes its arguments, the settings and an `Io`, and returns a report or a `CliError`. Commands never print, so `laminapdf mcp` can call the same functions. `src/main.rs` is the clap front end and `src/render.rs` the text and JSON output. `src/mcp/` is the MCP server (`mcp` feature): the server in `mod.rs`, the tool list and its schemas in `tools.rs`, the output limit in `truncate.rs` and the path policy in `roots.rs`. The rest is shared: `src/ranges.rs` (the range grammar), `src/exit.rs` (exit codes and errors), `src/io.rs` (stdin, stdout, atomic writes) and `src/schema.rs` (the schemas).

Tests (`tests/`, reading the shared [`fixtures/`](../../fixtures/README.md)): one file per command (`info.rs`, `check.rs`, `pages.rs`, `extract.rs`, `explain.rs`), and `exit_codes.rs` (every code from a real run), `pipes.rs`, `json_schemas.rs` (the committed schemas are the generated ones, and every `--json` output validates), `no_prompt.rs` (nothing waits for input), `injection.rs` (document text stays in data fields, and colour reaches only a terminal) `io.rs`, and with the `mcp` feature `mcp.rs` (the server through rmcp's client: the tool list, schemas, roots, write flags and injection). `examples/golden.rs` writes `fixtures/golden/cli-rotate-hello.pdf` for `cargo xtask golden --update`.

```sh
cargo run -p laminapdf-cli -- info fixtures/corpus/hello.pdf
cargo test -p laminapdf-cli --features mcp
UPDATE_GOLDEN=1 cargo test -p laminapdf-cli --test json_schemas   # after changing a report type
```
