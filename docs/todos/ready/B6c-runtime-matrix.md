# B6c — Runtime matrix in CI: browsers, Node 22/24/26, Deno, Bun and workerd

| Field | Value |
| --- | --- |
| ID | `B6c` |
| Status | `ready` |
| Plan | [pdf-library §7 B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf), [§5 platform matrix](../../plans/pdf-library.md#platform-matrix) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B6b](./B6b-lamina-pdf-package.md) |
| Blocks | [B7a](./B7a-docs-site.md), [B7b](./B7b-versioning-and-0-1-0-alpha.md), [B9c](./B9c-text-extraction-bindings-and-differential.md) |
| ADRs | Implements 0016's runtime support claims; [0018](../../decisions/0018-test-strategy.md) §2 (lanes: which runtimes run per PR vs nightly/release) |
| Branch | `ci/b6c-runtime-matrix` |
| PR title | `ci(pdf): run the @lamina/pdf example suite in browsers, Node, Deno, Bun and workerd` |
| Size | M (≈ 400–700 lines: test configs, workflow, a Worker fixture) |
| PR | — |

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
- A Worker example (`packages/pdf/examples/worker/`) that generates a PDF per request — also used by docs ([B7a](./B7a-docs-site.md)).
- Performance smoke (not asserted in PR lanes; recorded): hello and 100-page report timings per runtime written to the job summary; the nightly trend is a job in `.github/workflows/nightly.yml` added here, recording the timings in the benchmark history (`history.jsonl` on `bench-data`, the record format of `cargo xtask bench` from [X3a](../done/X3a-nightly-lane.md)).

**Out of scope** (and which todo covers it)

- Clean-install-from-tarball tests: [B7b](./B7b-versioning-and-0-1-0-alpha.md).
- Bundler matrix (Vite, webpack, esbuild) smoke builds: included here only as a single Vite build check; a fuller matrix is a follow-up if failures appear.

## Tasks

- [ ] Add dev deps (`@vitest/browser`, `playwright`, `@cloudflare/vitest-pool-workers`, `wrangler`) and check npm licences
- [ ] Configs and shared test entry; ensure tests read goldens without Node APIs in browser/worker (fetch fixtures via Vite's asset handling)
- [ ] Deno/Bun runners; Vite build smoke
- [ ] Workflow jobs (fast vs full lanes) with Playwright browser cache
- [ ] Worker example + memory measurement (workerd reports) for the 100-page report with two fonts
- [ ] Document how to run each runtime locally in `packages/pdf/README.md` and `CLAUDE.md`

## Acceptance criteria

- [ ] The example suite passes in Chromium, Firefox, WebKit, Node 22/24/26, Deno, Bun and workerd with byte-identical outputs
- [ ] A Vite production build of a tiny app importing `@lamina/pdf` works in Chromium
- [ ] Timings and Worker peak memory are reported per runtime; any miss of the §4 targets is listed in the PR and filed as an issue (not a CI failure)

## Post-merge checks

None.

## Tests

As above; all under `packages/pdf/test/` sharing one example list.

## Docs to update

- [ ] [pdf-library §10 B6](../../plans/pdf-library.md#10-to-dos): tick "Run the TS example suite in …"
- [ ] `CLAUDE.md` commands, package README

## Risks and open questions

- WebKit on Linux CI can be flaky; per ADR 0018 §4 a flaky test is a bug: mark with an issue, don't retry.
- workerd's `.wasm` import rules differ from browsers; the `workerd` condition must point to a module that imports the compiled module, verified by attw/publint where possible.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
