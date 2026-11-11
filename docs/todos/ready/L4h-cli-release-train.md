# L4h — CLI release train: tag `laminapdf-cli-v<version>` after each daily release

| Field | Value |
| --- | --- |
| ID | `L4h` |
| Status | `ready` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L4g](../done/L4g-cli-release-targets-and-publishing.md), [C7e](../done/C7e-automated-web-releases.md), [T4j](../ongoing/T4j-documents-in-typescript.md) |
| Dependency notes | L4g's tag push builds and publishes the CLI; C7e's web train is the pattern to follow. T4j is a queue position, not a code dependency: the owner asked for this after the CLI's T4 todos. The train's first tag needs the owner's L4g setup, done 2026-11-10 except the npm packages' first publish and trusted publishers. |
| ADRs | Follows [0017](../../decisions/0017-versioning-and-release.md) (release gating, the release bot), [0031](../../decisions/0031-cli-and-mcp-contract.md) |
| Branch | `ci/cli-release-train` |
| PR title | `ci(cli): tag a CLI release after each daily release that changed it` |
| Size | M (≈ 200–350 lines: the workflow, the xtask decision and its tests, docs) |
| PR | — |

## Goal

Release the `laminapdf` CLI without the owner pushing a tag. After each daily library release succeeds, the release bot tags `laminapdf-cli-v<version>` on the release's commit when the CLI changed since its last release. The tag starts `cli-release.yml`, which builds all eight targets and publishes the GitHub release, npm and the Homebrew formula.

## Context

- The CLI shares the workspace's lockstep version (ADR 0017), and only the daily release bumps it. A new CLI release therefore follows a daily release, at that release's version, on its commit.
- [C7e](../done/C7e-automated-web-releases.md): `web-release-train.yml` and `cargo xtask web next-release` (`tools/xtask/src/web_release.rs`) decide whether the reader changed and tag with the release bot. A tag the bot pushes starts workflows; one pushed with `GITHUB_TOKEN` would not.
- `release.yml` (the daily release lane, from a `v<version>` tag) and `release-train.yml`; `tools/release/README.md`.
- `cli-release.yml` and its publish gate (`LAMINA_PDF_PUBLISH_CLI`), from L4g.

## Scope

**In scope**

- `cli-release-train.yml`: on `workflow_run` of the release lane, when it completed successfully for a `v<version>` tag push, check out that tag's commit (never a pull request's code). If `laminapdf-cli-v<version>` doesn't exist and the CLI changed, the release bot tags that commit and pushes the tag.
- `cargo xtask cli next-release`: decides, like `web next-release`. "Changed" means files the CLI binary or its packages ship changed since the newest `laminapdf-cli-v*` tag: the crate, the workspace crates it depends on (from `cargo metadata`), `dist-workspace.toml`, `packages/cli` and `tools/cli-release`. With no CLI tag yet, it releases.
- A `workflow_dispatch` path that runs the decision for a given release tag and prints it, tagging nothing unless asked.
- A kill switch: the repository variable `CLI_RELEASE_TRAIN=off` stops future trains, as `WEB_RELEASE_TRAIN` does.
- Tests in `tools/xtask/src/workflows.rs`: the trigger, the checkout of the tag's commit, the release bot's token, the kill switch.

**Out of scope**

- Changing how `cli-release.yml` builds or publishes (L4g).
- A CLI version apart from the workspace's.

## Tasks

- [ ] `cargo xtask cli next-release`, with tests on a temporary repository: no tag yet, unchanged, changed, tag already present.
- [ ] `cli-release-train.yml`, its dispatch path and the kill switch.
- [ ] `workflows.rs` tests.
- [ ] Docs: `tools/release/README.md` ("The CLI's release": the train replaces the manual tag), `crates/laminapdf-cli/README.md`, CLAUDE.md.

## Acceptance criteria

- [ ] A dispatch for an existing `v<version>` tag prints the decision and tags nothing.
- [ ] After a daily release whose commit changed the CLI, `laminapdf-cli-v<version>` appears on that commit and `cli-release.yml` runs on it.
- [ ] After a daily release that didn't change the CLI, no tag is pushed.
- [ ] `CLI_RELEASE_TRAIN=off` stops the train.

## Tests

`cargo xtask cli next-release`'s unit tests; `tools/xtask/src/workflows.rs`; the dispatch dry run on the PR.

## Docs to update

`tools/release/README.md`, `crates/laminapdf-cli/README.md`, CLAUDE.md's CLI release paragraph, [shareable libraries §9 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp-1).

## Risks and open questions

- **Workflow installation:** if the agent's GitHub access can't push a new workflow file, the owner copies it from `tools/release/`, as with the other trains.
- **Publishing gate:** while `LAMINA_PDF_PUBLISH_CLI` isn't `true`, a train's tag builds the CLI but publishes nothing. The tag then exists, so the next train skips that version.

## Definition of done

- [ ] Claimed: the work PR's first commit moves this file `ready/` → `ongoing/` (Status `ongoing`, links updated)
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
