# C3c — Viewer performance harness: long tasks, memory and time-to-first-page in CI

| Field | Value |
| --- | --- |
| ID | `C3c` |
| Status | `done` |
| Plan | [web reader §6 C3. Viewer core](../../plans/web-app.md#c3-viewer-core) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C3b](./C3b-viewer-virtualiser-and-zoom.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) tier 11 and §2 lanes (no wall-clock assertions in PR lanes; perf in nightly; regressions open issues) and §4 (flaky tests are bugs) |
| Branch | `test/c3c-viewer-perf-harness` |
| PR title | `test(viewer): add a playwright performance harness for scrolling, memory and first page` |
| Size | M (200–600 lines) |
| PR | [#254](https://github.com/laudebugs/laminapdf/pull/254) |

## Goal

Measure the [§4 non-functional targets](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) from C3 onward, as the plan requires, so tuning decisions and later regressions are data-driven. A Playwright harness scrolls a 1,000-page fixture end to end in Chromium, Firefox and WebKit, records long tasks, memory, time-to-first-page and time-to-sharp, and publishes the numbers. C3's acceptance — "scrolls end to end within the memory and long-task budgets; cancelled renders never paint" — is checked here.

## Context

- [web-app.md §4 non-functional table](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) — the numbers.
- [ADR 0018 §1 tier 11, §2](../../decisions/0018-test-strategy.md#1-tiers) — perf runs in the nightly lane; PR lanes don't assert durations; a >10 % regression opens an issue.
- [C3b](../done/C3b-viewer-virtualiser-and-zoom.md) `renderstats` events and the demo page.
- [C1b result](../done/C1b-pdfium-wasm-spike.md) — memory-measurement methods per browser.

## Scope

**In scope**

- `packages/viewer/perf/` (or `tools/perf/`): Playwright project with three browsers, fixtures generator, scenarios, metrics collection, JSON + Markdown report.
- Scenarios: (1) open 10-page 1 MB PDF, warm cache → first page visible; (2) same, cold load under Fast 4G throttling (Chromium CDP network emulation); (3) scroll 1,000-page text PDF end to end at a fixed speed, twice; (4) fast fling across 300 pages then stop → time until all visible pages sharp; (5) zoom in/out bursts.
- Metrics: `PerformanceObserver('longtask')` (Chromium) and a rAF-gap fallback (Firefox/WebKit), main-thread task max, frames > 50 ms; JS heap + WASM memory + bitmap bytes (from `renderstats` accounting) and `measureUserAgentSpecificMemory` where available; time-to-first-page; time-to-sharp; count of paints for cancelled requests (must be 0).
- CI: a **non-gating** job on PRs touching `packages/viewer` that posts the report as an artifact/comment; the nightly job stores history and opens an issue on > 10 % regression.
- A reference-machine definition (document CPU/RAM/OS for the official numbers; CI runner numbers are relative).

**Out of scope** (and which todo covers it)

- Turning budgets into hard CI gates and adding the app-shell JS size budget → [C7c](../ready/C7c-performance-gates-and-launch.md).
- Search performance → measured in [C5b](../done/C5b-find-in-document.md) using this harness.

## Tasks

- [x] Claim: the work PR's first commit moves this file to `ongoing/` (git workflow §2), from a fresh worktree off `main`.
- [x] Fixture generator `perf/fixtures.ts`: 10-page ~1 MB mixed text/image PDF and 1,000-page text PDF (plus a 300-page image-heavy PDF) generated with `@laminapdf/sdk` builder, seeded; cached in a gitignored dir.
- [x] Instrumentation hook (on the harness's own page, `perf/page/`, not the demo): expose `window.__laminaPerf` (only in the demo/perf build) collecting `renderstats`, long tasks, memory samples every 500 ms.
- [x] Scenarios (`perf/scenarios.ts`) with deterministic scrolling (`mouse.wheel` steps at fixed intervals) and network throttling for the cold scenario.
- [x] Reporter writing `target/viewer-perf/<browser>.json` and `summary.md` comparing to §4 targets (pass/fail columns, informational).
- [x] Memory stability check: memory after the second end-to-end pass ≤ memory after the first + 5 % (detects leaks).
- [x] GitHub Actions (written; the owner commits them, see Notes): `perf` job (PR, non-required, path-filtered inside the job per ADR 0018 §2 pattern) and nightly schedule with history kept as a workflow artifact or a branch-free data store; regression >10 % → open an issue automatically.
- [x] Tune C3b defaults (margin, pool size, cache budget, tile size) based on results and record the chosen values in the viewer README.

## Acceptance criteria

- [ ] The harness runs locally with one command (`pnpm --filter @laminapdf/viewer perf`) and in CI for Chromium, Firefox and WebKit. **Partly:** it runs locally in Chromium (the only browser here); Firefox and WebKit run in CI once the owner commits the workflows.
- [ ] On the reference machine, the report shows for the 1,000-page fixture: no main-thread task > 50 ms during scroll; visible pages sharp within 300 ms of stopping; total memory ≤ 600 MB and stable on the second pass; cancelled-render paints = 0. Any target missed is listed with an issue link (C7c must close it). **Not yet:** the reference machine's numbers come from the first nightly run with the workflow on `main`. A CI container's full Chromium run met these four; its misses (the cold first page, zooming in) are in C7c's context.
- [x] First page visible for the 10-page file reported (target ≤ 700 ms warm, ≤ 3 s cold Fast 4G): 77 ms warm, 4.2 s cold in Chromium on a CI container.
- [x] The nightly job's steps run on the PR branch (the `perf` PR job, or the same script invoked locally with the nightly flags) and produce a history entry; the regression-issue step is exercised once against a forced regression (a dry-run mode that prints the issue it would open is enough before merge). Done locally: `cargo xtask viewer-perf chromium --quick --history <forced> --runner local --dry-run` against a history whose baselines were far lower printed the issue's table; the history record is unit-tested (`a_record_should_become_the_next_runs_baseline`).
- [x] PR lanes contain no wall-clock assertions (ADR 0018).

## Post-merge checks

- [ ] The scheduled nightly `perf` run on `main` succeeds and stores a history entry (the schedule only fires once the workflow is on `main`).

## Tests

The harness is itself the test suite (`perf/*.perf.ts`, Playwright). Unit-test the reporter's comparison and regression maths with Vitest.

## Docs to update

`packages/viewer/README.md` (perf section, how to run, reference machine); tick "Add a perf harness…" in [web-app.md §9](../../plans/web-app.md#9-to-dos); `CLAUDE.md` commands (the perf command).

## Risks and open questions

- **CI runner noise**: shared runners vary widely; use them for trends and relative regressions only, and record official numbers on the reference machine (who owns it? — the maintainer, documented in the README).
- **Firefox/WebKit lack long-task and memory APIs**; the rAF-gap proxy and our own byte accounting are approximations; say so in the report.
- **Generating 1,000 pages** in test setup needs B6's builder in Node/browser; if slow, cache by seed hash.

## Notes from the work

- **The harness** is `packages/viewer/perf/`, and `cargo xtask viewer-perf` wraps it with the job summary and the nightly history (bench's regression rule, `viewer/<browser>/<figure>` keys, `--dry-run`):
  - `fixtures.ts`: seeded, cached by a hash of its source and of the text module that writes them;
  - `page/`: Vite, over the built package, with `globalThis.__laminaPerf`;
  - `scenarios.ts`: Playwright, with real wheel input;
  - `report.ts` and `report.test.ts`: the §4 checks, the summary and the history entries;
  - `serve.ts`: gzip, as Cloudflare serves the app (a plain Vite preview made the cold figure 7.6 s instead of 4.2);
  - `run.ts`: the command.
- **Two viewer changes.** The document worker answers `memory` (`client.memory()`: the backend's and Lamina PDF's WebAssembly memories), so total memory includes the worker in every browser. And a zoom now marks stretched pages `low` instead of leaving them `sharp` until the new pass paints, which is what made "time to sharp after a zoom" measurable (a browser test pins it).
- **Time to sharp** runs from the last scroll or zoom event to the moment every visible page turned sharp, caught on `renderstats`. Timing from the harness's own polling read 0 ms, and then a 50 ms floor.
- **CI.** Workflow files can't be pushed from here, so the PR description carries them: a `viewer-perf` job in `nightly.yml` after `pdf-runtimes` (history, a regression issue from `report`), and `viewer-perf.yml`, a non-required PR workflow with a path filter, in its own file so the PR lanes' gates are unchanged. `release.rs`' check now requires one gated `RECORD` per job that pushes to the history.
- **Not done here:** the reference machine's numbers (the first nightly run on `main`), and Firefox and WebKit, which only CI has.

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
