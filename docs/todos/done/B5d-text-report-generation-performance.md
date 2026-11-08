# B5d — Generation performance: a 100-page text report within 500 ms

| Field | Value |
| --- | --- |
| ID | `B5d` |
| Status | `done` |
| Plan | [pdf-library §4 non-functional](../../plans/pdf-library.md#non-functional), [§7 B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B6b](./B6b-lamina-pdf-package.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) (no wall-clock asserts in tests: the timing goes in a benchmark), [0016](../../decisions/0016-package-design.md) §6 (a fix must not push a module over its budget) |
| Branch | `perf/b5d-text-report-generation` |
| PR title | `perf(build): a 100-page text report within the 500 ms target` |
| Size | M (profiling, a benchmark, targeted fixes in lamina-text / lamina-build) |
| PR | [#92](https://github.com/laudebugs/lamina/pull/92) |

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
- Runtime matrix timing in browsers: [B6c](../done/B6c-runtime-matrix.md).

## Tasks

- [x] Add the report as a bench (native) and a Node script under `packages/sdk/` (not a test: no wall-clock asserts)
- [x] Profile, record the top items before and after in this todo's Outcome
- [x] Fix (shaping caches per run/font, fewer reshapes in line breaking, fewer copies across the binding; deviation: the binding was not the cost, see Outcome)
- [x] Check that the text module stays within its 960 KB budget (`cargo xtask size`)

## Acceptance criteria

- [x] The 100-page report runs in ≤ 500 ms in Node on the reference machine (recorded, not asserted), or the remaining gap is explained and a decision on the target recorded with the owner
- [x] Every golden in `fixtures/golden/` and `fixtures/golden/examples/` is unchanged
- [x] The bench is part of `cargo xtask bench`

## Post-merge checks

- [ ] The nightly bench history shows the new bench

## Tests

The existing golden and example tests (bytes must not change); the new bench.

## Docs to update

- [x] [pdf-library §7 B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate) and §4 (the measured numbers)

## Risks and open questions

- The cost may be inherent to shaping every line (HarfRust); then caching by (font, text, features) is the main lever.
- If the target is unrealistic for 600 shaped boxes, the owner decides whether to revise it (as with the ADR 0016 budgets).

## Outcome

- Benchmarks: `crates/lamina/benches/report.rs` (criterion, in `cargo xtask bench`'s list, so its history goes to `bench-data` nightly): `generate/report_100_pages`, the Rust twin of `packages/sdk/examples/report.ts` (B6c's report, the one `cargo xtask pdf-runtimes` times in every runtime; the bench checks its bytes against the TS suite's `REPORT_SHA256` before timing), and `generate/report_100_pages_text_boxes`, B6b's shape (a Helvetica-Bold heading and six 4-sentence Noto Sans text boxes per page). In the façade rather than `lamina-build`: the report ends with a save, as in Node. `packages/sdk/bench/reports.mjs` times both reports in Node over the built `dist/` (`node packages/sdk/bench/reports.mjs [runs]`; Node ≥ 22.18 for the type-stripped import of `examples/report.ts`).
- Profiles (native: macOS `sample` on the bench; WASM: `node --cpu-prof` with the named `target/wasm/text/lamina_text.named.wasm`), share of the B6c report's time:
  - Before: shaping 46 %, of which building HarfRust's shape plan for every run (`ShapePlan::new`: `find_language_feature`, `langsys`, the map compile) 25 % and the shaping itself the rest; line breaking shaped each short text twice (once for the estimates, once as the line); `pdf_run` 21 % (per-glyph `String`s and `Vec`s for the ToUnicode claims); Flate of the content streams and fonts 10 %.
  - After (native): Flate 26 % (it fixes the output bytes, so it stays), shaping 20 %, `pdf_run` 20 %, itemisation 7 %. Node (B6c report): text layout 34 % (shaping 15 %, itemisation 7 %), page writing 28 % (`pdf_run` 15 %, Flate 9 %), font subsetting 2 %; the JS glue is about 5 %, so the binding was not the cost.
- Fixes (no output byte changed: every golden, the examples and `REPORT_SHA256` are as before):
  - `lamina-text`: each font face caches HarfRust shape plans, found with HarfRust's own `ShapePlanKey::matches` (direction, script, language, features, feature variations; no allocation on a hit), 64 per face with the least recently used dropped, shared by clones and instances (`Font::shape_plan`); a cached plan is the one HarfRust would build for the buffer.
  - `break_lines` memoises shaping per paragraph by (item, range, level): the estimates' runs serve lines that hold an item whole (most single-line texts, which were shaped twice), and candidate lines that line breaking measures again (backing off, a longer line, splitting an overlong word, the line it keeps) are shaped once; candidates share the memo's runs and only kept lines copy them. A test with a counting shaper fails if any range is shaped twice. On the text-box report the memo beyond the whole-item reuse measured no change (67.7 vs 67.8 ms native): its paragraphs rarely back off.
  - `pdf_run`: a one-glyph cluster claims its text without `split_cluster`/`in_reading_order`'s copies; the cluster check compares without concatenating; default-ignorable and script lookups skip ASCII (tested against the Unicode data for all 128).
  - `lamina-build`: laid-out runs are moved, not cloned.
- Measured on the M-series Mac, shared with CI and another session: the load average was 170–250 on 8 cores throughout, so the timings are noisy; before and after were alternated under the same load. Native (minimum of 20 builds in a plain timing loop over the bench's code): B6c report 78 → 39 ms, text-box report 107 → 64 ms (criterion: 48 ms and 91 ms). Node 24 (`cargo xtask pdf-runtimes node`, median of 3 cold runs): B6c report 658–958 ms → 251–422 ms; `reports.mjs` (minimum of 10): B6c report 365–370 → 167–178 ms, text-box report 339–517 → 200–329 ms. WebAssembly memory 4.1 → 3.9 MB.
- Target: the B6c report is within 500 ms in Node even under this load (B6b's 2.96 s for the text-box report is now 0.2–0.33 s). Nothing about the target needs the owner's decision.
- Sizes (`cargo xtask size`, gzip): base 372,711 B (unchanged), text 946,912 B (+2,988 B, budget 960,000 B).
- Deviations: the bench is in `crates/lamina/benches` (the façade), which the scope allows; no binding change was made (the profile shows the core, not the JSON records, dominates); the B6b text-box report has no committed TS example, so its two copies (the bench and `reports.mjs`) are twins by construction, not checked byte for byte.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
