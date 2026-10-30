# Designing the lamina PDF CLI for humans and AI shell agents (state: October 2026)

> Method note: this session could not resolve qpdf.readthedocs.io, clig.dev, modelcontextprotocol.io, blog.arcjet.com or mankier.com (network allow-list). Primary sources were therefore read from their **GitHub source repositories** (raw.githubusercontent.com: qpdf manual `.rst`, clig.dev's `content/_index.md`, the MCP spec `.mdx`/`schema.ts`, rmcp/usage/github-mcp-server/playwright-mcp READMEs, mutool man page, gh help source, ripgrep flag defs, Ghostscript `Use.rst`, FreeBSD `sysexits.h`) and crates.io API. The rendered-site URLs are given next to the raw ones where they are the canonical address. Crate versions were read from crates.io on 2026-10-29. Items not verifiable this session are in **Gaps**.

## 1. Command grammars of existing PDF CLIs (qpdf, mutool, pdftk, cpdf, poppler, Ghostscript)

### Takeaway
qpdf is the best model for *machine contracts* (versioned JSON with a `--json-help` schema, job-JSON input, tri-state exit codes 0/2/3, rich page-range grammar), mutool is the best model for *verb subcommands*, pdftk for a terse *handle-based page algebra*; Ghostscript is the anti-pattern (interactive by default, `-d/-s` switch soup).

### Cited Findings
**qpdf**
- Exit codes: `0` no errors or warnings; `1` "not used" (reserved because the shell uses 1 when it cannot invoke the command); `2` "errors were found; the file was not processed"; `3` "warnings were found without errors"; if both errors and warnings occur, `2` is used; `--warning-exit-0` turns warnings-only into 0; `--no-warn` hides warnings but still exits 3; `--is-encrypted`/`--requires-password` have their own exit codes — [qpdf manual cli.rst §Exit Status](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst) (rendered: https://qpdf.readthedocs.io/en/stable/cli.html#exit-status).
- Real-world consequence: Stirling-PDF had to change its wrapper because qpdf's exit 3 (success with warnings) during repair was treated as failure — [search summary of Stirling-PDF commit](https://github.com/Stirling-Tools/Stirling-PDF/commit/507d21772d212299514f254471008433a64ed3ff) (secondary; indicates "warnings ≠ failure" codes surprise integrators).
- Since qpdf 9.0.0, `--check` exits 3 rather than 2 for linearization-only warnings — [qpdf release notes via search](https://qpdf.readthedocs.io/en/11.1/release-notes.html).
- Page-range syntax: comma-separated groups; `<n>`; `r<n>` = n-th from the end; `z` = last page (= `r1`); `a-b` inclusive, counting down if a > b; `x` prefix excludes (`a-b,xc`, `a-b,xc-d`); suffix `:even`/`:odd` selects alternate pages *of the resulting set*, not original numbers — [cli.rst §Page Ranges](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst).
- Page selection: `qpdf in.pdf --pages [--file=]file [options] [...] -- out.pdf`; newer `--file=` / `--range=` named forms alongside positional `input-file [--password=] [page-range]`; `.` means the primary input (`qpdf in.pdf --pages . a.pdf b.pdf 1-z:even -- out.pdf`); `--collate[=n[,m…]]`, `--split-pages[=n]`, `--rotate=[+|-]angle[:page-range]` — [cli.rst §Page Selection](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst).
- In-place / streams: `--replace-input` in place of outfile overwrites the input; outfile may be `-` for stdout, but "reading from stdin is not supported"; output need not be seekable even for linearized files; `@file` argument files and `@-` to read arguments from stdin — [cli.rst](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst).
- Job JSON: `--job-json-file=file` is "read and treated as if the equivalent command-line arguments were supplied", repeatable and mixable with options; `--job-json-help` describes the format; it is unrelated to `--json` but combinable — [cli.rst](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst).
- JSON output: `--json[=version]` ("latest" default), `--json-key=key` (repeatable; restricts top-level keys), `--json-output[=version]` (faithful round-trippable representation), `--json-input`, `--update-from-json` — [cli.rst](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst). Default moved from v1 to v2 in qpdf 11 — [qpdf json.rst](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/json.rst).
- JSON compatibility contract: top-level `"version"` integer bumped only on incompatible change (key removal, format change, semantic change); also in `qpdf[0].jsonversion`; within a version new keys may be added, so "consumers … should ignore keys they don't know about"; `--json-help` prints a JSON object of identical shape whose leaves are descriptions, with `<placeholder>` keys for maps keyed by object IDs — [json.rst §JSON Compatibility Guarantees](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/json.rst).

**mutool (MuPDF)**
- One binary with subcommands: `audit, bake, barcode, clean, convert, create, draw, extract, info, merge, pages, poster, recolor, run, show, sign, trace, trim` — [mutool.1 man page](https://raw.githubusercontent.com/ArtifexSoftware/mupdf/master/docs/man/mutool.1).
- Page lists: comma-separated numbers and ranges, e.g. `1,5,10-15,20-N`, where `N` is the last page — [mutool.1](https://raw.githubusercontent.com/ArtifexSoftware/mupdf/master/docs/man/mutool.1).
- `mutool show file.pdf [object-number …]` prints objects/streams to stdout (decoded by default; `-e` raw, `-b` binary, `-o file`) — a good model for an object-level `inspect` — [mutool.1 SHOW](https://raw.githubusercontent.com/ArtifexSoftware/mupdf/master/docs/man/mutool.1).
- `mutool merge [options] file1 [pages1] [file2 [pages2] …]` — positional file/range pairs — [mutool.1 MERGE](https://raw.githubusercontent.com/ArtifexSoftware/mupdf/master/docs/man/mutool.1).
- Exit status only "0 upon successful execution and non-zero upon error" (no warning code) — [mutool.1 EXIT STATUS](https://raw.githubusercontent.com/ArtifexSoftware/mupdf/master/docs/man/mutool.1).
- `mutool draw` vs `convert`: draw's "options are harder to use and follow a different syntax", users told to try `convert` first (2019 article, older version) — [Hoekwater, MAPS 49 (2019)](http://www.ntg.nl/maps/49/11.pdf). `clean -g/-gg/-ggg` = increasing garbage collection/renumbering/dedup (same source + [Ubuntu manpage](https://manpages.ubuntu.com/manpages/jammy/man1/mutool.1.html)).
- Licence: MuPDF is AGPL/commercial (repo note in lamina CLAUDE.md); grammar can be borrowed, code cannot.

**pdftk**
- Handle grammar: `pdftk A=fronts.pdf B=backs.pdf shuffle A1-end Bend-1 output out.pdf`; range = handle + optional begin/end + optional qualifier (`even`/`odd`) + optional rotation; `end` keyword; reverse ranges (`Bend-1`); `B` alone = `B1-end`; `1-6even` → 2,4,6; `shuffle` interleaves ranges; `burst` takes no ranges and writes `out_%d.pdf` — [pdftk man page](https://leancrew.com/all-this/man/man1/pdftk.html), [pdftk-java manpage](https://manpages.debian.org/buster/pdftk-java/pdftk.pdftk-java.1.en.html).
- Keyword-operation grammar (`cat`, `shuffle`, `burst`, `output`) with no leading dashes is memorable but non-POSIX — same sources.

**cpdf** — single executable, AGPL (commercial licence otherwise); features include split/merge keeping bookmarks, encryption AES-128/256, stamping, attachments, "Export and import PDF files in JSON format", PDF/UA-1/2 — [cpdf-binaries README](https://raw.githubusercontent.com/coherentgraphics/cpdf-binaries/master/README.md).

**Ghostscript**
- Interactive by default: after processing files it reads PostScript from the keyboard; `-dBATCH -dNOPAUSE` disable prompting; `-o out` is a shorthand that also sets BATCH/NOPAUSE; output via `-sDEVICE=png16m -r300 -sOutputFile=out-%d.png` — [Ghostscript Use.rst](https://raw.githubusercontent.com/ArtifexSoftware/ghostpdl/master/doc/src/Use.rst).
- `SAFER` default since 9.50 after earlier sandbox escapes — [Use.rst](https://raw.githubusercontent.com/ArtifexSoftware/ghostpdl/master/doc/src/Use.rst). (Lesson for agents: safe defaults must not need opt-in flags.)

### Inferences
- Adopt qpdf's page-range grammar as the superset (it covers mutool's `N` as `z`, pdftk's `end`, reverse ranges, `x` exclusion, `:odd/:even`) and *also accept* `end`/`N` aliases plus pdftk-style handles (`A=x.pdf`) in a single `pages` subcommand; this gives continuity for all three user bases.
- Adopt qpdf's tri-state exit model but with a different number for "warnings" so `1` stays a generic failure (see §2), and ship qpdf-style `--json-help`/JSON Schema with a versioned `schema_version`.
- Avoid Ghostscript's "interactive unless you pass two flags" and qpdf's "no stdin input" (lamina is bytes-in/bytes-out, so stdin is natural).

### Gaps
- cpdf page-range keywords (`end`, `~n`, `reverse`, `portrait`, `AND` chaining) and its `-json` output schema: manual (cpdfmanual.pdf) not reachable this session — verify at https://www.coherentpdf.com/cpdfmanual/cpdfmanual.html.
- poppler-utils (pdftotext/pdftoppm/pdfinfo) exit codes (commonly 0/1 open error/2 output error/3 permissions/99 other) — man pages on gitlab.freedesktop.org not reachable; unverified.
- No first-hand survey data on "what users love/hate"; complaints above are inferred from the docs' own caveats.

## 2. Modern CLI conventions (clig.dev, 12-factor, POSIX, exit codes, env vars, JSON)

### Takeaway
The consensus contract: data on stdout, everything else on stderr; `-` for stdin/stdout; TTY-detect for colour/animations/prompts; `NO_COLOR`; `--json` (and `--plain`); never prompt without a TTY; `--force`/`--no-input`/`--dry-run`; 0 = success, small set of documented non-zero codes.

### Cited Findings
- "Return zero exit code on success, non-zero on failure… Map the non-zero exit codes to the most important failure modes." "Send messaging to `stderr`. Log messages, errors, and so on should all be sent to `stderr`." — [clig.dev source](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md) (rendered https://clig.dev/).
- TTY: "The most simple and straightforward heuristic for whether a particular output stream… is being read by a human is whether or not it's a TTY"; disable colour when stdout/stderr isn't a TTY (check each individually), when `NO_COLOR` is set and non-empty, optionally `MYAPP_NO_COLOR`; "If stdout is not an interactive terminal, don't display any animations." — [clig.dev](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md).
- Machine output: "Have machine-readable output where it does not impact usability"; `--plain` for one-record-per-line output; "Display output as formatted JSON if `--json` is passed"; `-q` to suppress non-essential output; "Encourage your users to use `--plain` or `--json` in scripts to keep output stable" — [clig.dev](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md).
- Standard flag names: `-f/--force`, `--json`, `-n/--dry-run`, `--no-input`, `-q`, `-o`, `-v` — [clig.dev §Arguments and flags](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md).
- Interactivity: "Only use prompts or interactive elements if stdin is an interactive terminal"; "If `--no-input` is passed, don't prompt"; "Confirm before doing anything dangerous… prompt for y/yes if running interactively, or requiring them to pass `-f` or `--force` otherwise"; for severe actions, `--confirm="name-of-thing"` keeps it scriptable — [clig.dev](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md).
- "If input or output is a file, support `-` to read from stdin or write to stdout." "If your command is expecting to have something piped to it and stdin is an interactive terminal, display help immediately and quit." — [clig.dev](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md).
- Pagers: use only if stdin or stdout is a TTY; "Don't treat stderr like a log file" by default — [clig.dev](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md). clig.dev cites Jeff Dickey's [12 Factor CLI Apps](https://medium.com/@jdxcode/12-factor-cli-apps-dd3c227a0e46) (2018 — older).
- Env vars to honour: `NO_COLOR`, `FORCE_COLOR`, `PAGER`, `TERM`, etc. — [clig.dev §Environment variables](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md).
- gh: `--json fields` (omit the arg to list available fields), `--jq` (embedded jq, no install needed), `--template` (Go templates); pretty-prints only to a terminal; env `GH_PAGER`/`PAGER`, `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `GH_PROMPT_DISABLED`; exit codes 0 success, 1 failure, 2 cancelled, 4 auth required — [gh help_topic.go](https://raw.githubusercontent.com/cli/cli/trunk/pkg/cmd/root/help_topic.go).
- ripgrep `--json`: JSON Lines stream with message types `begin`, `end` (with per-file stats), `match`, `context`, `summary` — [ripgrep flags/defs.rs](https://raw.githubusercontent.com/BurntSushi/ripgrep/master/crates/core/flags/defs.rs). This is the canonical NDJSON-per-file pattern for batch.
- sysexits.h: `EX_USAGE 64`, `EX_DATAERR 65`, `EX_NOINPUT 66`, `EX_UNAVAILABLE 69`, `EX_SOFTWARE 70`, `EX_OSERR 71`, `EX_CANTCREAT 73`, `EX_IOERR 74`, `EX_TEMPFAIL 75`, `EX_NOPERM 77`, `EX_CONFIG 78` — [FreeBSD sysexits.h](https://raw.githubusercontent.com/freebsd/freebsd-src/main/include/sysexits.h).
- Agent-oriented CLI example: Arcjet's CLI returns exit code 4 and a JSON "confirmation envelope" on stdout instead of an interactive prompt when `--confirm` is absent, and defaults to JSON when stdout is not a TTY — [Arcjet blog via search summary](https://blog.arcjet.com/designing-a-cli-for-ai-agents/) (page not fetchable; summary only).

### Inferences
- Exit codes for lamina (recommendation): `0` ok; `1` generic failure; `2` usage error (clap's default for parse errors); `3` completed with warnings (qpdf-compatible, opt out with `--warnings-ok`); `4` needs confirmation / refused destructive op without `--force`; `5` password required/wrong; `64–78` not used directly but mapped in JSON `error.sysexit` for tooling that wants them. Document with `lamina help exit-codes` (gh pattern).
- Defaults: human tables on TTY, `--json` explicit; optionally `LAMINA_OUTPUT=json` env to make JSON the default for agent sessions (avoid Arcjet-style auto-JSON on non-TTY for `extract text`, where piping raw text is the common case).

### Gaps
- cargo `--message-format=json` reason-tagged stream docs (cargo `external-tools.md`) 404'd at the path tried; known shape (`{"reason":"compiler-message",…}`, `build-finished`) is from training knowledge — verify at https://doc.rust-lang.org/cargo/reference/external-tools.html.
- kubectl `-o json|yaml|jsonpath`, uv, deno, wrangler output flags not verified this session.

## 3. Batch and parallel processing

### Takeaway
Be xargs/parallel-friendly (one file per invocation works, NUL-safe input, stdout only data), *and* offer built-in multi-file mode with `--jobs N` that streams NDJSON results in deterministic input order and reports partial failure via a summary + exit code.

### Cited Findings
- ripgrep's model: per-file `begin`/`end` messages plus a final `summary`, one JSON object per line — [ripgrep defs.rs](https://raw.githubusercontent.com/BurntSushi/ripgrep/master/crates/core/flags/defs.rs).
- qpdf reads arguments from a file or stdin (`@file`, `@-`) — a pattern for very long batch lists — [qpdf cli.rst](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst).
- Output-name templates: pdftk `burst output out_%d.pdf`; Ghostscript `-sOutputFile=out-%d.png` — [pdftk man](https://leancrew.com/all-this/man/man1/pdftk.html), [Ghostscript Use.rst](https://raw.githubusercontent.com/ArtifexSoftware/ghostpdl/master/doc/src/Use.rst).
- MCP 2026-07-28 says servers SHOULD return tools in deterministic order to improve caching/prompt-cache hits — the same argument applies to deterministic CLI output ordering — [MCP 2026-07-28 changelog](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2026-07-28/changelog.mdx).

### Inferences
- `lamina <cmd> a.pdf b.pdf … --jobs 8 --json` → NDJSON, one `{"type":"result","input":…}` per file, emitted in argument order (buffer out-of-order completions; rayon in the binary only — core crates stay single-threaded per portability rules), then `{"type":"summary","ok":N,"failed":M,"warnings":K}`; exit `1` if any failed, `3` if only warnings. `--fail-fast` to stop on first error. `--files-from FILE|-` with `-0/--null` for NUL-separated lists (xargs -0 parity).
- Output naming: `-o 'out/{stem}.txt'` / `{page}` / `{n:03}` templates instead of printf `%d`; refuse to overwrite unless `--force`; `--in-place` writes atomically (temp file + rename) — atomicity is a recommendation, no source verified here.

### Gaps
- No primary source fetched on GNU parallel conventions or rayon ordering; rayon `par_iter().collect()` preserving order is from training knowledge.

## 4. Agent-readiness: schemas, MCP, security, what agents need

### Takeaway
As of Oct 2026 MCP's current stable revision is **2026-07-28** (stateless, `server/discover`, tasks moved to an extension, roots/sampling/logging deprecated), with `outputSchema`/`structuredContent` and untrusted annotations from 2025-06-18/2025-11-25; at the same time major vendors (Playwright) now tell coding agents a CLI + skills is more token-efficient than MCP. So: make the CLI itself the primary agent interface (stable JSON, schema introspection, concise output), and offer `lamina mcp` as a thin stdio adapter over the same command layer.

### Cited Findings
**MCP spec status**
- Published revisions: 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25, 2026-07-28, plus `draft` — [MCP docs.json](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/docs.json).
- 2025-11-25 changes: icons for tools/resources/prompts (SEP-973); tool-name guidance (SEP-986: 1–128 chars, case-sensitive, ASCII letters/digits/`_-.`); input validation errors must be Tool Execution Errors (`isError: true`), not protocol errors, so models can self-correct (SEP-1303); stdio servers may use stderr for all logging; JSON Schema 2020-12 default dialect; experimental tasks — [2025-11-25 changelog](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2025-11-25/changelog.mdx), [2025-11-25 tools](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2025-11-25/server/tools.mdx).
- 2026-07-28 changes: removes protocol sessions and the `initialize` handshake (every request carries protocol version/capabilities in `_meta`); new mandatory `server/discover`; `subscriptions/listen`; tasks moved to extension `io.modelcontextprotocol/tasks`; Multi Round-Trip Requests (`resultType: "input_required"`) replace server-initiated sampling/elicitation; every result has `resultType`; `tools/list` must not vary per connection and SHOULD be deterministic; `ttlMs`/`cacheScope` on list results; `inputSchema`/`outputSchema` may use any JSON Schema 2020-12 keywords and `structuredContent` may be any JSON value (SEP-2106); **Roots, Sampling and Logging deprecated** — "pass directories or files via tool parameters, resource URIs, or server configuration instead of Roots… log to stderr (stdio)" — [2026-07-28 changelog](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2026-07-28/changelog.mdx), [2026-07-28 tools](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2026-07-28/server/tools.mdx).
- Structured output: `structuredContent` result field; a tool returning it "SHOULD also return the serialized JSON in a TextContent block" for back-compat; with `outputSchema`, "Servers MUST provide structured results that conform to this schema" and clients SHOULD validate — [2025-11-25 tools.mdx](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2025-11-25/server/tools.mdx).
- Annotations (`ToolAnnotations`): `title`; `readOnlyHint` (default false); `destructiveHint` (default **true**, meaningful only if not read-only); `idempotentHint` (default false); `openWorldHint` (default true) — [schema.ts 2025-11-25](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/2025-11-25/schema.ts). "Clients MUST consider tool annotations to be untrusted unless they come from trusted servers." — [tools.mdx](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2025-11-25/server/tools.mdx).
- Security: servers MUST validate inputs, implement access controls, rate-limit, sanitize outputs; clients SHOULD confirm sensitive ops, show inputs, validate results, time out, audit-log — [tools.mdx §Security Considerations](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2025-11-25/server/tools.mdx).
- Filesystem MCP server CVE-2025-53109/53110: sandbox bypass via symlinks and naive path-prefix matching; fix = canonicalize + allow-list paths — [OWASP MCP Top 10 / secondary blogs via search](https://owasp.org/www-project-mcp-top-10/2025/MCP05-2025%E2%80%93Command-Injection%26Execution), [mcpplaygroundonline](https://mcpplaygroundonline.com/blog/mcp-security-tool-poisoning-owasp-top-10-mcp-scan) (secondary). Tool-poisoning (instructions hidden in tool descriptions) and indirect prompt injection via returned content — same secondary sources (Invariant Labs research).
- GitHub's mitigation for injected content: github-mcp-server "Lockdown mode" is "a best-effort content filter intended to reduce the risk of prompt injection from untrusted repository content… not an authorization boundary"; `--read-only` "takes priority" over `--tools`; `--toolsets` to reduce context size — [github-mcp-server README](https://raw.githubusercontent.com/github/github-mcp-server/main/README.md).

**CLI vs MCP for coding agents**
- Playwright: "Modern coding agents increasingly favor CLI–based workflows exposed as SKILLs over MCP because CLI invocations are more token-efficient: they avoid loading large tool schemas and verbose accessibility trees into the model context"; MCP "remains relevant for specialized agentic loops that benefit from persistent state" — [playwright-mcp README](https://raw.githubusercontent.com/microsoft/playwright-mcp/main/README.md).
- MCP-serving CLIs: GitHub ships a separate `github-mcp-server stdio` binary (not `gh mcp`) — [github-mcp-server README](https://raw.githubusercontent.com/github/github-mcp-server/main/README.md); Docker `docker mcp gateway run --profile …` over stdio — [Docker docs](https://docs.docker.com/ai/mcp-catalog-and-toolkit/cli/); Claude Code `claude mcp serve`; Playwright via `npx @playwright/mcp`; a `sentry mcp` subcommand is only a proposal (July 2026 issue) — [getsentry/toolkit#1470](https://github.com/getsentry/toolkit/issues/1470); Codex's `codex mcp` manages client config (list/get/add/remove) rather than serving — [search summary](https://www.usecarly.com/blog/codex-mcp-servers/).
- Agent CLI guidance (blogs, 2026, variable rigor): `--json`, `--yes/--no-input`, `--quiet`, `--dry-run`, consistent verb–noun structure; hanging on a prompt "kills" the agent workflow; gh-style field selection reduces output — [dev.to: Writing CLI Tools That AI Agents Actually Want to Use](https://dev.to/uenyioha/writing-cli-tools-that-ai-agents-actually-want-to-use-39no), [Memori lessons (2026-02-28)](https://archit15singh.github.io/posts/2026-02-28-designing-cli-tools-for-ai-agents/), [deployhq](https://www.deployhq.com/blog/6-developer-clis-ai-coding-agents-use-well).

**Machine-readable command specs**
- usage (jdx): "Define your CLI once" in KDL; generates completions, Markdown, man pages and **typed TypeScript/Python SDKs**; Rust `usage-rs` derive (v6 per README) and a clap migration guide; `usage generate completion|markdown|manpage` — [jdx/usage README](https://raw.githubusercontent.com/jdx/usage/main/README.md); `usage-lib` crate is at 7.1.3 on crates.io (2026-10-29) — [crates.io](https://crates.io/crates/usage-lib).
- qpdf's `--json-help` (schema-by-example) — §1.

### Inferences
- Agent surface (recommended): `lamina schema [cmd] --json` emits JSON Schema 2020-12 for each command's input (flags) and output (`--json` shape) generated from the same Rust types via `schemars`; this is reused verbatim as MCP `inputSchema`/`outputSchema`, so CLI and MCP never drift. Also `lamina --help --json` alias, and a `usage` KDL export for completions/docs.
- `lamina mcp` (stdio only at first): tools = the read-only commands (`info`, `extract`, `inspect`, `check`, `diff`, `render`) with `readOnlyHint: true`, plus write commands with `destructiveHint` set honestly (`false` when output is a new file, `true` for `--in-place`). Require an explicit `--root DIR` allow-list (Roots is deprecated in 2026-07-28, so config not protocol), canonicalize paths, reject symlink escapes, never overwrite inputs unless started with `--allow-in-place`.
- Prompt injection from PDFs: extracted text is attacker-controlled. Wrap extracted content in clearly delimited JSON fields (`"content"`), never mix it into `"message"`/`"hint"` fields, report hidden/invisible text (white text, off-page, OCG-hidden, tiny font) as flags (`"visibility":"hidden"`), and offer `--visible-only`. These are recommendations; no source evaluated PDF-specific injection.
- Token economy: default `info` output under ~20 lines; `--fields a,b` (gh-style) and `--max-chars/--pages` on `extract`; never invoke a pager unless stdout is a TTY; `--quiet` suppresses progress; errors one line + JSON form.

### Gaps
- No quantitative evidence found on token budgets per tool call for Claude Code/Codex/Cursor (search found no such source).
- Whether `gh` exposes a `--help --json`-style command spec could not be confirmed; none found.
- Whether 2026-07-28 is labelled "stable/latest" on the site: rmcp's README calls it "the stable MCP 2026-07-28 specification"; the spec site itself wasn't reachable.

## 5. Rust implementation and distribution

### Takeaway
clap 4 derive + clap_complete + clap_mangen, annotate-snippets/miette-style diagnostics on stderr, indicatif progress (stderr, hidden when not a TTY), anstream for NO_COLOR-aware colour, schemars for schemas, rmcp 3.x for `lamina mcp`, cargo-dist for GitHub releases/Homebrew/installers, and the biome/esbuild npm pattern (meta package + per-platform `optionalDependencies`).

### Cited Findings
- Latest stable versions on crates.io (read 2026-10-29): clap 4.6.7 (2026-09-14), clap_complete 4.6.11, clap_complete_nushell 4.6.2, clap_mangen 0.3.3, miette 7.6.0 (last updated 2025-04-27 — older), annotate-snippets 0.12.17 (2026-10-19), indicatif 0.18.6, anstream 1.0.0 (2026-02-11), schemars 1.2.3, rmcp 3.8.0 (2026-10-26), cargo-dist 0.32.0 (2026-05-22) — [crates.io API](https://crates.io/api/v1/crates/clap) (same endpoint per crate).
- rmcp: "An official Rust Model Context Protocol SDK implementation with tokio async runtime"; "implements the stable MCP 2026-07-28 specification while remaining fully compatible with the 2025-11-25 release"; `#[tool]`, `#[tool_router]`, `#[tool_handler]` macros, `Parameters<T>` with schemars-derived input schemas, `serve(stdio())`; 3.x migration guide exists — [rust-sdk README](https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/README.md).
- cargo-dist (dist): plans releases, builds binaries and "installers", tarballs — [cargo-dist README](https://raw.githubusercontent.com/axodotdev/cargo-dist/main/README.md); installer list (shell, PowerShell, Homebrew, npm, MSI) at [dist book/installers](https://axodotdev.github.io/cargo-dist/book/installers/index.html) (not fetched; list from training knowledge).
- npm per-platform pattern: `@biomejs/biome` has `bin: biome` and `optionalDependencies` on `@biomejs/cli-{win32,darwin,linux}-{x64,arm64}`, `linux-*-musl`, and `android-{x64,arm64}` (v2.6.4) — [biome package.json](https://raw.githubusercontent.com/biomejs/biome/main/packages/%40biomejs/biome/package.json).

### Inferences
- Crate layout: `crates/lamina-cli` with `layer = "app"` (or `edge`), depending on core crates; core stays free of clap/tokio/rayon. Put command logic in a library module (`commands::info(args) -> Result<InfoReport>`) consumed by both the clap front-end and the rmcp tools; `tokio` only behind a `mcp` cargo feature.
- Diagnostics: annotate-snippets (rustc's renderer, actively maintained per crates.io dates) is preferable to miette for pointing at byte offsets in the PDF (`file.pdf:offset 18342: xref entry 12 points past EOF`).
- WASI: since core is bytes-in/bytes-out, a `wasm32-wasip2` build of the CLI could run via `npx`/wasmtime with preopened dirs as a natural sandbox (recommendation; no source verified this session).
- Licence check: cargo-dist, clap, rmcp are MIT/Apache per their repos (verify via `cargo deny` per ADR 0019).

### Gaps
- winget manifest generation support in dist, and Homebrew-core vs tap policy, not verified.
- No source fetched on esbuild's postinstall fallback vs biome's pure optionalDependencies approach.

## 6. Proposed command set with example sessions (recommendations)

### Takeaway
A verb-first single binary, `lamina <command> [inputs…] [-o OUT|--in-place] [--json]`, with qpdf page ranges, qpdf-style versioned JSON, and the same commands exposed over `lamina mcp`.

### Cited Findings
- Grammar precedents: mutool verbs (`info`, `pages`, `merge`, `extract`, `show`, `sign`, `clean`, `draw/convert`) — [mutool.1](https://raw.githubusercontent.com/ArtifexSoftware/mupdf/master/docs/man/mutool.1); qpdf ranges/`--split-pages`/`--rotate=+90:1-3`/`--replace-input` — [qpdf cli.rst](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst); pdftk handles and `shuffle` — [pdftk man](https://leancrew.com/all-this/man/man1/pdftk.html); `-` for stdio, `--force`, `--dry-run`, `--no-input` — [clig.dev](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md).

### Inferences
Command set (all accept `-` for stdin input and `-o -` for stdout; never write to stdout when it is a TTY and output is binary — print an error instead):

| Command | Purpose | MCP annotation |
|---|---|---|
| `info` | version, pages, encryption, metadata, forms/signatures present, warnings | readOnly |
| `pages merge/split/select/rotate/reorder/shuffle/delete` | page algebra (qpdf ranges, pdftk handles) | additive (new file) |
| `extract text\|markdown\|json\|images\|tables\|attachments\|fonts` | content | readOnly |
| `render` | page → PNG/SVG (`--dpi`, `--pages`) | readOnly |
| `fill` | AcroForm from JSON (`lamina fill f.pdf --data data.json`), `fill --list-fields` | additive |
| `sign` / `verify` | PAdES sign; verify signatures/DocMDP | sign additive, verify readOnly |
| `encrypt` / `decrypt` | AES-256 R6; passwords via `--password-file`/env, never argv by default | additive |
| `optimize` | object streams, dedup, recompress, subset | additive |
| `check` / `repair` | validation (exit 3 on warnings) / rewrite with repair log | check readOnly |
| `diff` | text/visual/structural diff of two PDFs | readOnly |
| `inspect` | object-level: trailer, `obj 12`, `--path /Root/Pages/Kids/0`, `--raw` (mutool show analogue) | readOnly |
| `convert` | images→PDF, PDF→PDF/A (later) | additive |
| `schema`, `mcp`, `completions`, `help exit-codes` | tooling | — |

Example session (human):
```console
$ lamina info report.pdf
report.pdf  PDF 1.7 · 42 pages · 1.8 MB · not encrypted · tagged
Title: Q3 Report   Producer: LibreOffice 7.6
Forms: 0 fields   Signatures: 1 (valid? run `lamina verify`)
warning[W0102]: xref offset off by 3 bytes (repaired)
$ echo $?
3
$ lamina pages select report.pdf 1-5,z,r3 -o summary.pdf
$ lamina pages merge cover.pdf report.pdf:2-z appendix.pdf -o full.pdf
$ lamina pages shuffle A=fronts.pdf B=backs.pdf A Bz-1 -o duplex.pdf
$ lamina pages rotate scan.pdf +90:1-3,x2 --in-place
error[E0004]: refusing to modify scan.pdf in place without --force (stdin is not a terminal)
$ lamina pages split big.pdf --every 10 -o 'parts/{stem}-{n:03}.pdf'
```

Pipes:
```console
$ curl -sL https://example.org/a.pdf | lamina extract text - --pages 1-3 | rg -i invoice
$ lamina pages select in.pdf 1-z:odd -o - | lamina render - --pages 1 --dpi 72 -o thumb.png
$ lamina fill form.pdf --data - -o filled.pdf < answers.json
$ fd -e pdf -0 | xargs -0 -n1 -P8 lamina check --json --quiet
$ lamina check docs/*.pdf --jobs 8 --json | jq -c 'select(.type=="result" and .status!="ok")'
```

Agent session (non-TTY, JSON):
```console
$ lamina info report.pdf --json --fields pages,encrypted,warnings
{"schema":"lamina.info/1","input":"report.pdf","pages":42,"encrypted":false,
 "warnings":[{"code":"W0102","message":"xref offset off by 3 bytes (repaired)","offset":18342}]}
```

NDJSON batch:
```json
{"type":"result","schema":"lamina.check/1","index":0,"input":"a.pdf","status":"ok","warnings":[]}
{"type":"result","schema":"lamina.check/1","index":1,"input":"b.pdf","status":"error","error":{"code":"E0201","kind":"password_required"}}
{"type":"summary","ok":1,"warnings":0,"failed":1,"elapsed_ms":412}
```

Error JSON shape (stderr when `--json`; also the MCP `isError` text/structuredContent):
```json
{
  "schema": "lamina.error/1",
  "error": {
    "code": "E0201",
    "kind": "password_required",
    "message": "b.pdf is encrypted (AES-256, R6); a user password is required",
    "input": "b.pdf",
    "location": {"offset": 1024, "object": "12 0 R"},
    "hint": "pass --password-file FILE or set LAMINA_PASSWORD",
    "exit_code": 5,
    "retryable": false,
    "docs": "https://lamina.dev/errors/E0201"
  }
}
```
Rules: `code`/`kind` stable and listed in `fixtures/codes.json` (lamina already tests warning/error codes against Rust enums, per repo CLAUDE.md); `message` human; `hint` is the actionable next step for an agent; document-derived strings never appear in `message`/`hint`; unknown keys must be ignored by consumers (qpdf rule); bump `/N` only on breaking change.

MCP:
```console
$ lamina mcp --root ./docs --read-only        # stdio; tools: lamina_info, lamina_extract_text, lamina_render, …
```
Tool definitions reuse `lamina schema extract.text --json` as `inputSchema`/`outputSchema`, annotations as in the table, deterministic tool order.

### Gaps
- Naming/branding of the binary (`lamina` vs `lam`) and whether `pages` sub-verbs vs flat verbs (`merge`, `split`) test better with users — no evidence gathered; worth a small usability check.
- PAdES/LTV `verify` semantics and PDF/A `convert` depend on future ADRs not yet accepted.
