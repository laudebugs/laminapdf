# C3c — Viewer performance harness: long tasks, memory and time-to-first-page in CI

| Field | Value |
| --- | --- |
| ID | `C3c` |
| Status | `ready` |
| Plan | [web reader §6 C3. Viewer core](../../plans/web-app.md#c3-viewer-core) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C3b](./C3b-viewer-virtualiser-and-zoom.md) |
| Blocks | [C5a](./C5a-text-layer-and-selection.md), [C5b](./C5b-find-in-document.md), [C6a](./C6a-thumbnails-outline-page-labels.md), [C6c](./C6c-print-download-properties.md), [C7b](./C7b-offline-pwa.md), [C7c](./C7c-performance-gates-and-launch.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) tier 11 and §2 lanes (no wall-clock assertions in PR lanes; perf in nightly; regressions open issues) and §4 (flaky tests are bugs) |
| Branch | `test/c3c-viewer-perf-harness` |
| PR title | `test(viewer): add a playwright performance harness for scrolling, memory and first page` |
| Size | M (200–600 lines) |
| PR | — |

## Goal

Measure the [§4 non-functional targets](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) from C3 onward, as the plan requires, so tuning decisions and later regressions are data-driven. A Playwright harness scrolls a 1,000-page fixture end to end in Chromium, Firefox and WebKit, records long tasks, memory, time-to-first-page and time-to-sharp, and publishes the numbers. C3's acceptance — "scrolls end to end within the memory and long-task budgets; cancelled renders never paint" — is checked here.

## Context

- [web-app.md §4 non-functional table](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) — the numbers.
- [ADR 0018 §1 tier 11, §2](../../decisions/0018-test-strategy.md#1-tiers) — perf runs in the nightly lane; PR lanes don't assert durations; a >10 % regression opens an issue.
- [C3b](./C3b-viewer-virtualiser-and-zoom.md) `renderstats` events and the demo page.
- [C1b result](../done/C1b-pdfium-wasm-spike.md) — memory-measurement methods per browser.

## Scope

**In scope**

- `packages/viewer/perf/` (or `tools/perf/`): Playwright project with three browsers, fixtures generator, scenarios, metrics collection, JSON + Markdown report.
- Scenarios: (1) open 10-page 1 MB PDF, warm cache → first page visible; (2) same, cold load under Fast 4G throttling (Chromium CDP network emulation); (3) scroll 1,000-page text PDF end to end at a fixed speed, twice; (4) fast fling across 300 pages then stop → time until all visible pages sharp; (5) zoom in/out bursts.
- Metrics: `PerformanceObserver('longtask')` (Chromium) and a rAF-gap fallback (Firefox/WebKit), main-thread task max, frames > 50 ms; JS heap + WASM memory + bitmap bytes (from `renderstats` accounting) and `measureUserAgentSpecificMemory` where available; time-to-first-page; time-to-sharp; count of paints for cancelled requests (must be 0).
- CI: a **non-gating** job on PRs touching `packages/viewer` that posts the report as an artifact/comment; the nightly job stores history and opens an issue on > 10 % regression.
- A reference-machine definition (document CPU/RAM/OS for the official numbers; CI runner numbers are relative).

**Out of scope** (and which todo covers it)

- Turning budgets into hard CI gates and adding the app-shell JS size budget → [C7c](./C7c-performance-gates-and-launch.md).
- Search performance → measured in [C5b](./C5b-find-in-document.md) using this harness.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Fixture generator `perf/fixtures.ts`: 10-page ~1 MB mixed text/image PDF and 1,000-page text PDF (plus a 300-page image-heavy PDF) generated with `@laminapdf/pdf` builder, seeded; cached in a gitignored dir.
- [ ] Instrumentation hook in the demo: expose `window.__laminaPerf` (only in the demo/perf build) collecting `renderstats`, long tasks, memory samples every 500 ms.
- [ ] Scenario specs `perf/*.perf.ts` with deterministic scrolling (`mouse.wheel` steps at fixed intervals) and network throttling for the cold scenario.
- [ ] Reporter writing `perf-results/<browser>.json` and `perf-results/summary.md` comparing to §4 targets (pass/fail columns, informational).
- [ ] Memory stability check: memory after the second end-to-end pass ≤ memory after the first + 5 % (detects leaks).
- [ ] GitHub Actions: `perf` job (PR, non-required, path-filtered inside the job per ADR 0018 §2 pattern) and nightly schedule with history kept as a workflow artifact or a branch-free data store; regression >10 % → open an issue automatically.
- [ ] Tune C3b defaults (margin, pool size, cache budget, tile size) based on results and record the chosen values in the viewer README.

## Acceptance criteria

- [ ] The harness runs locally with one command (`pnpm --filter @laminapdf/viewer perf`) and in CI for Chromium, Firefox and WebKit.
- [ ] On the reference machine, the report shows for the 1,000-page fixture: no main-thread task > 50 ms during scroll; visible pages sharp within 300 ms of stopping; total memory ≤ 600 MB and stable on the second pass; cancelled-render paints = 0. Any target missed is listed with an issue link (C7c must close it).
- [ ] First page visible for the 10-page file reported (target ≤ 700 ms warm, ≤ 3 s cold Fast 4G).
- [ ] The nightly job's steps run on the PR branch (the `perf` PR job, or the same script invoked locally with the nightly flags) and produce a history entry; the regression-issue step is exercised once against a forced regression (a dry-run mode that prints the issue it would open is enough before merge).
- [ ] PR lanes contain no wall-clock assertions (ADR 0018).

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

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
