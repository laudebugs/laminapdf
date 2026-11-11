# L4g — CLI release: every target built, npm packages and the Homebrew formula published

| Field | Value |
| --- | --- |
| ID | `L4g` |
| Status | `done` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L4d](./L4d-cli-distribution.md), [X6](./X6-windows-ci.md) |
| Dependency notes | L4d built three of dist's eight targets and left publishing to the owner; X6 gave CI Windows runners. The repository is public, so the formula this todo publishes, which points at the GitHub release, works for everyone; [L4f](../ongoing/L4f-command-line-docs.md) then adds a download table to the docs site. |
| ADRs | Follows [0017](../../decisions/0017-versioning-and-release.md) §7 (npm trusted publishing from a GitHub-hosted runner, no stored registry token), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (checksums, SBOM, notices), [0031](../../decisions/0031-cli-and-mcp-contract.md) (the binary and its packages) |
| Branch | `ci/cli-release-targets-and-publishing` |
| PR title | `ci(cli): build every release target and publish the npm packages and the Homebrew formula` |
| Size | M (≈ 300–500 lines: the workflow's build matrix and two publish jobs, Windows in the test scripts, the workflow tests, the owner's setup) |
| PR | [#395](https://github.com/laudebugs/laminapdf/pull/395) |

## Goal

Make a `laminapdf-cli-v*` tag release the CLI everywhere it says it runs. CI builds and installs all eight targets, not three. The same tag then publishes the nine npm packages through npm's trusted publishing and pushes the Homebrew formula to `laudebugs/homebrew-tap`, so no step is left to the owner after the first release.

## Context

- [L4d](../done/L4d-cli-distribution.md): `cli-release.yml`, dist's config, `tools/cli-release/` and `packages/cli`; its notes list what it left out.
- `.github/workflows/release.yml`'s `publish-npm` job: npm trusted publishing from `ubuntu-latest` with npm pinned, no checkout, tarballs from an artifact.
- `release.yml` and `release-train.yml`: a short-lived token from the release bot (`actions/create-github-app-token`) for writes to another repository.

## Scope

**In scope**

- Builds on GitHub-hosted runners for the five missing targets: `x86_64-apple-darwin` (`macos-15-intel`), `x86_64-unknown-linux-gnu` and `-musl` (`ubuntu-24.04`), `x86_64-pc-windows-msvc` (`windows-2025`) and `aarch64-pc-windows-msvc` (`windows-11-arm`). Each archive is extracted and run, and its npm platform package installed and run where npm can install it on that runner.
- Windows in `archive-test.sh` and `install-test.sh`: `.zip` archives and `laminapdf.exe`.
- `global` and `publish` need every build: a release is all eight targets or nothing.
- `publish-npm`: the wrapper and the eight platform packages, platform packages first, with npm's trusted publishing in the `release` environment; a version already on npm is skipped, so a re-run finishes a partial publish.
- `publish-homebrew`: dist's formula pushed to `laudebugs/homebrew-tap` (`Formula/laminapdf.rb`) with a release-bot token scoped to that repository.
- The owner's one-time setup, written down: the tap repository, the release bot installed on it, each npm package's first publish and its trusted publisher.

**Out of scope**

- A download table on the docs site: [L4f](../ongoing/L4f-command-line-docs.md).
- Build-provenance attestations: the `publish` job already makes them whenever the repository is public, as it now is.
- Code signing and notarisation: [D6c](../ready/D6c-macos-signing-and-notarisation.md), [D6d](../ready/D6d-windows-signing.md).
- crates.io: `laminapdf-cli` stays `publish = false` (ADR 0031).

## Tasks

- [x] Claim in the work PR (its first commit adds this file in `ongoing/`).
- [x] Build matrix for the eight targets; Windows support in the test scripts.
- [x] `global` and `publish` wait for every build.
- [x] `publish-npm` and `publish-homebrew`, behind the same switches as `publish`.
- [x] Update `tools/xtask/src/workflows.rs`' tests of the workflow.
- [x] The owner's setup in the CLI's README and `tools/release/README.md`.
- [x] A `workflow_dispatch` dry run on the PR branch: every build passes.

## Acceptance criteria

- [x] A dry run builds all eight targets, and each archive installs and runs `laminapdf --version` and `laminapdf info` on a fixture.
- [x] The npm wrapper with its platform package installs and runs on macOS (both CPUs), Linux glibc (both CPUs) and Windows (both CPUs).
- [x] The publish jobs run only on a `laminapdf-cli-v*` tag push with `LAMINA_PDF_PUBLISH_CLI` set to `true`, in the `release` environment, and only after every build passed; `tools/xtask/src/workflows.rs` tests the condition.
- [x] No long-lived token is stored: npm uses trusted publishing, and the tap push uses a token minted for that job.

## Post-merge checks

- [ ] After the owner's setup, the first `laminapdf-cli-v*` tag publishes the nine npm packages and the formula: `npm install -g @laminapdf/cli` and `brew install laudebugs/tap/laminapdf` work anywhere.

## Tests

`tools/xtask/src/workflows.rs` (the publish gate, every build needed, no stored secret but the release bot's); the dry-run dispatch on the PR.

## Docs to update

`crates/laminapdf-cli/README.md` (install), `tools/release/README.md` (the owner's setup), `dist-workspace.toml` and `cli-release.yml`'s headers, CLAUDE.md's CLI release paragraph, [shareable libraries §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1).

## Risks and open questions

- **Runner labels:** `macos-15-intel` and `windows-11-arm` are GitHub's current labels for those machines; a renamed label fails the build, not the release.
- **npm's first publish:** npm sets a trusted publisher only on a package that exists, so the owner publishes each package once from the dry run's tarballs. Until then, `publish-npm` fails and says which package is missing.

## Definition of done

- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- Dry run [42894611917](https://github.com/laudebugs/laminapdf/actions/runs/42894611917) on the PR: all eight builds passed (archive, npm install where npm installs, static musl, Homebrew on both Macs), and `global` packed nine npm packages with `publish.json` (tag `alpha`).
- Windows: dist's zips are flat (no top folder), and Git Bash's GNU tar comes before Windows' own on PATH, so `generate.ts` calls `System32\tar.exe` by full path and both test scripts find the folder holding the binary.
- Intel Mac: pnpm/action-setup can't verify its darwin-x64 binary, so the build installs the pinned pnpm with npm; cargo-about 0.9.2 has no x86_64-apple-darwin binary and install-action's source fallback drops its `cli` feature, so that runner runs `cargo install --features cli`.
- L4f was rescoped in this PR: the repository is public, so the GitHub release serves the archives, and L4f keeps only the docs site's download table.
