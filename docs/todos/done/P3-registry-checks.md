# P3 — Registry checks after the first release: installs, semver, docs

| Field | Value |
| --- | --- |
| ID | `P3` |
| Status | `done` |
| Plan | [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [pdf-library §6](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Publishing |
| Depends on | [P2](./P2-first-registry-prerelease.md) |
| Blocks | [P4](../ready/P4-docs-match-the-release.md) |
| ADRs | Follows [0017](../../decisions/0017-versioning-and-release.md) §6–§7, [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `build/p3-registry-checks` |
| PR title | `build: check registry installs and semver against the published version` |
| Size | M (≈ 300–500 lines: xtask, CI jobs, docs pages) |
| PR | [#258](https://github.com/laudebugs/laminapdf/pull/258) |

## Goal

Once `0.2.0-alpha.1` is on crates.io and npm ([P2](../done/P2-first-registry-prerelease.md)), check the published packages the way users meet them: install them from the registries in every runtime. Semver checks on every PR, against the published version, then keep later releases honest about breaking changes. The docs site switches from checkout installs to registry installs.

## Context

- [P2](../done/P2-first-registry-prerelease.md)'s out-of-scope list, which this todo was split from on 2026-11-01.
- [pdf-library §8 risks](../../plans/pdf-library.md#8-risks): cargo-semver-checks from 0.2.
- `cargo xtask verify-install` (B7b) installs the packed tarballs and path or git dependencies today.

## Scope

**In scope**

- `cargo xtask verify-install --from-registry`: the quick start from crates.io and npm (the `alpha` tag; the real registries, since `0.2.0-alpha.1` is published first, so the stand-in registries the original P2 planned, Verdaccio and a local Cargo registry, are no longer needed to prove the harness) in Node 22/24/26, Deno, Bun, Chromium (a Vite app), workerd and a Rust binary, byte-identical to `fixtures/golden/examples/hello.pdf`; a release-lane job after the publish jobs.
- `cargo-semver-checks` in the fast lane against the latest published version of each public crate (pinned in `doctor.rs`), and a TypeScript API report (`api-extractor` or similar) diffed against the published `@laminapdf/sdk`.
- The docs site: install pages, quick starts and Your first PDF install from the registries; the stability policy as a page; the changelog linked.
- `README.md`: the status line and registry badges; docs.rs builds checked.

**Out of scope**

- npm provenance and attestations: once the repository is public (ADR 0017 §7).

## Tasks

- [x] `verify-install --from-registry` and its release-lane job
- [x] The cargo-semver-checks job and the TypeScript API diff
- [x] The docs site's install pages, quick starts and stability page; README
- [x] Tick architecture's "Publish the first pre-release" and "Publish the stability policy… cargo-semver-checks", and pdf-library §10 B7's "Later, when the owner decides: publish…"

## Acceptance criteria

- [x] `cargo xtask verify-install --from-registry` passes against `0.2.0-alpha.1` in every runtime (`0.2.0-alpha.2`, the first version published: Node 22, Deno, Bun, workerd and Rust, run on Linux; the release lane's `verify-registry` jobs run Node 22/24/26 and Chromium too)
- [x] cargo-semver-checks runs on PRs and fails a deliberate breaking change in a test branch; the TS API diff job fails a deliberate change the same way (the pending changesets' breaking changes since `0.2.0-alpha.2` stood in for the test branch: both checks report them, and `cargo xtask semver` fails once the minor changesets are removed)
- [x] The docs site's build passes, and no page tells readers to build from a checkout to install

## Notes from the work

- **Baselines:** both registries hold a `0.0.0` placeholder that cargo-semver-checks would pick, so `cargo xtask semver` passes `--baseline-version` (npm's newest non-placeholder version by default). `laminapdf-extract` isn't on crates.io yet, so it is listed as unchecked until a release publishes it.
- **Pass or fail:** a strict check (`--release-type minor`) would fail every PR after a breaking one until the release. Instead the job fails only when the changesets' bump is below minor, so the release's version stays honest; whether each PR declares its own break stays a review matter. Between a version PR and npm approving that version, the version PR has consumed the changesets (moved to `.changeset/pre/` in pre mode), so the prepared version's step and those changesets count too.
- **API reports:** each line is keyed by the declarations it sits in, cut after their names, so a member removed from one of several classes is found and a class that gains `extends` keeps its members. The `node` and `workerd` entries of `.` get their own reports.
- **API Extractor** looks for `package.json` from its config's folder and treats packages under `node_modules` as external, so the config goes inside the analysed package and the published `@laminapdf/sdk` is copied out of `node_modules` first. A string-literal union (such as `ErrorCode`) is split into one line per member, so a new code isn't reported as a break.
- **The stability policy** moved to the docs site (`reference/stability`), its one home; `docs/stability-policy.md` keeps how it's checked. The changelog page renders `CHANGELOG.md` through a content loader, without its commit hashes.
- **CI:** the `semver` job (fast lane, in `fast-gate`) and the release lane's `registry-ready` and `verify-registry` jobs are workflow changes; the PR description carries them for the owner to commit.

## Post-merge checks

- [ ] docs.rs shows every public crate's documentation
- [ ] The docs site's production deploy shows the registry installs

## Tests

`verify-install --from-registry` in each runtime; the semver jobs' deliberate-break checks.

## Docs to update

- [x] [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), `README.md`, the docs site, `CLAUDE.md`

## Risks and open questions

- The registry installs depend on the owner's npm approvals of P2's versions.
- cargo-semver-checks needs a published baseline for every crate; a crate added later starts with none.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
