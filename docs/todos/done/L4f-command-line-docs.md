# L4f — Command line: a section of the docs site for the `laminapdf` tool

| Field | Value |
| --- | --- |
| ID | `L4f` |
| Status | `done` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L4d](./L4d-cli-distribution.md), [L4g](./L4g-cli-release-targets-and-publishing.md) |
| Dependency notes | L4g publishes all eight targets to the GitHub release, npm and the Homebrew tap; the first release, `laminapdf-cli-v0.2.0-alpha.5`, exists, so the download links answer. |
| ADRs | Follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (checksums), [0031](../../decisions/0031-cli-and-mcp-contract.md) (the binary, its commands, exit codes, JSON and MCP) |
| Branch | `docs/l4f-command-line-docs` |
| PR title | `docs(cli): a Command line section on the docs site` |
| Size | M (≈ 600–900 lines: seven pages, a sidebar group, a build check, README lines) |
| PR | [#415](https://github.com/laudebugs/laminapdf/pull/415) |

## Goal

Give the `laminapdf` command-line tool its own section on the docs site. Today the only documentation is `crates/laminapdf-cli/README.md` on GitHub, so a reader of the docs site can't find out the tool exists, how to install it or what it does.

The section installs the tool (Homebrew, npm and a table of direct downloads, each linking its archive and checksum on the GitHub release), then walks through the tasks it does, the conventions scripts rely on, and the MCP server for AI agents.

Rescoped on 2026-11-10 from "a download table on the install page": the owner asked for a whole Command line section, which takes in the download table.

## Context

- `crates/laminapdf-cli/README.md`: every command, many inputs, output templates, page ranges, exit codes, JSON and the MCP server. The pages tell the same facts as tasks, for readers who don't read the source tree.
- [L4g](../done/L4g-cli-release-targets-and-publishing.md): `cli-release.yml` attaches `laminapdf-cli-<target>.tar.xz` (`.zip` on Windows), each with a `.sha256`, plus `sha256.sum`, to the `laminapdf-cli-v<version>` GitHub release.
- GitHub's `releases/latest/download/<file>` means the newest non-pre-release across every tag in the repository, and CLI releases are pre-releases, so the links name the version: `releases/download/laminapdf-cli-v<version>/<file>`.
- The CLI's version is the workspace's lockstep version, so the links use the pages' `{{version}}`.
- `apps/site/src/data/sources.ts` reads the install page's install lines and package table, and the CLI README's console examples.

## Scope

**In scope**

- A "Command line" sidebar group after "Read and edit PDFs":
  - **Overview** (`cli/`): what the tool does, a short tour, and where each task lives.
  - **Install** (`cli/install`): Homebrew, npm, the download table of the eight targets, checksums, macOS quarantine on browser downloads, shell completions and man pages.
  - **Inspect and check PDFs** (`cli/inspect`): `info`, `check` (repairs and failed redactions), `reveal` and `explain`.
  - **Extract and render** (`cli/extract-and-render`): `extract` in each format, `render` with output templates.
  - **Make and change PDFs** (`cli/build-and-edit`): `build` from Markdown with fonts and profiles, `pages rotate`.
  - **Scripts and batch jobs** (`cli/scripts`): standard input and output, page ranges, many inputs, `--json` and its schemas, exit codes, passwords and limits.
  - **AI agents (MCP)** (`cli/mcp`): `laminapdf mcp`, client configuration, roots, write flags and what the server withholds.
- The install page: a short "Command-line tool" section that links `cli/install`.
- A docs-build check: the download table's targets match `dist-workspace.toml`'s, with a test that a missing target fails it.
- `crates/laminapdf-cli/README.md`: the "Not yet published" note goes; the install lines and the README link the docs pages.

**Out of scope** (and which todo covers it)

- The desktop downloads and `downloads.laminapdf.com`: [D5a](../done/D5a-update-worker-and-adr-0014.md).
- The website's `/download` page: [D7b](../ready/D7b-settings-renderer-switch-and-download-page.md).
- Signing and notarisation: [D6c](../ready/D6c-macos-signing-and-notarisation.md), [D6d](../ready/D6d-windows-signing.md).
- A shell installer (`curl … | sh`): later, on demand.
- JSON documents in `laminapdf build`: T4k (from T4e's PR) adds them to these pages.
- Pages generated from clap's help: the man pages already are; these pages are task guides.

## Tasks

- [x] The seven pages and the sidebar group.
- [x] The install page's "Command-line tool" section.
- [x] The docs-build check of the download table against `dist-workspace.toml`, and its test.
- [x] The CLI README's install lines.

## Acceptance criteria

- [x] The install page lists all eight targets, each linking its archive and checksum on the release for the current version.
- [x] `curl -fsSIL` on every link in the table answers 200 without GitHub credentials.
- [x] The docs build fails if the page's targets differ from `dist-workspace.toml`'s.
- [x] Every command example on the pages runs against the current CLI as shown.
- [x] `cargo xtask check docs` passes; the pages appear in the sidebar, `llms.txt` and their Markdown twins.

## Tests

- The docs-build check above, with a test that a missing target fails it.

## Docs to update

- `apps/docs/src/content/docs/install.mdx`; `crates/laminapdf-cli/README.md`; `apps/docs/plugins/sidebar.mjs`.
- [Shareable libraries plan §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1): tick this todo's line.

## Risks and open questions

- **Version drift:** the links use the docs' version. If the library releases without the CLI, the links name a CLI release that doesn't exist. [L4h](../done/L4h-cli-release-train.md) releases the CLI with every library release, which closes the gap.
- **Two homes for the same facts:** the README and the pages both describe the commands. The README keeps the contributor's view (code, tests, release) and links the pages for the rest.

## Definition of done

- [x] Claimed: the work PR's first commit moves this file `ready/` → `ongoing/` (Status `ongoing`, links updated)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **Seven pages**, every command and output copied from a run of the CLI on `main`; page ranges checked against `extract`. The published 0.2.0-alpha.5 binary predates `build`, which the next CLI release brings.
- **The download check** (`apps/docs/plugins/cli-downloads.mjs`) runs at config setup and compares the table's archive and checksum links with `dist-workspace.toml`'s targets; all 16 links answered 200 for 0.2.0-alpha.5.
- **Found on the way:** `laminapdf help exit-codes` left 6 out of its priority order; it now comes from `ExitCode::by_priority`.
- **The Homebrew formula** installs neither man pages nor completions; the install page says how to read the man pages from the archive and that `laminapdf completions` writes completions.
