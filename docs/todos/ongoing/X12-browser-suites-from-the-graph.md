# X12 — Run each browser suite only when the dependency graph reaches it

| Field | Value |
| --- | --- |
| ID | `X12` |
| Status | `ongoing` |
| Plan | [CI caching §8.3](../../plans/ci-caching.md#83-browser-suites-by-the-dependency-graph-x12) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X10a](../done/X10a-desktop-inputs-from-the-graph.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x12-suites-by-graph` |
| PR title | `ci: run each browser suite only when the dependency graph reaches it` |
| Size | L |
| PR | — |

## Goal

The fast lane's runtime legs run every browser suite on almost every code PR. Each of Chromium, Firefox and WebKit runs the SDK's examples, every package's browser tests, the reader's e2e suite, the docs site's and the website's. On main's run 43751465593 they took 16, 12 and 40 minutes, WebKit on a macOS runner. A change to the CLI, a docs page's text or the website's copy reaches none of those suites, or only one. This todo works out which suites a PR's paths can reach from the same graph X10a walks for the desktop app, and runs only those. The reader's deploy flag (`web`) moves to the same graph.

## Context

- `tools/xtask/src/changes/desktop.rs` (X10a): the crates reached through path dependencies (`cargo metadata --no-deps`) and the packages reached through `workspace:` dependencies.
- `tools/xtask/src/pdf_runtimes.rs`: `render_suites` lists what each browser runs.
- `ci.yml`'s `pdf-runtimes` matrix and `fast-gate`; `web-deploy.yml`'s `preview-build`.

## Scope

**In scope**

- `changes/graph.rs`: the crate and package graphs, shared by the desktop's inputs and the suites'.
- A suite per package with browser tests, plus the SDK's examples, the reader, the docs site and the website, each with its root package, its extra inputs and the inputs every suite shares.
- `cargo xtask changes` prints `runtime-legs`, the runtime matrix with each leg's suites; pushes to `main` run every leg and suite.
- `cargo xtask pdf-runtimes --suites <list>`; the default runs every suite, as the nightly and release lanes do.
- `web` from the reader's suite inputs in place of `OUTSIDE_WEB`.

**Out of scope** (and which todo covers it)

- The `wasm`, `typescript` and `semver` jobs stay on `ts`: together they take about 12 minutes, on Linux.

## Tasks

- [ ] `changes/graph.rs`, with `desktop.rs` over it.
- [ ] `changes/suites.rs`: the suites, their inputs, `runtime_legs`.
- [ ] `pdf_runtimes.rs`: `--suites`.
- [ ] `ci.yml`: the matrix from `runtime-legs`, and `fast-gate`'s rule; `workflows.rs`' tests.
- [ ] `touches_web` from the reader's inputs.
- [ ] Plan §8.1 and §8.3, CLAUDE.md.

## Acceptance criteria

- [ ] A change only to `crates/laminapdf-cli/` runs no runtime leg and sets `web=false`.
- [ ] A change only to `apps/site/` runs only the website's suite, in each browser, and no server leg.
- [ ] A change to `crates/laminapdf-cos/` runs every suite that reaches the WebAssembly.
- [ ] A push to `main` runs every leg and every suite.

## Post-merge checks

- The first PRs after the merge skip the legs this todo says they skip.

## Tests

`tools/xtask/src/changes/suites.rs`, `graph.rs`, `pdf_runtimes.rs`, `workflows.rs`.

## Docs to update

The CI caching plan's §8.1 table and a new §8.3; CLAUDE.md's `cargo xtask changes` and `pdf-runtimes` entries.

## Risks and open questions

- A file a suite reads that no graph or list names skips that suite on a PR, until the next push to `main` runs it. Every suite counts the fixtures, the manifests, `tools/xtask` and the workflow, and when unsure, a path counts.
