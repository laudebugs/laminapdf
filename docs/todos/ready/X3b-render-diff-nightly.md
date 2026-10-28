# X3b — Nightly hayro-vs-PDFium render diffs

| Field | Value |
| --- | --- |
| ID | `X3b` |
| Status | `ready` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting), web plan [C2](../../plans/web-app.md#c2-render-backend-and-adrs) ("the hayro-vs-PDFium diff runs in the nightly lane") |
| Round / milestone | Cross-cutting (with Round C) |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [X3a](./X3a-nightly-lane.md) |
| Blocks | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (nightly: hayro vs PDFium render diffs), ADR 0004 (written in C2) |
| Branch | `ci/x3b-render-diffs` |
| PR title | `ci: nightly hayro-vs-PDFium render diffs with pass-rate history` |
| Size | M (≈ 300–600 lines) |
| PR | — |

## Goal

Measure, every night, how far hayro is from PDFium on the external corpus, so the decision to make hayro the default renderer (architecture principle 4, ADR 0004) is made on data. Render every page of the corpus with both adapters of `lamina-render`, compare perceptually, and record the pass rate and the worst offenders over time.

## Context

- [Architecture §2 principle 4](../../plans/architecture.md#principles-each-becomes-part-of-an-adr) (rendering is swappable; hayro becomes default at parity), [§4 `hayro-parity` spike](../../plans/architecture.md#4-spikes-to-close-open-questions).
- [Rendering notes §1, §5](../../research/architecture/notes/rendering.md#5-recommendation-options-and-risks).
- Render-golden tooling from [B5c](./B5c-lamina-facade-and-examples.md) (thresholds, diff images).

## Scope

**In scope**

- `cargo xtask render-diff --corpus external [--pages N]`: render via `lamina-render` (both backends, same DPI), compare (SSIM / per-pixel ΔE with thresholds from the `hayro-parity` spike), emit a JSON report and an HTML summary with the top-N diffs.
- Nightly job in `nightly.yml`, artefacts uploaded; pass-rate history appended (same storage as benchmarks).
- A regression in pass rate > 2 points opens an issue (not a failure).

**Out of scope**

- PR-lane render goldens (B5c and C2 own them).

## Tasks

- [ ] Implement `render-diff` on top of `lamina-render`
- [ ] Nightly job, history storage, issue on regression
- [ ] Document the metric and thresholds in ADR 0004 notes or the C2 docs

## Acceptance criteria

- [ ] Nightly run produces a pass-rate number and an HTML report for the external corpus
- [ ] History shows at least two data points; a forced regression opens an issue

## Tests

Unit tests for the comparator and report generator.

## Docs to update

- [ ] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the render-diff part of the nightly-lane item
- [ ] `CLAUDE.md`

## Risks and open questions

- PDFium in CI (native binary vs WASM): reuse whatever C2 chose.
- Rendering the whole external corpus may take hours; sample pages if needed and record the sampling.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
