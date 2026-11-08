# L4f — CLI downloads: release binaries on `downloads.laminapdf.com`, linked from the docs site

| Field | Value |
| --- | --- |
| ID | `L4f` |
| Status | `ready` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L4d](../ongoing/L4d-cli-distribution.md) |
| Dependency notes | Uses the host that [ADR 0011 §1](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology) reserves for desktop downloads; [D5a](./D5a-update-worker-and-adr-0014.md) later adds the desktop files to the same bucket and need not wait for this todo. |
| ADRs | Amends [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the downloads host also serves the CLI); follows 0017 (release gating), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (checksums, SBOMs, notices) |
| Branch | `build/l4f-cli-downloads` |
| PR title | `build(cli): serve release binaries from downloads.laminapdf.com and link them from the docs` |
| Size | M (≈ 300–500 lines: upload step, formula rewrite, docs page, tests) |
| PR | — |

## Goal

Let anyone download the `laminapdf` binary from Lamina PDF's own domain while the repository is private. L4d attaches the archives to a GitHub release, and dist's Homebrew formula points at those GitHub URLs, so both answer 404 to everyone but collaborators. This todo copies every release archive to an R2 bucket served at `downloads.laminapdf.com`, points the Homebrew formula there, and adds a download table to the docs site's install page. npm needs nothing: the registry is public whatever the repository is.

## Context

- [L4d](../ongoing/L4d-cli-distribution.md): `cli-release.yml` builds the archives, `sha256` checksums and the formula with cargo-dist (`hosting = "github"` in `dist-workspace.toml`); its `publish` job runs only on a `laminapdf-cli-v*` tag while `LAMINA_PDF_PUBLISH_CLI` is `true`.
- [ADR 0011 §1](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology) and the [website plan's host table](../../plans/website.md): `downloads.`, `staging-downloads.` and `nightly-downloads.laminapdf.com` are reserved for the desktop downloads (ADR 0014, D5a).
- [Desktop plan §5](../../plans/desktop-app.md#5-design-summary): desktop releases go to R2 under `releases/{version}/{target}-{arch}/`, served through an R2 custom domain.
- [Cloudflare notes](../../research/architecture/notes/cloudflare.md): R2 pricing and limits, custom domains, no egress fees.
- `apps/docs/src/content/docs/install.mdx` (the install page) and `apps/docs/README.md` (`LAMINA_PDF_DOCS_GITHUB` stays off while the repository is private, so the docs link nothing on GitHub).
- [D7b](./D7b-settings-renderer-switch-and-download-page.md): the website's `/download` page, for the desktop app.

## Scope

**In scope**

- **Bucket layout** (recommendation): one R2 bucket, `lamina-pdf-downloads`, with the CLI under `cli/`:
  - `cli/<version>/<archive>` and `cli/<version>/sha256.sum`: immutable, cached for a year.
  - `cli/latest/laminapdf-<target>.tar.xz` (`.zip` on Windows), `cli/latest/sha256.sum` and `cli/latest/release.json` (version, date, and per archive its target, size and SHA-256): replaced on each release, cached for five minutes.
  - The versioned copy is the record; `latest/` holds names without the version, so the docs can link them without a rebuild.
- **Upload:** a step in `cli-release.yml`'s `publish` job, after the GitHub release, uploads the archives with Wrangler (pinned as the docs workflow pins it). It refuses to overwrite an existing `cli/<version>/` object, and it writes `latest/` last, so a failed upload leaves the previous release in place. A dry run prints the object keys and writes nothing.
- **Homebrew:** the formula's archive URLs point at `downloads.laminapdf.com/cli/<version>/`. dist 0.32 writes GitHub URLs for `hosting = "github"`, so `tools/cli-release/` rewrites the generated formula and checks that every URL and checksum matches the uploaded archives.
- **Docs site:** `install.mdx` gains a "Command-line tool" section with Homebrew, npm and a table of direct downloads per platform, each linking `cli/latest/` and the checksum file. The page names the version with `{{version}}` only if the CLI's version stays the fixed group's; otherwise it reads `release.json` in the browser (an inline script needs its hash in the CSP, which `@laminapdf/site-kit` fills in). The README's "Not yet published" note and its `cargo install --git` line, which fails for anyone outside the repository, point at the docs page instead.
- **ADR 0011 amendment:** the downloads row also covers the CLI, and the bucket follows ADR 0011's rules: no logs beyond Cloudflare's own analytics, no cookies.
- **Owner setup**, written in `tools/cli-release/README.md`: create the bucket, attach `downloads.laminapdf.com` as its custom domain, and add a Cloudflare API token with R2 write access to that bucket alone as the `release` environment's secret.

**Out of scope** (and which todo covers it)

- The desktop downloads, the update Worker and ADR 0014: [D5a](./D5a-update-worker-and-adr-0014.md).
- The website's `/download` page: [D7b](./D7b-settings-renderer-switch-and-download-page.md) adds a link to the CLI section here.
- Signing and notarisation: [D6c](./D6c-macos-signing-and-notarisation.md), [D6d](./D6d-windows-signing.md). Until they land, the docs say that macOS quarantines a browser download and show the `xattr` command, beside the Homebrew line that avoids it.
- Nightly CLI builds on `nightly-downloads.laminapdf.com`: later, on demand.

## Tasks

- [ ] Get the owner's answers to the open questions below.
- [ ] Add the upload step and the `release.json` writer to `cli-release.yml` and `tools/cli-release/`, with the dry-run path.
- [ ] Rewrite the Homebrew formula's URLs to the downloads host and check them against the archives.
- [ ] Write the docs section and update the CLI README's install lines.
- [ ] Amend ADR 0011's host table; write the owner setup in `tools/cli-release/README.md`.

## Acceptance criteria

- [ ] A dry run on `main` lists every object key it would write, with sizes and SHA-256, and writes nothing.
- [ ] After the first real release, `curl -fsSI` on each `cli/latest/` archive answers 200 from outside the repository's access, and each archive matches its line in `sha256.sum`.
- [ ] `brew install laudebugs/tap/laminapdf` works on a Mac with no GitHub credentials.
- [ ] The docs build fails if the install page links a target that `dist-workspace.toml` lacks.

## Tests

- Unit tests for the formula rewrite and `release.json`, in `packages/cli`'s Vitest suite, which already tests `tools/cli-release/generate.ts`.
- A `tools/xtask/src/workflows.rs` test: the upload step runs only in the `publish` job, after the GitHub release, and only on a tag push.

## Docs to update

- `apps/docs/src/content/docs/install.mdx`; `crates/laminapdf-cli/README.md`; a new `tools/cli-release/README.md`.
- [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md)'s host table; the [website plan](../../plans/website.md)'s host table.
- [Shareable libraries plan §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1): tick this todo's line.

## Risks and open questions

- **Host:** R2 behind `downloads.laminapdf.com` (recommended: Cloudflare already serves every site, R2 has no egress fees, and D5a uses the same host), or GitHub releases once the repository is public?
- **One bucket for the CLI and the desktop app**, with `cli/` and `desktop/` prefixes (recommended), or one each?
- **A shell installer** (`curl -fsSL https://downloads.laminapdf.com/cli/install.sh | sh`): dist can write one, with the same URL rewrite. Is it wanted, or are Homebrew, npm and the table enough?
- **Version:** does the CLI keep the fixed group's version (and so `{{version}}` on the docs page), or its own `laminapdf-cli-v*` line?
- **Cost:** R2 storage at about 10 MB per archive and eight archives per release stays within the free tier for years; old versions can expire after a year through a lifecycle rule.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
