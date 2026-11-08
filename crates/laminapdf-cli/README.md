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
$ laminapdf extract invoice.pdf --format md > invoice.md
$ laminapdf render report.pdf --pages 1-3 --dpi 72 -o 'thumbs/{stem}-{n:03}.png'
$ laminapdf check docs/*.pdf --jobs 8 --json | jq -c 'select(.type=="result" and .status!="ok")'
$ laminapdf explain xref-repaired
```

## Install

> **Not yet published.** The release builds below are configured and tested in CI, but no release exists yet. Until one does, build from source: `cargo install --git https://github.com/laudebugs/laminapdf laminapdf-cli --features mcp`.

Each release will offer the same binary three ways, so you need no Rust toolchain:

- **Homebrew** (macOS and Linux): `brew install laudebugs/tap/laminapdf`.
- **npm**: `npm install -g @laminapdf/cli`, or `npx @laminapdf/cli info report.pdf`. npm installs only the binary for your platform, as an optional dependency; no install script downloads anything ([`packages/cli`](../../packages/cli/README.md)).
- **Archives**: `laminapdf-cli-<target>.tar.xz` (`.zip` on Windows) on the GitHub release, each with a `.sha256` checksum. An archive holds the binary, `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD_PARTY_NOTICES.md` (the licences of every crate in the binary), man pages in `man/` and shell completions in `completions/`.

Release binaries include the MCP server (`laminapdf mcp`). Builds exist for eight targets: macOS, Linux (glibc and musl) and Windows, each on arm64 and x86_64. CI builds and runs three of them today, on the runners the project has: macOS arm64, Linux arm64 with glibc and Linux arm64 with musl. The other five are built once a runner for each exists; Windows waits on a Windows runner. The binaries are unsigned for now, so macOS Gatekeeper and Windows SmartScreen warn about a downloaded archive; Homebrew and npm installs avoid most of these warnings.

## Commands

- **`info <inputs…>`**: the PDF version, page count and sizes, file size, encryption, whether the document is tagged, its Info dictionary and the warnings reading it raised. `--fields a,b` keeps only those keys of the JSON report.
- **`check <inputs…>`**: reads every object, every page and every content stream within the limits, and reports what a tolerant read had to repair. It also reports what a failed redaction left in the file: text a box or image is painted over (`covered-text`), redaction marks that were never applied (`redaction-not-applied`), and text an earlier revision still holds (`text-in-earlier-revision`). It exits 3 when it found either. `--strict` fails (exit 1) at the first repair, but the redaction findings aren't repairs, so they stay warnings. `--warnings-ok` turns exit 3 into 0 for both, so a CI gate that must catch failed redactions should read the warning codes in `--json`, not the exit code.
- **`reveal <inputs…> [--fail-on CATEGORIES] [--include-hidden-text]`**: what a PDF holds beyond its visible pages, in seven categories:
  - `history`: earlier revisions, with the objects each one adds, changes and frees and the words it holds that the latest revision doesn't, and objects nothing references
  - `metadata`: every Info entry, the XMP packet's size and namespaces, `/PieceInfo` and thumbnails
  - `attachments`: embedded files, with name, size and type
  - `active-content`: scripts, actions and what fires them, and XFA forms
  - `annotations`: annotations by type, with each comment's author and text
  - `forms`: form fields and their values
  - `hidden-content`: hidden text counted by reason (covered, invisible, off the page, tiny, clipped, low contrast), layers that are off, and unapplied redaction marks

  The text report has one line per category it found. With `--json`, `reveal` prints `laminapdf.reveal/1`. `reveal` counts hidden text without showing it. `--include-hidden-text` lists each hidden run with its page, box and text. With `--fail-on history,metadata,…` (or `all`), `reveal` exits 6 when a file holds anything in those categories, so a release job can stop a file before it ships:

  ```console
  $ laminapdf reveal dist/*.pdf --fail-on history,hidden-content,active-content,attachments
  ```

  Most files have an Info dictionary, so `--fail-on metadata` stops most files. Leave it out unless you strip metadata before release.
- **`pages rotate <input> <angle>[:<range>] (-o OUT | --in-place)`**: `+90` and `-90` turn pages from their current rotation, `90` sets it; the range names the pages. The result is an incremental update where the file allows one, so the input's bytes come first.
- **`extract <inputs…> [--pages RANGE] [--format text|md|json|chunks]`**: by default each page's text, pages separated by a form feed, as `pdftotext` does (`--visible-only` leaves out invisible text). The other formats read the blocks in reading order: `md` is Markdown with headings, lists and tables, `json` the `laminapdf.extract/1` document (every block with its box, and what was dropped; with `--format md --json`, the document carries the Markdown too, as `markdown`), `chunks` the Markdown cut for retrieval, one JSON object per line. They never contain hidden text. `--max-chars N` keeps whole blocks up to N characters of Markdown and adds `"truncated": true` to the JSON. Each page in the JSON has a `textLayer` that says whether its text can be used or the page needs OCR; a scanned page gives no Markdown and raises the `extract-needs-ocr` warning, which sets exit code 3 (0 with `--warnings-ok`).
- **`explain <code>`**: the page of an error or warning code, as in [`docs/errors/`](../../docs/errors/README.md).
- **`render <inputs…> -o TEMPLATE [--pages RANGE] [--dpi N | --scale F] [--backend hayro|pdfium]`**: each page as a PNG image, 150 dpi unless `--dpi` or `--scale` (pixels per point) says otherwise. The output is a template, described under Output templates; `-o -` writes one page to standard output. The built-in renderer is hayro. `--backend pdfium` uses PDFium instead, loaded from the folder `LAMINA_PDF_PDFIUM_DIR` names. With `--json`, the `laminapdf.render/1` report lists each page's pixel size and file. The renderer reads the file itself, so `render` refuses `--strict` and the limit flags.
- **`schema [command]`**: the JSON Schema of a command's arguments, its `--json` report, its error line and its batch lines, from the same types the commands and `laminapdf mcp` use; without a command, the list of schemas.
- **`completions bash|zsh|fish|powershell|elvish`**: a shell completion script, such as `laminapdf completions zsh > ~/.zfunc/_laminapdf`.
- **`help exit-codes`**, `help <command>` and `--version` (with `--json`, the versions and schema ids).

`-` reads standard input, `-o -` writes standard output, and a PDF never goes to a terminal. Passwords come from `--password-file FILE` or `LAMINA_PDF_PASSWORD`, never from the command line. Each limit `fixtures/codes.json` lists has a flag (`--max-stream-decoded N`, …), shown by `--help`.

## Many inputs

`info`, `check`, `extract`, `render` and `reveal` take several inputs, from the command line and from `--files-from FILE` (one per line, or separated by NUL bytes with `-0`, as `find -print0` writes them; `-` reads the list from standard input). They run in parallel on `--jobs N` threads (by default, one per CPU core, up to 8), and the results come out in the order you named the inputs. A single input named on the command line prints its usual report, as described under Commands and JSON; several inputs, or any list, print as a batch. With `--json` each result is one line, then a summary:

```console
$ fd -e pdf -0 | laminapdf info --files-from - -0 --json --fields pages
{"index":0,"input":"a.pdf","pages":4,"schema":"laminapdf.info/1","status":"ok","type":"result"}
{"error":{"code":"password-required",…},"index":1,"input":"b.pdf","schema":"laminapdf.info/1","status":"error","type":"result"}
{"elapsedMs":12,"failed":1,"ok":1,"skipped":0,"type":"summary","warnings":0}
```

With `--json`, a failed input is a `result` line with an `error` on standard output, not a line on standard error. In text mode each report comes under a `==> input <==` line, and errors and the counts go to standard error. When standard error is a terminal, a progress bar shows there; `--quiet` hides it.

A batch exits 1 when any input failed, or 5 when every failure was a missing or wrong password. Otherwise it exits 6 when `reveal --fail-on` found something in some input, 3 when some had warnings (0 with `--warnings-ok`), and 0 when every input succeeded. The summary's `found` counts the inputs that held something `--fail-on` names. `--fail-fast` starts no input after the first failure, and the summary counts those skipped.

## Output templates

One `-o` names every file `render` writes. In a template such as `out/{stem}-{n:03}.png`, these fields are filled in for each input and page:

| Field | Value |
| --- | --- |
| `{stem}` | the input's file name without its extension (`report` for `docs/report.pdf`; `stdin` for `-`) |
| `{name}` | the input's file name (`report.pdf`) |
| `{index}` | the input's position on the command line, from 0 |
| `{n}` | the page number, from 1 |

A width after a colon, written with a leading zero, pads a number with zeros: `{n:03}` is `007`. `{{` and `}}` write braces.

The command works out every path before it writes anything. It refuses, with exit 2, a template that would give two pages or two inputs the same file, such as one without `{n}` for a document of several pages, or one without `{stem}`, `{name}` or `{index}` for several inputs. `-o -` takes one page of one input named on the command line. Without `--force`, an existing file stops it with exit 4, also before any page is written. It doesn't create folders, so make them first.

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
| 6 | `reveal --fail-on` found something in a category it names |

When several apply, the first in 2, 5, 4, 1, 6, 3, 0 wins. Only `reveal --fail-on` returns 6.

## JSON

With `--json`, the report goes to standard output with a `schema` id (`laminapdf.info/1`, `laminapdf.check/1`, `laminapdf.pages/1`, `laminapdf.text/1`, `laminapdf.extract/1`, `laminapdf.explain/1`, `laminapdf.reveal/1`) and lists its warnings. With one input, a failure leaves standard output empty and writes one line to standard error (a batch writes a `result` line instead, as Many inputs says): `{"schema":"laminapdf.error/1","error":{…}}`, the error object every Lamina PDF surface uses ([ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md)), with `input` and `exitCode`. Document text, such as a title, appears only in report fields, never in an error's message or hint. Within a schema version keys may be added, so read only the ones you need. The schemas are in [`schemas/`](schemas/), generated from the report types, and `laminapdf schema <command>` prints them with the command's arguments.

## MCP server

For clients that speak only the Model Context Protocol, `laminapdf mcp` serves the same commands as MCP tools on standard input and output. Only builds with the crate's `mcp` feature have it (`cargo build -p laminapdf-cli --features mcp`), so the plain CLI carries no async runtime.

```console
$ laminapdf mcp --root ./docs                  # read-only: laminapdf_check, laminapdf_explain, laminapdf_extract, laminapdf_info, laminapdf_reveal
$ laminapdf mcp --root ./docs --allow-write    # adds laminapdf_pages_rotate and laminapdf_render, which write new files only
```

A client starts it as a command. In Claude Code, add it with `claude mcp add laminapdf -- laminapdf mcp --root /home/me/docs`; Claude Desktop and most other clients take the same command in their configuration file:

```json
{ "mcpServers": { "laminapdf": { "command": "laminapdf", "args": ["mcp", "--root", "/home/me/docs"] } } }
```

Each tool takes the command's arguments as JSON (`{"input": "report.pdf", "pages": "1-3"}`) and answers with the command's report as `structuredContent`, so its `outputSchema` is the report's schema. `laminapdf_extract` takes every `format` the command does; its `outputSchema` covers both the text report and `laminapdf.extract/1`. A failure is a tool error whose text is the same `laminapdf.error/1` line the CLI writes under `--json`, so the model can read the code and correct its call. The line goes in the text rather than in `structuredContent`, which has to match the report's schema.

The server is safe without flags, because a model may follow instructions planted in a document:

- **Roots.** Tools touch only files under the `--root` folders. A relative path starts at the first root. Before it checks a path, the server resolves it and follows its links. So `..`, an absolute path elsewhere and a link that leads out all get `path-outside-roots`. Outside the roots, a missing file gets the same answer as one that exists, so a call can't probe for files.
- **Read-only by default.** The server lists the write tools only with `--allow-write`. They write new files only, with one exception: when the server also has `--allow-in-place`, the rotate tool can replace its input. The render tool never replaces a file or writes to standard output.
- **Document text stays in data.** Titles and extracted text appear only in report fields. Error messages and hints never quote the document, and tool descriptions are fixed text. `laminapdf_reveal` counts hidden text but never shows it, because hidden text can carry instructions meant for a model. Its input has no `failOn` or `includeHiddenText`.
- **Bounded calls.** At most one call per CPU runs at a time. The server stops a call that runs past `--timeout` (60 s by default) at its next step and answers `cancelled`. A step that can't stop, such as opening a large file, gets 5 more seconds. After that the server answers anyway, and the call keeps its CPU slot until the step ends.
- **Bounded output.** A report over `--max-output-bytes` (1 MiB by default, 1 KiB at least) loses items from the end of its lists, then text, and gets `"truncated": true`. The cut report still matches the tool's `outputSchema`.

The server opens documents with the command line's settings: `--strict`, the limit flags and the password (`--password-file` or `LAMINA_PDF_PASSWORD`). It refuses `--force`, because only `--allow-in-place` may let a tool replace a file.

## Code

`src/commands/` holds one function per command, which takes its arguments, the settings and an `Io`, and returns a report or a `CliError`. Commands never print, so `laminapdf mcp` can call the same functions. `src/main.rs` is the clap front end and `src/render.rs` the text and JSON output. `src/mcp/` is the MCP server (`mcp` feature): the server in `mod.rs`, the tool list and its schemas in `tools.rs`, the output limit in `truncate.rs` and the path policy in `roots.rs`. The rest is shared: `src/ranges.rs` (the range grammar), `src/exit.rs` (exit codes and errors), `src/io.rs` (stdin, stdout, atomic writes), `src/schema.rs` (the schemas) and `src/batch.rs` (inputs, the thread pool that hands results back in order, the batch exit code). The front end's `src/many.rs` prints a batch, and `src/docs.rs` serves `schema`, `completions` and the man pages. `src/template.rs` parses and fills in output templates, and `src/commands/render.rs` renders through `laminapdf-render` and writes PNGs with the `png` crate.

Tests (`tests/`, reading the shared [`fixtures/`](../../fixtures/README.md)): one file per command (`info.rs`, `check.rs`, `pages.rs`, `extract.rs`, `explain.rs`), and `exit_codes.rs` (every code from a real run), `pipes.rs`, `json_schemas.rs` (the committed schemas are the generated ones, and every `--json` output validates), `no_prompt.rs` (nothing waits for input), `injection.rs` (document text stays in data fields, and colour reaches only a terminal), `io.rs`, `batch.rs` (argument order over 50 files, NDJSON against its schemas, the exit codes, `--fail-fast`, `--files-from`), `schema.rs` (every command's schema compiles as 2020-12 and embeds the committed ones), `completions.rs`, `render.rs` (every example's pages against the render goldens, templates, clashes and refusals before any write, standard output), and with the `mcp` feature `mcp.rs` (the server through rmcp's client: the tool list, schemas, roots, write flags and injection). `examples/golden.rs` writes `fixtures/golden/cli-rotate-hello.pdf` for `cargo xtask golden --update`.

Releases ([L4d](../../docs/todos/done/L4d-cli-distribution.md)): [`dist-workspace.toml`](../../dist-workspace.toml) configures cargo-dist (`dist`, pinned in `tools/xtask/src/doctor.rs`) for this crate alone, with the `mcp` feature; `about/` holds cargo-about's configuration and the notices template. [`.github/workflows/cli-release.yml`](../../.github/workflows/cli-release.yml) runs `dist plan` on pull requests that touch this crate or the release files. Dispatched by hand, it builds the archives, installs each one (the archive, the npm tarballs that [`tools/cli-release/generate.ts`](../../tools/cli-release/generate.ts) writes from it, and on macOS the Homebrew formula) and runs it. A `laminapdf-cli-v<version>` tag publishes, but only while the owner's variable `LAMINA_PDF_PUBLISH_CLI` is `true`.

```sh
cargo run -p laminapdf-cli -- info fixtures/corpus/hello.pdf
cargo xtask cli-docs                                                # every completion script and a man page per command, into target/cli-docs/
cargo xtask cli-dist                                                # those plus THIRD_PARTY_NOTICES.md (cargo-about): what each release archive holds
dist build --artifacts=local --target "$(rustc -vV | sed -n 's/host: //p')"   # this host's archive into target/distrib/
cargo test -p laminapdf-cli --features mcp
UPDATE_GOLDEN=1 cargo test -p laminapdf-cli --test json_schemas   # after changing a report type
```
