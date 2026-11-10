# L4f — CLI downloads: a download table on the docs site's install page

| Field | Value |
| --- | --- |
| ID | `L4f` |
| Status | `ready` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L4d](../done/L4d-cli-distribution.md), [L4g](../ongoing/L4g-cli-release-targets-and-publishing.md) |
| Dependency notes | L4g builds and publishes all eight targets to the GitHub release, npm and the Homebrew tap. The links in this todo's table answer 200 only once the first `laminapdf-cli-v*` release exists. |
| ADRs | Follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (checksums), [0031](../../decisions/0031-cli-and-mcp-contract.md) (the binary and its packages) |
| Branch | `docs/l4f-cli-download-table` |
| PR title | `docs(cli): install the command-line tool from the docs site` |
| Size | S (≈ 100–200 lines: one docs section, a link check, README lines) |
| PR | — |

## Goal

Let a reader install the `laminapdf` command-line tool from the docs site's install page. The page gains a "Command-line tool" section with Homebrew, npm and a table of direct downloads, one row per target, each linking the archive and its checksum on the GitHub release.

The repository is public, so the GitHub release serves the archives to everyone. This todo first planned an R2 bucket at `downloads.laminapdf.com` because release links answered 404 while the repository was private. That reason is gone, so the bucket, the formula rewrite and the ADR 0011 amendment are dropped.

## Context

- [L4g](../ongoing/L4g-cli-release-targets-and-publishing.md): `cli-release.yml` attaches `laminapdf-cli-<target>.tar.xz` (`.zip` on Windows), each with a `.sha256`, plus `sha256.sum`, to the `laminapdf-cli-v<version>` GitHub release.
- GitHub serves the newest release's files at `https://github.com/laudebugs/laminapdf/releases/latest/download/<file>`, but "latest" means the newest non-pre-release across every tag in the repository, and CLI releases are marked pre-release. Versioned links (`releases/download/laminapdf-cli-v<version>/<file>`) avoid both problems.
- `apps/docs/src/content/docs/install.mdx` (the install page) and its `{{version}}` substitution.
- [D7b](./D7b-settings-renderer-switch-and-download-page.md): the website's `/download` page, for the desktop app; it links this section.

## Scope

**In scope**

- `install.mdx`: a "Command-line tool" section with Homebrew (`brew install laudebugs/tap/laminapdf`), npm (`npm install -g @laminapdf/cli`, `npx @laminapdf/cli`) and a table of the eight targets, each row linking its archive and `.sha256` on the versioned release URL.
- The version: the CLI shares the workspace's lockstep version, so the links use `{{version}}`.
- A docs-build check: the table's targets match `dist-workspace.toml`'s, so a target added there fails the build until the page lists it.
- `crates/laminapdf-cli/README.md`: the "Not yet published" note and the `cargo install --git` line point at the docs page once the first release exists.
- Unsigned binaries: the section says macOS quarantines a browser download, shows the `xattr -d com.apple.quarantine` command, and says Homebrew and npm avoid the warning.

**Out of scope** (and which todo covers it)

- The desktop downloads and `downloads.laminapdf.com`: [D5a](./D5a-update-worker-and-adr-0014.md).
- The website's `/download` page: [D7b](./D7b-settings-renderer-switch-and-download-page.md).
- Signing and notarisation: [D6c](./D6c-macos-signing-and-notarisation.md), [D6d](./D6d-windows-signing.md).
- A shell installer (`curl … | sh`): later, on demand.

## Tasks

- [ ] Write the install page's "Command-line tool" section and its download table.
- [ ] Add the docs-build check of the table against `dist-workspace.toml`.
- [ ] Update the CLI README's install lines.

## Acceptance criteria

- [ ] The install page lists all eight targets, each linking its archive and checksum on the release for the current version.
- [ ] After the first CLI release, `curl -fsSIL` on every link in the table answers 200 without GitHub credentials.
- [ ] The docs build fails if the page's targets differ from `dist-workspace.toml`'s.

## Tests

- The docs-build check above, with a test that a missing target fails it.

## Docs to update

- `apps/docs/src/content/docs/install.mdx`; `crates/laminapdf-cli/README.md`.
- [Shareable libraries plan §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1): tick this todo's line.

## Risks and open questions

- **First release:** until the owner finishes L4g's setup and tags a release, every link answers 404. Merge this todo after that release, or hide the table behind the release's existence.

## Definition of done

- [ ] Claimed: the work PR's first commit moves this file `ready/` → `ongoing/` (Status `ongoing`, links updated)
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
