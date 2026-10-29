# X3a — Nightly CI lane: external-corpus differential, extended fuzzing, scaling tests and benchmarks

| Field | Value |
| --- | --- |
| ID | `X3a` |
| Status | `done` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting) |
| Round / milestone | Cross-cutting (after the first fuzz targets) |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md) |
| Blocks | [B7b](../ready/B7b-versioning-and-0-1-0-alpha.md), [B8b](../ready/B8b-conformance-profiles-and-verapdf.md), [E1b](../ready/E1b-xmp-packet-model.md), [E1c](../ready/E1c-metadata-ops.md), [E3e](../ready/E3e-xmp-editor.md), [E3f](../ready/E3f-attachments-and-associated-files.md), [E4g](../ready/E4g-appearance-parity-suite.md), [E6c](../ready/E6c-add-text.md), [E12f](../ready/E12f-leak-tests.md), [E15b](../ready/E15b-structure-tree-editing.md), [X3b](../ready/X3b-render-diff-nightly.md) |
| ADRs | Implements [0018](../../decisions/0018-test-strategy.md) §2 nightly lane, §3 corpora (`cargo xtask corpus fetch`, pinned manifest), §4 (nightly may retry once only to report flakiness), tier 11 (performance and scaling); [0003](../../decisions/0003-reader-tolerance-and-limits.md) §4 (differential triage, exceptions file) |
| Branch | `ci/x3a-nightly-lane` |
| PR title | `ci: nightly lane with external-corpus differential, 10-minute fuzzing, scaling tests and benchmarks` |
| Size | L (≈ 600–1,000 lines: workflow, xtask corpus manifest/fetch, differential runners, bench harness) |
| PR | [#32](https://github.com/laudebugs/lamina/pull/32) |

## Goal

Run the expensive, environment-sensitive checks every night on `main`, where a failure opens an issue and blocks the next release instead of unrelated PRs: lamina against pdf.js, PDFium, qpdf and pypdf over large external corpora and all mutants, every fuzz target for 10 minutes, the release-mode scaling tests (including the ignored `reader_cache` test), and benchmarks with history.

## Context

- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) nightly lane; [§3](../../decisions/0018-test-strategy.md#3-corpora-licensing-and-provenance) corpora (pdf.js, PDFium, qpdf suites, SafeDocs, veraPDF and Arlington test files) fetched by `cargo xtask corpus fetch`, pinned by SHA-256 with licences; tier 11 scaling (ratios, never absolute times; regressions > 10% open an issue).
- Ignored scaling test: [`crates/lamina-cos/tests/reader_cache.rs`](../../../crates/lamina-cos/tests/reader_cache.rs) `pages_should_scale_linearly_with_shared_resources` ("run with --release in the nightly lane").
- Scheduled fuzz workflow from [B2c](../done/B2c-lamina-model-corpus-fuzz.md) (folded in here).

## Scope

**In scope**

- `.github/workflows/nightly.yml` (cron + `workflow_dispatch`) on `main`; on failure, open or update an issue labelled `nightly` (via `gh`). The workflow's latest run status on `main` is what [B7b](../ready/B7b-versioning-and-0-1-0-alpha.md)'s release lane checks (B7b depends on this todo and owns the guard).
- `cargo xtask corpus fetch [--source …]` with `fixtures/corpus-manifest.toml` (URL, SHA-256, licence, size); downloads into gitignored `target/corpus/`; CI cache keyed on the manifest hash; optional R2 mirror URL as a fallback.
- `cargo xtask differential --live --corpus external`: open/repair/fail outcome, page count and outline vs pdf.js (`pdfjs-dist` Node script), PDFium (`pypdfium2`), qpdf (`--json`) and pypdf, with `fixtures/differential/exceptions.toml` (rule from ADR 0003 per entry); all mutants of `fixtures/corpus` (not just the 500 PR-lane set); text differential added when [B9c](../ready/B9c-text-extraction-bindings-and-differential.md) lands (`--text`); round-trip over the external corpus when [B3c](../ready/B3c-save-planner-and-roundtrip.md) lands.
- Fuzzing: every target in `fuzz/` for 10 min (`-max_total_time=600 -timeout=10 -rss_limit_mb=2048`), corpus persisted in the Actions cache, crashes uploaded and an issue opened with the minimised input.
- Scaling: `cargo test --release -- --ignored` for scaling tests (incl. `reader_cache`), retry once only to report flakiness.
- Benchmarks: `benches/` (criterion or divan) for open+walk, decode, save; history stored (e.g. `github-action-benchmark` to a `gh-pages`/data branch); > 10% regression opens an issue, never fails the build.
- Removal of the separate `fuzz.yml` from B2c (its jobs move here).

**Out of scope** (and which todo covers it)

- hayro-vs-PDFium render diffs: [X3b](../ready/X3b-render-diff-nightly.md) (needs web C2's `lamina-render`).
- veraPDF: [B8b](../ready/B8b-conformance-profiles-and-verapdf.md) adds its job to this workflow.
- PR-lane suites: [X4](../ongoing/X4-full-lane-gate.md).
- The release-lane guard (refuse a release when the latest nightly failed) and running the nightly suites on a release commit: [B7b](../ready/B7b-versioning-and-0-1-0-alpha.md), which owns the release lane.

## Tasks

- [x] Write `fixtures/corpus-manifest.toml` (start with pdf.js test PDFs that are redistributable-licence-checked, qpdf test suite, PDFium testing corpus, SafeDocs subset); `cargo xtask corpus fetch`. **Deviation:** pdf.js, PDFBox, PDFium and qpdf (2,172 files, 177 MB, pinned by commit with per-file SHA-256); SafeDocs is left out (multi-GB archives with mixed provenance), and so are PDFium's `.in` templates (they need its generator script); the mirror fallback is wired (`mirror` / `$LAMINA_CORPUS_MIRROR`) but no mirror exists yet
- [x] Differential runners in `tools/differential/` (pinned versions, manifest of tool versions in the report)
- [x] `nightly.yml` jobs: differential, fuzz (matrix over targets), scaling, benches; issue-on-failure step
- [x] Bench harness in `crates/lamina-cos/benches/` (+ save benches once B3b exists): criterion, open + walk, Flate decode, `Writer` serialisation; save benches still wait for B3b
- [x] Document in `CLAUDE.md` and ADR 0018 (implementation notes)

## Acceptance criteria

- [x] `nightly.yml` passes `actionlint`; its jobs run on the branch through a temporary `pull_request`/`push` trigger (or `act`) with a short budget (`--time 60` per fuzz target, a manifest subset) and that trigger is removed before merge; run linked in the PR. Runs: [1](https://github.com/laudebugs/lamina/actions/runs/40829874450) (found the case-sensitivity bug in the qpdf patterns), [2](https://github.com/laudebugs/lamina/actions/runs/40830476951) (found a criterion cache bug in `bench`), [3](https://github.com/laudebugs/lamina/actions/runs/40831485187) (every job green except the forced failure). Budgets: 60 s per fuzz target, a 600-input differential sample
- [x] The issue-on-failure step is exercised on the branch run with a forced failure (input or env flag) and opens/updates a `nightly` issue (closed afterwards). Run 1 opened [#33](https://github.com/laudebugs/lamina/issues/33) and runs 2 and 3 commented on it. **Deviation:** #33 is still open; closing it needs the owner (this session wasn't permitted to write to issues)
- [x] `cargo xtask corpus fetch` downloads and verifies the manifest (SHA-256) locally; a tampered hash fails
- [x] The `reader_cache` scaling test passes locally with `cargo test --release -- --ignored` (`cargo xtask scaling`)
- [x] The differential report (branch run) lists per-tool agreement rates and every disagreement is in the exceptions file or reported as a new issue. Run 3: 0 failures, 57 splits, 178 excused, 1 known. **Deviation:** only disagreements where every tool disagrees with lamina must be excused; splits are listed in the report (see Notes)

## Post-merge checks

- [ ] First scheduled nightly run on `main` is green (link it in a follow-up PR or the done todo)
- [ ] A `workflow_dispatch` run from `main` with the failing input opens an issue
- [ ] Every fuzz target runs the full 10 minutes nightly; crashes become artefacts and issues
- [ ] Benchmark history is recorded across at least two nightly runs

## Tests

The workflow itself; unit tests for manifest parsing and the differential comparator in xtask.

## Docs to update

- [x] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the nightly-lane item (differential, veraPDF part by B8b, render diffs by X3b, benchmarks)
- [x] [pdf-library §10 B2](../../plans/pdf-library.md#10-to-dos) "run 10 min in CI" (if not already ticked by B2c)
- [x] `CLAUDE.md`, `fixtures/README.md` (corpus manifest); also `fuzz/README.md` and `tools/differential/README.md`

## Risks and open questions

- Corpus licences: only fetch, never commit; record each source's licence in the manifest.
- Runner time: hours per night; use the self-hosted runner when online (PR #11's selector) and keep GitHub-hosted fallback within limits.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **What fails the differential.** A fact fails only when every tool that compared it disagrees with lamina and `exceptions.toml` doesn't excuse it, or when lamina panics; disagreements where some tool agrees with lamina are reported as splits (ADR 0018 implementation notes). The first full local run (5,686 inputs) had 436 such failures before triage. Seven value-matched `[[class]]` rules, each citing an ADR 0003 rule, explain all but 5; those are `[[known]]` entries. Final run: 0 failures, 567 splits, 1,755 excused, 5 known.
- **Known disagreements.** Two `enc` mutants fail with `syntax` where every other reader opens them; undefined PDFDocEncoding bytes in outline titles become U+FFFD; `issue22011.pdf` has one page fewer. They need a tracking issue; until one exists the `[[known]]` entries point at [#32](https://github.com/laudebugs/lamina/pull/32).
- **Benchmarks** run on GitHub-hosted Ubuntu and keep `history.jsonl` on the `bench-data` branch, which the first run on `main` creates. The baseline is the median of the last five runs; noisy runners may still trip the 10% threshold.
