# X12 — Run each browser suite only when the dependency graph reaches it

| Field | Value |
| --- | --- |
| ID | `X12` |
| Status | `done` |
| Plan | [CI caching §8.3](../../plans/ci-caching.md#83-browser-suites-by-the-dependency-graph-x12) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X10a](./X10a-desktop-inputs-from-the-graph.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x12-suites-by-graph` |
| PR title | `ci: run each browser suite only when the dependency graph reaches it` |
| Size | L |
| PR | [#664](https://github.com/laudebugs/laminapdf/pull/664) |

## Goal

The fast lane's runtime legs run every browser suite on almost every code PR, so a change to the CLI's code or the website's copy still runs the SDK's examples, nine packages' browser tests and three apps' e2e suites in three browsers. On `main`'s run 43751465593 those legs took 16, 12 and 40 minutes, WebKit on macOS. This todo works out each suite's inputs from the graph X10a walks for the desktop app, and runs only the suites a pull request's paths reach. The reader's deploy flag moves to the same graph.

## Context

- The plan's §8.3: the suites, what each reads, and the expected savings.
- `tools/xtask/src/changes/desktop.rs` (X10a): the crate and package walk this todo shares.
- `tools/xtask/src/pdf_runtimes.rs`: `render_suites`, what each browser leg runs, and the rule that a runtime printing no measurements fails.
- `ci.yml`'s `changes`, `pdf-runtimes` and `fast-gate`; `web-deploy.yml`'s `preview-build`; `tools/xtask/src/workflows.rs`, which tests each job's `if` against its gate.

## Scope

**In scope**

- `changes/graph.rs`: the crate and package graphs, read once, with `desktop.rs` over it.
- `changes/suites.rs`: the plan's suites, each with its root, its extra inputs and the inputs every suite shares; `runtime_legs`, the matrix with each leg's suites.
- `cargo xtask changes` prints `runtime-legs`. Pushes to `main` run every leg and suite.
- `cargo xtask pdf-runtimes --suites <list>`, with no list running every suite. A browser leg without `examples` records no measurements and doesn't fail for it.
- `ci.yml`'s runtime matrix from `runtime-legs`; `fast-gate` expects the job when the list isn't empty.
- `touches_web` from the `web` suite's inputs, the deploy workflow and the manifests, in place of `OUTSIDE_WEB`, for the reader's deploys and for its daily release (`web_release.rs`' `ships`, which called the same function).
- A test that `docs.yml`'s and `site.yml`'s path filters cover their suites' graph inputs; `site.yml` gains `packages/icons/**`.

**Out of scope**

- `wasm`, `typescript` and `semver` stay on `ts` (about 12 minutes together, on Linux).

## Tasks

- [x] `changes/graph.rs`, with its tests moved from `desktop.rs`; `desktop.rs` over it.
- [x] `changes/suites.rs`: `SUITES`, `inputs`, `reached`, `runtime_legs`, with tests on the repository's graph.
- [x] `changes.rs`: the `runtime-legs` output and `touches_web` from the graph.
- [x] `pdf_runtimes.rs`: `--suites`, the measurement rule, `help` text.
- [x] `ci.yml`: the push step's full matrix, the `pdf-runtimes` matrix and `if`, `fast-gate`; `workflows.rs`' tests.
- [x] `site.yml`'s path filter, and the filter test.
- [x] The plan's §8.1 table, CLAUDE.md's `changes` and `pdf-runtimes` entries.

## Acceptance criteria

- [x] A change only to `crates/laminapdf-cli/src/` gives an empty `runtime-legs` and `web=false`, and doesn't release the reader.
- [x] A change only to `apps/site/src/data/` runs the `site` suite alone, in each browser leg, and no server leg.
- [x] A change to `crates/laminapdf-cos/` runs `examples`, `render-hayro`, `viewer`, `web` and `docs`, and the server leg.
- [x] A change to `docs/errors/` with a code change runs `docs`.
- [x] A push to `main` runs every leg and every suite; the nightly's and the release lane's `pdf-runtimes` runs are unchanged.

## Post-merge checks

- The next PRs that change only the CLI or the website skip the legs the plan says they skip, and `fast-gate` passes.

## Tests

`tools/xtask/src/changes/graph.rs`, `suites.rs`, `desktop.rs`; `suites.rs`' `touches_web` tests; `web_release.rs`' `ships` test; `pdf_runtimes.rs`' argument tests; `workflows.rs`.

## Docs to update

The plan's §8.1 table and §8.3's checkbox; CLAUDE.md's `cargo xtask changes` and `pdf-runtimes` entries.

## Risks and open questions

- A file a suite reads that no graph or list names skips that suite on a pull request, until the next push to `main` runs it. Every suite counts `fixtures/`, the manifests, `tools/xtask/`, `ci.yml` and its actions, and when unsure, a path counts.
- A change to `tools/xtask/` runs every suite, as the desktop's inputs do: most xtask modules feed some leg.
