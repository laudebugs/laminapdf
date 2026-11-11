# L4h — CLI release train: tag `laminapdf-cli-v<version>` after each daily release

| Field | Value |
| --- | --- |
| ID | `L4h` |
| Status | `ongoing` |
| Plan | [shareable libraries §7 L4](../../plans/shareable-libraries.md#l4-cli-and-mcp) |
| Round / milestone | Round L4 (shareable libraries) |
| Depends on | [L4g](../done/L4g-cli-release-targets-and-publishing.md), [C7e](../done/C7e-automated-web-releases.md) |
| Dependency notes | L4g's tag push builds and publishes the CLI; C7e's web train is the pattern to follow. The owner's L4g setup is done: the first release, `laminapdf-cli-v0.2.0-alpha.5`, published on 2026-11-11. |
| ADRs | Follows [0017](../../decisions/0017-versioning-and-release.md) (release gating, the release bot), [0031](../../decisions/0031-cli-and-mcp-contract.md) |
| Branch | `ci/cli-release-train` |
| PR title | `ci(cli): tag a CLI release after each daily release that changed it` |
| Size | S (≈ 150–250 lines: the workflow, its tests, docs) |
| PR | — |

## Goal

Release the `laminapdf` CLI without the owner pushing a tag. After each daily library release succeeds, the release bot tags `laminapdf-cli-v<version>` on the release's commit.

**Decided 2026-11-11 (owner): every release, not only when the CLI changed.** The docs site's download table (L4f) links the release named by the docs' version, so a skipped version would leave its links answering 404. Tagging every release keeps the CLI and the libraries in lockstep and needs no change detection; the cost is one CLI build a day and unchanged binaries republished. This replaces the `cargo xtask cli next-release` decision below. The owner also asked for this before T4j, so the T4j dependency (a queue position) is dropped. The tag starts `cli-release.yml`, which builds all eight targets and publishes the GitHub release, npm and the Homebrew formula.

## Context

- The CLI shares the workspace's lockstep version (ADR 0017), and only the daily release bumps it. A new CLI release therefore follows a daily release, at that release's version, on its commit.
- [C7e](../done/C7e-automated-web-releases.md): `web-release-train.yml` and `cargo xtask web next-release` (`tools/xtask/src/web_release.rs`) decide whether the reader changed and tag with the release bot. A tag the bot pushes starts workflows; one pushed with `GITHUB_TOKEN` would not.
- `release.yml` (the daily release lane, from a `v<version>` tag) and `release-train.yml`; `tools/release/README.md`.
- `cli-release.yml` and its publish gate (`LAMINA_PDF_PUBLISH_CLI`), from L4g.

## Scope

**In scope**

- `cli-release-train.yml` (reviewed in `tools/release/`, installed in `.github/workflows/`): on `workflow_run` of the release lane (`Release`), when a `v<version>` tag push completed successfully, check the run with the API (the tag still names the run's commit), and have the release bot create `laminapdf-cli-v<version>` on that commit. Nothing is checked out or run from the repository.
- A tag that already exists is reported and left alone, so a re-run or a hand-pushed tag is harmless.
- A `workflow_dispatch` path for a given release run: it prints what it would tag and tags only when asked.
- A kill switch: the repository variable `CLI_RELEASE_TRAIN=off` stops future trains, as `WEB_RELEASE_TRAIN` does.
- Tests in `tools/xtask/src/workflows.rs`: the trigger, the run's checks, the release bot's token, the kill switch, the installed copy matching the reviewed one.

**Out of scope**

- Changing how `cli-release.yml` builds or publishes (L4g).
- A CLI version apart from the workspace's.

## Tasks

- [ ] `cli-release-train.yml`, its dispatch path and the kill switch.
- [ ] `workflows.rs` tests.
- [ ] Docs: `tools/release/README.md` ("The CLI's release": the train replaces the manual tag), `crates/laminapdf-cli/README.md`, CLAUDE.md.

## Acceptance criteria

- [ ] A dispatch for an existing `v<version>` tag prints the decision and tags nothing.
- [ ] After a daily release whose commit changed the CLI, `laminapdf-cli-v<version>` appears on that commit and `cli-release.yml` runs on it.
- [ ] A second run for the same release reports the existing tag and pushes nothing.
- [ ] `CLI_RELEASE_TRAIN=off` stops the train.

## Tests

`tools/xtask/src/workflows.rs`; a dispatch dry run once the workflow is on `main`.

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
