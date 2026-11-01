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
| PR | [#133](https://github.com/laudebugs/lamina/pull/133) |

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

- [x] `cargo xtask render-diff --corpus external` run on the branch (locally, or through the nightly job with a temporary `push` trigger on this branch, removed before merge) produces a pass-rate number, a JSON report and an HTML summary
  - Locally on the shared Mac (macOS ARM64, 2026-10-31), the nightly's sampling: every file of the four sources, the first 20 pages of each, 96 DPI, `--jobs 4`: **5,108 rows, pass+minor 92.19 %, pass 86.77 %**; per source pass+minor: pdf.js 90.56 % (1,472 rows), PDFBox 93.85 % (309), PDFium 77.96 % (422), qpdf 94.91 % (2,905). Classes: 4,432 pass, 277 minor, 194 fail, 78 hayro-error, 27 pdfium-error, 95 both-error, 5 timeout. The seven files that crashed or hung were each PDFium's alone (`jpx_numcomps.pdf` and `bug_457855936.pdf` crash; `operator_list_cycle.pdf`, both `tiling-pattern-tiny-steps*.pdf`, `bug_451265.pdf` and `bug_509964646.pdf` hang), as in C1a. pdf.js's 90.6 % is C1a's 90.8 % within the page cap. No temporary `push` trigger was added. These figures predate the review fix that counts a crashed, hung or unopened file once per page instead of once, so the first nightly's numbers will be slightly lower; a 10-file smoke run after the fix gave 16 rows, pass+minor 81.25 %.
- [x] Two branch runs append two points to a scratch history (a test location, not the real one), and a run against a history whose last point is forced > 2 points higher takes the "open an issue" path (dry run: the issue body is printed instead of filed, or filed with a `test` label and closed)
  - Two full runs with `--history <scratch> --runner local-mac --record` appended two identical points (the renders are deterministic); the second reported "no rate dropped by more than 2 points". To keep it short, the forced case used a 200-file sample: a `--record` run into a copy of that history, then its last point raised by 2.5 points (`all`), 5 (`pdfjs`) and exactly 2 (`qpdf`), then `--dry-run`: the two drops above 2 points were reported (`regression.md`, `::warning::`) and the issue printed, qpdf's exactly-2-point drop was not, and nothing was appended. No issue was filed.

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
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **Where the code lives.** The reader is a second binary of `tools/render` (`render-diff`, next to `render-golden`), so it reuses `render-golden`'s parity comparison (`compare::parity`, C1a's metric) and image code. That workspace now depends on `lamina-render` (feature `hayro`) and `lamina-render-pdfium` by path; neither crate changed. `cargo xtask render-diff` (`tools/xtask/src/render_diff.rs`) builds it, runs it through `runner.rs` and writes the reports.
- **Differences from C1a's harness.** C1a restarted a crashed or hung file at the next page. Here `runner.rs` isolates files: the pages finished before a crash or hang are kept (each row is written as it is known), the rest count as that crash or hang once per page (by lamina's or an engine's page count, capped at `--pages`), and the file is then rendered by each engine alone to say which engine to blame. Page sizes that differ by one pixel are compared on their common part, as `render-golden`'s parity check does; C1a failed them. The nightly lane caps each file at 20 pages, so its rates aren't C1a's whole-corpus figures. The corpus is the nightly's four sources (pdf.js, PDFBox, PDFium, qpdf), not C1a's two.
- **Run time.** The whole corpus took about an hour on the shared Mac, most of it waiting on the seven PDFium hangs (each costs one 6-minute chunk timeout plus single-file reruns). The job's limit is 180 minutes.
