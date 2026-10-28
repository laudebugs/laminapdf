# X3a — Nightly CI lane: external-corpus differential, extended fuzzing, scaling tests and benchmarks

| Field | Value |
| --- | --- |
| ID | `X3a` |
| Status | `ready` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting) |
| Round / milestone | Cross-cutting (after the first fuzz targets) |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md) |
| Blocks | [B7b](./B7b-versioning-and-0-1-0-alpha.md), [B8b](./B8b-conformance-profiles-and-verapdf.md), [E1b](./E1b-xmp-packet-model.md), [E1c](./E1c-metadata-ops.md), [E3e](./E3e-xmp-editor.md), [E3f](./E3f-attachments-and-associated-files.md), [E4g](./E4g-appearance-parity-suite.md), [E6c](./E6c-add-text.md), [E12f](./E12f-leak-tests.md), [E15b](./E15b-structure-tree-editing.md), [X3b](./X3b-render-diff-nightly.md) |
| ADRs | Implements [0018](../../decisions/0018-test-strategy.md) §2 nightly lane, §3 corpora (`cargo xtask corpus fetch`, pinned manifest), §4 (nightly may retry once only to report flakiness), tier 11 (performance and scaling); [0003](../../decisions/0003-reader-tolerance-and-limits.md) §4 (differential triage, exceptions file) |
| Branch | `ci/x3a-nightly-lane` |
| PR title | `ci: nightly lane with external-corpus differential, 10-minute fuzzing, scaling tests and benchmarks` |
| Size | L (≈ 600–1,000 lines: workflow, xtask corpus manifest/fetch, differential runners, bench harness) |
| PR | — |

## Goal

Run the expensive, environment-sensitive checks every night on `main`, where a failure opens an issue and blocks the next release instead of unrelated PRs: lamina against pdf.js, PDFium, qpdf and pypdf over large external corpora and all mutants, every fuzz target for 10 minutes, the release-mode scaling tests (including the ignored `reader_cache` test), and benchmarks with history.

## Context

- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) nightly lane; [§3](../../decisions/0018-test-strategy.md#3-corpora-licensing-and-provenance) corpora (pdf.js, PDFium, qpdf suites, SafeDocs, veraPDF and Arlington test files) fetched by `cargo xtask corpus fetch`, pinned by SHA-256 with licences; tier 11 scaling (ratios, never absolute times; regressions > 10% open an issue).
- Ignored scaling test: [`crates/lamina-cos/tests/reader_cache.rs`](../../../crates/lamina-cos/tests/reader_cache.rs) `pages_should_scale_linearly_with_shared_resources` ("run with --release in the nightly lane").
- Scheduled fuzz workflow from [B2c](./B2c-lamina-model-corpus-fuzz.md) (folded in here).

## Scope

**In scope**

- `.github/workflows/nightly.yml` (cron + `workflow_dispatch`) on `main`; on failure, open or update an issue labelled `nightly` (via `gh`). The workflow's latest run status on `main` is what [B7b](./B7b-versioning-and-0-1-0-alpha.md)'s release lane checks (B7b depends on this todo and owns the guard).
- `cargo xtask corpus fetch [--source …]` with `fixtures/corpus-manifest.toml` (URL, SHA-256, licence, size); downloads into gitignored `target/corpus/`; CI cache keyed on the manifest hash; optional R2 mirror URL as a fallback.
- `cargo xtask differential --live --corpus external`: open/repair/fail outcome, page count and outline vs pdf.js (`pdfjs-dist` Node script), PDFium (`pypdfium2`), qpdf (`--json`) and pypdf, with `fixtures/differential/exceptions.toml` (rule from ADR 0003 per entry); all mutants of `fixtures/corpus` (not just the 500 PR-lane set); text differential added when [B9c](./B9c-text-extraction-bindings-and-differential.md) lands (`--text`); round-trip over the external corpus when [B3c](./B3c-save-planner-and-roundtrip.md) lands.
- Fuzzing: every target in `fuzz/` for 10 min (`-max_total_time=600 -timeout=10 -rss_limit_mb=2048`), corpus persisted in the Actions cache, crashes uploaded and an issue opened with the minimised input.
- Scaling: `cargo test --release -- --ignored` for scaling tests (incl. `reader_cache`), retry once only to report flakiness.
- Benchmarks: `benches/` (criterion or divan) for open+walk, decode, save; history stored (e.g. `github-action-benchmark` to a `gh-pages`/data branch); > 10% regression opens an issue, never fails the build.
- Removal of the separate `fuzz.yml` from B2c (its jobs move here).

**Out of scope** (and which todo covers it)

- hayro-vs-PDFium render diffs: [X3b](./X3b-render-diff-nightly.md) (needs web C2's `lamina-render`).
- veraPDF: [B8b](./B8b-conformance-profiles-and-verapdf.md) adds its job to this workflow.
- PR-lane suites: [X4](./X4-full-lane-gate.md).
- The release-lane guard (refuse a release when the latest nightly failed) and running the nightly suites on a release commit: [B7b](./B7b-versioning-and-0-1-0-alpha.md), which owns the release lane.

## Tasks

- [ ] Write `fixtures/corpus-manifest.toml` (start with pdf.js test PDFs that are redistributable-licence-checked, qpdf test suite, PDFium testing corpus, SafeDocs subset); `cargo xtask corpus fetch`
- [ ] Differential runners in `tools/differential/` (pinned versions, manifest of tool versions in the report)
- [ ] `nightly.yml` jobs: differential, fuzz (matrix over targets), scaling, benches; issue-on-failure step
- [ ] Bench harness in `crates/lamina-cos/benches/` (+ save benches once B3b exists)
- [ ] Document in `CLAUDE.md` and ADR 0018 (implementation notes)

## Acceptance criteria

- [ ] `nightly.yml` passes `actionlint`; its jobs run on the branch through a temporary `pull_request`/`push` trigger (or `act`) with a short budget (`--time 60` per fuzz target, a manifest subset) and that trigger is removed before merge; run linked in the PR
- [ ] The issue-on-failure step is exercised on the branch run with a forced failure (input or env flag) and opens/updates a `nightly` issue (closed afterwards)
- [ ] `cargo xtask corpus fetch` downloads and verifies the manifest (SHA-256) locally; a tampered hash fails
- [ ] The `reader_cache` scaling test passes locally with `cargo test --release -- --ignored`
- [ ] The differential report (branch run) lists per-tool agreement rates and every disagreement is in the exceptions file or reported as a new issue

## Post-merge checks

- [ ] First scheduled nightly run on `main` is green (link it in a follow-up PR or the done todo)
- [ ] A `workflow_dispatch` run from `main` with the failing input opens an issue
- [ ] Every fuzz target runs the full 10 minutes nightly; crashes become artefacts and issues
- [ ] Benchmark history is recorded across at least two nightly runs

## Tests

The workflow itself; unit tests for manifest parsing and the differential comparator in xtask.

## Docs to update

- [ ] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the nightly-lane item (differential, veraPDF part by B8b, render diffs by X3b, benchmarks)
- [ ] [pdf-library §10 B2](../../plans/pdf-library.md#10-to-dos) "run 10 min in CI" (if not already ticked by B2c)
- [ ] `CLAUDE.md`, `fixtures/README.md` (corpus manifest)

## Risks and open questions

- Corpus licences: only fetch, never commit; record each source's licence in the manifest.
- Runner time: hours per night; use the self-hosted runner when online (PR #11's selector) and keep GitHub-hosted fallback within limits.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
