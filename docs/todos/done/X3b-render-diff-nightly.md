# X3b — Nightly hayro-vs-PDFium render diffs

| Field | Value |
| --- | --- |
| ID | `X3b` |
| Status | `done` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting), web plan [C2](../../plans/web-app.md#c2-render-backend-and-adrs) ("the hayro-vs-PDFium diff runs in the nightly lane") |
| Round / milestone | Cross-cutting (with Round C) |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [X3a](./X3a-nightly-lane.md) |
| Blocks | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (nightly: hayro vs PDFium render diffs), [0004](../../decisions/0004-render-backend.md) §6 (the metric, thresholds and tracked number: its X3b implementation note) |
| Branch | `ci/x3b-render-diffs` |
| PR title | `ci: nightly hayro-vs-PDFium render diffs (X3b)` |
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

- [x] Implement `render-diff` on top of `lamina-render` (`tools/xtask/src/render_diff.rs`; the reader is `tools/render/src/bin/render-diff.rs`, which reuses `render-golden`'s parity comparison)
- [x] Nightly job, history storage, issue on regression (`render-diff` in `nightly.yml`; the report job files the issue; every write gated on `record`)
- [x] Document the metric and thresholds in ADR 0004 notes or the C2 docs (ADR 0004 §6, implementation note)

## Acceptance criteria

- [ ] `cargo xtask render-diff --corpus external` run on the branch (locally, or through the nightly job with a temporary `push` trigger on this branch, removed before merge) produces a pass-rate number, a JSON report and an HTML summary
- [ ] Two branch runs append two points to a scratch history (a test location, not the real one), and a run against a history whose last point is forced > 2 points higher takes the "open an issue" path (dry run: the issue body is printed instead of filed, or filed with a `test` label and closed)

## Post-merge checks

- [ ] The first scheduled nightly runs after merge produce a pass-rate number and an HTML report artefact for the external corpus
- [ ] The real history shows at least two nightly data points (link the runs in the done todo)

## Tests

Unit tests for the comparator and report generator.

## Docs to update

- [x] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the render-diff part of the nightly-lane item (split from veraPDF's)
- [x] `CLAUDE.md` (layout, commands, the nightly paragraph)

## Risks and open questions

- PDFium in CI (native binary vs WASM): reuse whatever C2 chose.
- Rendering the whole external corpus may take hours; sample pages if needed and record the sampling.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
