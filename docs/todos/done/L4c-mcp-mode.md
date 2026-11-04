# L4c — `laminapdf mcp`: a stdio MCP server over the CLI commands, with `--root` and read-only defaults

| Field | Value |
| --- | --- |
| ID | `L4c` |
| Status | `done` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L4a](./L4a-lamina-cli.md) |
| Blocks | None |
| ADRs | Implements 0031's MCP security defaults (from [L4a](./L4a-lamina-cli.md)). Follows 0026 (errors), [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits` on every call), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (rmcp and tokio) |
| Branch | `feat/l4c-mcp-mode` |
| PR title | `feat(cli): laminapdf mcp stdio server with root allow-list, read-only default and honest annotations` |
| Size | M (≈ 300–500 lines plus tests) |
| PR | [#225](https://github.com/laudebugs/laminapdf/pull/225) |

## Goal

Let MCP clients use laminapdf without a second implementation. `laminapdf mcp` is a stdio MCP server, built behind a cargo feature, that exposes the L4a commands as tools with the same input and output schemas and calls the same `commands::*` functions. It is safe by default: it only touches files under the `--root` directories, resolves symlinks before checking, is read-only unless told otherwise, and keeps document text out of everything but data fields. The research found agents increasingly prefer CLIs, so this stays a thin adapter; it exists for clients that only speak MCP.

## Context

- [Report: MCP 2026-07-28 is stateless, and agents increasingly prefer CLIs](../../research/libraries/shareable-libraries.md#mcp-2026-07-28-is-stateless-and-agents-increasingly-prefer-clis): `server/discover`, no `initialize`, deterministic `tools/list`, validation errors as tool errors, `outputSchema`, untrusted annotations, the filesystem server's symlink CVEs, github-mcp-server's `--read-only`.
- [CLI spec note §4](../../research/libraries/notes/cli-spec.md#4-agent-readiness-schemas-mcp-security-what-agents-need): spec details, annotation defaults (`destructiveHint` defaults to true), prompt-injection recommendations.
- [CLI spec note §5](../../research/libraries/notes/cli-spec.md#5-rust-implementation-and-distribution): rmcp 3.x (implements 2026-07-28, compatible with 2025-11-25), `#[tool]` macros, `serve(stdio())`.
- ADR 0031 (from [L4a](./L4a-lamina-cli.md)): the tool mapping and security defaults this PR implements.
- [Shareable libraries plan §10 risks](../../plans/shareable-libraries.md#10-risks): MCP spec churn and prompt injection.

## Scope

**In scope**

- Cargo feature `mcp` on `laminapdf-cli` (off by default for library users of the crate, on in release binaries via L4d). `tokio` and `rmcp` are optional dependencies behind it; the rest of the CLI stays synchronous.
- `laminapdf mcp --root DIR [--root DIR…] [--allow-write [--allow-in-place]] [--max-output-bytes N] [--timeout SECS]` over stdio, targeting MCP **2026-07-28** (stateless requests, `server/discover`) through a pinned rmcp 3.x, with 2025-11-25 compatibility as rmcp provides it. Logs go to stderr only.
- Tools mapped 1:1 to commands: `laminapdf_info`, `laminapdf_check`, `laminapdf_extract` (`format: "text"` until L4b adds the others), `laminapdf_explain` (read-only), and `laminapdf_pages_rotate` (write). `inputSchema` and `outputSchema` are the `schemars` schemas of the command's args and report (the same ones `laminapdf schema` prints after L4b). Results carry `structuredContent` conforming to `outputSchema` and the same JSON as a text block.
- `tools/list` is deterministic: a fixed order (alphabetical by tool name), and it does not vary per connection. Read-only is the default (there is no `--read-only` flag): write tools are not listed at all unless `--allow-write` is given.
- Honest annotations: read tools `readOnlyHint: true`, `openWorldHint: false`; `laminapdf_pages_rotate` `readOnlyHint: false`, `destructiveHint: false` when writing a new file. In-place writes are only possible with `--allow-in-place`, and then the tool's description and `destructiveHint: true` say so. Tool descriptions are static strings; they never include file names or document content.
- Path policy: every path argument is resolved against the roots, canonicalised (`std::fs::canonicalize`, which resolves symlinks), and refused unless the result is inside a canonicalised root. Output paths: the parent directory is canonicalised and checked, the file must not exist (or `--allow-in-place` and it is the input), and the write is atomic. `..`, absolute paths outside the roots and symlinks pointing out are all refused. `--root` is required; starting without it is exit 2.
- Prompt-injection hygiene: document text appears only in data fields (`text`, `title`, …) of `structuredContent`; error `message` and `hint` never contain it (ADR 0026). Responses over `--max-output-bytes` are truncated with `"truncated": true`.
- Errors: argument validation failures, path refusals and lamina errors are tool results with `isError: true` and the `laminapdf.error/1` document (so the model can correct itself); only malformed JSON-RPC is a protocol error.
- Every call runs with default `Limits` and a timeout; a timed-out call is a tool error. The timeout **trips the job's `CancelToken`** (ADR 0028) so the work stops at its next `Job::step`; dropping the future alone would leave a blocking thread decoding. A small concurrency cap (e.g. the CPU count) bounds in-flight calls.

**Out of scope** (and which todo covers it)

- The `laminapdf_render` tool and the Markdown/JSON formats of `laminapdf_extract`: added when [L4b](../ready/L4b-cli-batch-render-schema.md) merges (one line each in the tool router; whichever of L4b and L4c merges second adds them).
- HTTP transports, auth, resources and prompts: not planned; stdio only.
- MCP tasks extension for long jobs: later, if render or extract calls need it.

## Tasks

- [x] Load `/rust-best-practices`; follow [docs/guidelines/rust.md](../../guidelines/rust.md).
- [x] Add `rmcp` (pinned 3.x, features for server + stdio + schemars) and `tokio` (minimal features: `rt`, `io-std`, `macros`) as optional dependencies behind `mcp`. Check licences: expected MIT and/or Apache-2.0; confirm with `cargo xtask deny` (ADR 0019).
- [x] Add `src/mcp/mod.rs`: a server struct holding the roots and flags; `#[tool_router]` with one `#[tool]` per command that runs `commands::*` as a stepped job inside `spawn_blocking`, holding its `CancelToken`; on timeout, cancel the token and wait for the step loop to return `cancelled` before replying. A semaphore caps concurrent calls.
- [x] Add `src/mcp/roots.rs`: root canonicalisation at start-up, input and output path checks, unit-tested on a temporary tree with symlinks (Unix) and junctions where possible (Windows).
- [x] Map `CliError` to tool errors with `isError: true` and the error document.
- [x] Wire `laminapdf mcp` into the clap front end under `#[cfg(feature = "mcp")]`; without the feature, `laminapdf mcp` says it was built without MCP and exits 2.
- [x] Document client configuration (Claude Code, Claude Desktop, other clients) in the crate README with a JSON snippet.
- [x] Run the MCP inspector in CLI mode against the server and record the result in the PR.

### Example

```console
$ laminapdf mcp --root ./docs            # read-only: laminapdf_check, laminapdf_explain, laminapdf_extract, laminapdf_info
$ laminapdf mcp --root ./docs --allow-write   # adds laminapdf_pages_rotate (new files only)
```

```json
{"mcpServers":{"laminapdf":{"command":"laminapdf","args":["mcp","--root","/home/me/docs"]}}}
```

## Acceptance criteria

- [x] An integration test spawns `laminapdf mcp` and, over stdio, calls `server/discover`, `tools/list` and each tool; outputs validate against each tool's `outputSchema`.
- [x] `tools/list` is byte-identical across two runs and lists no write tool without `--allow-write`.
- [x] Paths outside `--root`, `..` escapes and a symlink inside the root pointing outside are refused as tool errors (`isError: true`), and nothing outside the root is read or written (test checks the file system).
- [x] An invalid argument (bad page range) is a tool error with the `laminapdf.error/1` document, not a protocol error.
- [x] A fixture with an injection string in its title and text: the string appears only in data fields, never in `message`, `hint` or tool descriptions.
- [x] The MCP inspector's checks pass (recorded in the PR; automate in CI if the inspector's CLI mode supports 2026-07-28 without network access).
- [x] `cargo build -p laminapdf-cli` without `mcp` pulls in no tokio or rmcp (`cargo tree` check in a test or xtask).
- [x] `cargo xtask check` passes, including cargo-deny.

## Post-merge checks

None.

## Tests

- `crates/laminapdf-cli/tests/mcp.rs` (feature-gated): JSON-RPC over the child's stdin/stdout, using rmcp's client or hand-written requests.
- Unit tests in `src/mcp/roots.rs` for the path policy (temporary directories, symlinks; Windows-specific cases behind `cfg(windows)`).
- Reuse `fixtures/cli/` from L4a (injection title, encrypted file).

## Docs to update

- [x] [Shareable libraries plan §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1): tick "`laminapdf mcp` (rmcp, stdio), `--root`, `--read-only`, honest tool annotations"
- [x] `crates/laminapdf-cli/README.md` (MCP section, client config, security model); ADR 0031 if any default changed during the work
- [x] `CLAUDE.md` (the `mcp` feature)

## Risks and open questions

- **MCP spec churn:** 2026-07-28 changed a lot from 2025-11-25. Pin rmcp, keep the adapter thin, and let the CLI stay the primary interface.
- **TOCTOU on paths:** a symlink can change between check and open. Recommendation: open the canonical path once and read from that handle; on Unix consider `openat`-style checks later (needs `unsafe` or a crate; out of scope unless cheap).
- **Bytes over MCP:** some clients cannot share files and would want base64 input. Not in scope; decide after user feedback (it bypasses `--root` but adds size limits).
- **Annotations are hints:** clients must treat them as untrusted; the real protection is `--read-only` and `--root`.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
