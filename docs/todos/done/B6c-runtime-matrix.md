# B6c — Runtime matrix in CI: browsers, Node 22/24/26, Deno, Bun and workerd

| Field | Value |
| --- | --- |
| ID | `B6c` |
| Status | `done` |
| Plan | [pdf-library §7 B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf), [§5 platform matrix](../../plans/pdf-library.md#platform-matrix) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B6b](./B6b-lamina-pdf-package.md) |
| Blocks | [B7a](../ready/B7a-docs-site.md), [B7b](../ready/B7b-versioning-and-0-1-0-alpha.md), [B9c](../ongoing/B9c-text-extraction-bindings-and-differential.md) |
| ADRs | Implements 0016's runtime support claims; [0018](../../decisions/0018-test-strategy.md) §2 (lanes: which runtimes run per PR vs nightly/release) |
| Branch | `ci/b6c-runtime-matrix` |
| PR title | `ci(pdf): run the @lamina/pdf example suite in browsers, Node, Deno, Bun and workerd` |
| Size | M (≈ 400–700 lines: test configs, workflow, a Worker fixture) |
| PR | [#88](https://github.com/laudebugs/lamina/pull/88) |

## Goal

Prove the package works where the plan says it does. The same TS example suite runs in Chromium, Firefox and WebKit (Vitest browser mode with Playwright), Node 22/24/26, Deno, Bun, and Cloudflare's workerd (`@cloudflare/vitest-pool-workers`), producing byte-identical output everywhere, with the measured hello-world and 100-page timings and Worker memory recorded against the §4 targets.

## Context

- [pdf-library §5 platform matrix](../../plans/pdf-library.md#platform-matrix) and Node LTS note; [§4 non-functional](../../plans/pdf-library.md#non-functional) (≤ 5 ms hello after `init()`, 100-page report ≤ 500 ms, < 64 MB peak in a Worker).
- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes): fast vs full vs release lanes.
- [Repo and packaging notes §6](../../research/architecture/notes/repo-and-packaging.md#6-ci-on-github-actions-target-matrix-caching-playwright-tauri-action-testing-strategy).

## Scope

**In scope**

- Vitest workspace configs in `packages/pdf/`: `vitest.node.config.ts`, `vitest.browser.config.ts` (Playwright provider: chromium, firefox, webkit), `vitest.workers.config.ts` (`@cloudflare/vitest-pool-workers`, `wrangler.toml` test fixture importing the `.wasm` as a module).
- Deno and Bun smoke runners (`deno test` / `bun test`) over a shared `examples` entry using the `deno`/`default` conditions.
- Workflow jobs: Node matrix (22, 24, 26) + Chromium in the fast lane; Firefox, WebKit, Deno, Bun, workerd in the full lane ([X4](../done/X4-full-lane-gate.md)) and the release lane.
- A Worker example (`packages/pdf/examples/worker/`) that generates a PDF per request — also used by docs ([B7a](../ready/B7a-docs-site.md)).
- Performance smoke (not asserted in PR lanes; recorded): hello and 100-page report timings per runtime written to the job summary; the nightly trend is a job in `.github/workflows/nightly.yml` added here, recording the timings in the benchmark history (`history.jsonl` on `bench-data`, the record format of `cargo xtask bench` from [X3a](../done/X3a-nightly-lane.md)).

**Out of scope** (and which todo covers it)

- Clean-install-from-tarball tests: [B7b](../ready/B7b-versioning-and-0-1-0-alpha.md).
- Bundler matrix (Vite, webpack, esbuild) smoke builds: included here only as a single Vite build check; a fuller matrix is a follow-up if failures appear.

## Tasks

- [x] Add dev deps (`@vitest/browser`, `playwright`, `@cloudflare/vitest-pool-workers`, `wrangler`) and check npm licences (deviation: `@vitest/browser-playwright`, `playwright`, `wrangler`, `deno`, `bun`; no pool-workers, see Outcome)
- [x] Configs and shared test entry; ensure tests read goldens without Node APIs in browser/worker (fetch fixtures via Vite's asset handling)
- [x] Deno/Bun runners; Vite build smoke
- [x] Workflow jobs (fast vs full lanes) with Playwright browser cache
- [x] Worker example + memory measurement (workerd reports) for the 100-page report with two fonts
- [x] Document how to run each runtime locally in `packages/pdf/README.md` and `CLAUDE.md`

## Acceptance criteria

- [x] The example suite passes in Chromium, Firefox, WebKit, Node 22/24/26, Deno, Bun and workerd with byte-identical outputs
- [x] A Vite production build of a tiny app importing `@lamina/pdf` works in Chromium
- [x] Timings and Worker peak memory are reported per runtime; any miss of the §4 targets is listed in the PR and filed as an issue (not a CI failure) (the report misses are [B5d](../done/B5d-text-report-generation-performance.md)'s, see Outcome)

## Post-merge checks

None.

## Tests

As above; all under `packages/pdf/test/` sharing one example list.

## Docs to update

- [x] [pdf-library §10 B6](../../plans/pdf-library.md#10-to-dos): tick "Run the TS example suite in …"
- [x] `CLAUDE.md` commands, package README

## Risks and open questions

- WebKit on Linux CI can be flaky; per ADR 0018 §4 a flaky test is a bug: mark with an issue, don't retry.
- workerd's `.wasm` import rules differ from browsers; the `workerd` condition must point to a module that imports the compiled module, verified by attw/publint where possible.

## Outcome

- Delivered: one example list, `packages/pdf/test/suite/examples.ts` (the six Rust twins against their goldens, and `examples/report.ts`: 100 A4 pages with two embedded fonts, Noto Sans and Noto Sans SC, checked against `REPORT_SHA256` since it has no Rust golden), run by a runner per runtime: `vitest.node.config.ts` (Node; the old `vitest.config.ts`), `vitest.browser.config.ts` (Vitest browser mode with `@vitest/browser-playwright`: one project per browser, fixtures fetched as Vite `?url` assets, plus a `vite-build` project that serves a Vite production build of `test/vite-app/` with `vite preview` and runs it in a Chromium page), `test/deno/` (`deno test`, the `deno` condition), `test/bun/` (`bun test`, the `node` condition) and `vitest.workers.config.ts` (workerd, see below). Every runner prints `LAMINA_MEASUREMENT {json}` lines; `cargo xtask pdf-runtimes [runtime…] [--history <dir> --runner <label> --record]` runs the runners, collects the lines into `target/xtask/pdf-runtimes/measurements.json` and a job-summary table against §4 (misses marked, never a failure), and appends `pdf/<runtime>/<case>` records to `history.jsonl`.
- Worker example: `packages/pdf/examples/worker/` (`/?name=` a letter in Helvetica, a 400 with the `LaminaError` as JSON for a name it can't draw: `missing-glyph` or `encoding-full`; every builder and document freed per request; `/report` the report; fonts bundled as `Data` modules), also served in the workerd run, which checks the report's hash and the 400.
- Measurements: `measure()` runs first in a fresh module instance in every runtime (Node: its own file, the `measure` project of `vitest.node.config.ts`, kept out of `cargo xtask wasm`; browsers, Deno, Bun: the first test of the file; workerd: the first request to a fresh Worker, with `scheduler.wait(0)` around each clock reading because workerd's clock only advances across I/O), so the first hello is cold and the memory is the hello and report runs' alone.
- CI: fast lane `pdf-runtimes` (Node 22, 26 and Chromium, over the `wasm` job's build uploaded as an artifact; Node 24, `.nvmrc`, is the `wasm` job's own run of the suite); full lane `pdf-build` (`cargo xtask wasm` once) and `pdf-runtimes` (Firefox, WebKit, Deno, Bun, workerd), both in `gate`'s `needs`; nightly `pdf-runtimes` (every runtime, after `bench`, recording on `main` and pushing `bench-data`; the release lane gets it through `workflow_call`); both nightly jobs fetch and push the history with `.github/actions/bench-history` (a refused push is re-appended to the branch as it is now and retried). Playwright browsers live in each runner's tool cache (`PLAYWRIGHT_BROWSERS_PATH`); Deno, Bun and wrangler are dev dependencies (pinned in the pnpm catalog), so CI and local runs use the same versions with no system install. `cargo xtask nightly` runs `wasm` and `pdf-runtimes` too.
- Measured locally (M-series Mac shared with other work; one run each; hello is the median of 20 after a first call, the report the median of 3; WebAssembly memory is both modules' linear memory after the report, its peak since it never shrinks):

  | Runtime | First hello | Hello | 100-page report | WebAssembly memory |
  | --- | --- | --- | --- | --- |
  | Node 22 | 0.7 ms | 0.4 ms | 622 ms | 5.4 MB |
  | Node 24 | 0.8 ms | 0.9 ms | 623 ms (462 ms run alone) | 5.4 MB |
  | Node 26 | 1.2 ms | 0.6 ms | 572 ms | 5.4 MB |
  | Chromium | 0.7 ms | 0.4 ms | 298 ms | 5.4 MB |
  | Firefox | 1.0 ms | 1.0 ms | 520 ms (712 ms in another run) | 5.4 MB |
  | WebKit | 2.0 ms | 1.0 ms | 270 ms | 5.4 MB |
  | Deno | 1.2 ms | 0.4 ms | 526 ms | 5.2 MB |
  | Bun | 1.7 ms | 1.4 ms | 442 ms | 5.1 MB |
  | workerd | 34 ms | 1.0 ms | 562 ms | 4.7 MB |

  Hello (≤ 5 ms) and Worker memory (< 64 MB) are met everywhere; the report (≤ 500 ms) is missed in Node, Firefox, Deno and workerd (520–710 ms over several runs), which is [B5d](../done/B5d-text-report-generation-performance.md)'s target. workerd's first hello includes its lazy compilation of the module. The Node figures come from the whole Node suite running in parallel Vitest workers.
- Deviations:
  - workerd runs through wrangler's `createTestHarness` from a Vitest test in Node (`vitest.workers.config.ts`), not `@cloudflare/vitest-pool-workers`: its latest release (0.23.0) supports Vitest 4 only and the repo is on Vitest 5. The examples still run inside workerd (test/workerd/worker.ts answers with each example's bytes and the measurements), bundled by wrangler through the `workerd` export condition; Node compares the bytes with the goldens.
  - Worker memory is the WebAssembly linear memory (`wasmMemory()`, measured inside the Worker): workerd reports no per-isolate figure locally, and the JS heap is not included.
  - The nightly job records the timings but doesn't compare them: millisecond timings on a shared runner vary by more than the 10 % the criterion benchmarks use, so they would open a regression issue most nights.
  - Playwright's Firefox (155 and later) reads the user's Firefox data folder (`~/Library/Application Support/Firefox`) even with its own profile; where it can't read that folder (the runners, sandboxed shells: macOS refuses it, presumably as another app's data) it exits with "Could not find profile folder." (the first full-lane run failed this way). `vitest.browser.config.ts` points `MOZ_APP_DATA`/`MOZ_LOCAL_APP_DATA` at `node_modules/.cache/lamina-firefox/` in the checkout for the Firefox project.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
