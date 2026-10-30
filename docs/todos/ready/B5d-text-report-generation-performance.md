# B5d — Generation performance: a 100-page text report within 500 ms

| Field | Value |
| --- | --- |
| ID | `B5d` |
| Status | `ready` |
| Plan | [pdf-library §4 non-functional](../../plans/pdf-library.md#non-functional), [§7 B5](../../plans/pdf-library.md#b5-builder-and-the-lamina-crate) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B6b](../done/B6b-lamina-pdf-package.md) |
| Blocks | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) (no wall-clock asserts in tests: the timing goes in a benchmark), [0016](../../decisions/0016-package-design.md) §6 (a fix must not push a module over its budget) |
| Branch | `perf/b5d-text-report-generation` |
| PR title | `perf(build): a 100-page text report within the 500 ms target` |
| Size | M (profiling, a benchmark, targeted fixes in lamina-text / lamina-build) |
| PR | — |

## Goal

Plan §4 asks for a 100-page text report in ≤ 500 ms after `init()`. B6b measured 2.96 s in `@lamina/pdf` (Node 24, the text module, on the M-series Mac): a Helvetica-Bold heading and six 4-sentence Noto Sans text boxes per page (600 text boxes, 85 KB output). Find where the time goes, fix it, and keep a benchmark so it can't regress silently.

## Context

- [B6b's Outcome](../done/B6b-lamina-pdf-package.md#outcome): the measurement and its script shape (`hello` 2.6 ms, report 2.96 s, text-module memory 5 MB).
- `lamina-text` layout ([B4b](../done/B4b-shaping-bidi-line-breaking.md)): `break_lines` measures by shaping; a greedy breaker that reshapes each candidate line is a likely cost. `lamina-build`'s `text_box` and `FontRegistry::finish_fonts` (subsetting) are the other suspects.
- The criterion benches and their history in the nightly lane (`cargo xtask bench`, `crates/lamina-cos/benches`, `crates/lamina-save/benches`).

## Scope

**In scope**

- Measure the same report natively (`cargo bench`) and in WASM (Node) to separate core cost from binding cost (JSON records, copies).
- Profile (e.g. `cargo flamegraph` / samply natively; the Node profiler for WASM) and fix the top costs in lamina-text / lamina-build without changing output bytes (the goldens stay byte-identical).
- A criterion bench `report_100_pages` in `crates/lamina-build/benches` (or the façade), run by `cargo xtask bench` with its history.

**Out of scope** (and which todo covers it)

- Worker memory under 64 MB (measured at 5 MB by B6b; re-check only).
- Runtime matrix timing in browsers: [B6c](../ongoing/B6c-runtime-matrix.md).

## Tasks

- [ ] Add the report as a bench (native) and a Node script under `packages/pdf/` (not a test: no wall-clock asserts)
- [ ] Profile, record the top items before and after in this todo's Outcome
- [ ] Fix (shaping caches per run/font, fewer reshapes in line breaking, fewer copies across the binding)
- [ ] Check that the text module stays within its 960 KB budget (`cargo xtask size`)

## Acceptance criteria

- [ ] The 100-page report runs in ≤ 500 ms in Node on the reference machine (recorded, not asserted), or the remaining gap is explained and a decision on the target recorded with the owner
- [ ] Every golden in `fixtures/golden/` and `fixtures/golden/examples/` is unchanged
- [ ] The bench is part of `cargo xtask bench`

## Post-merge checks

- [ ] The nightly bench history shows the new bench

## Tests

The existing golden and example tests (bytes must not change); the new bench.

## Docs to update

- [ ] [pdf-library §7 B5](../../plans/pdf-library.md#b5-builder-and-the-lamina-crate) and §4 (the measured numbers)

## Risks and open questions

- The cost may be inherent to shaping every line (HarfRust); then caching by (font, text, features) is the main lever.
- If the target is unrealistic for 600 shaped boxes, the owner decides whether to revise it (as with the ADR 0016 budgets).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
