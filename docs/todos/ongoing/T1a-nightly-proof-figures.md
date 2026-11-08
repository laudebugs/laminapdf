# T1a — The nightly records the figures that show how the core copes with real files

| Field | Value |
| --- | --- |
| ID | `T1a` |
| Status | `ongoing` |
| Plan | [trust and proof §T1](../../plans/trust-and-proof.md#t1-publish-the-proof) |
| Round / milestone | Round T (trust and proof) |
| Depends on | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md#2-ci-lanes) §2 (the nightly lane; nothing recorded when the release lane calls it) and tiers 6, 7, 9 and 10; [0004 §6](../../decisions/0004-render-backend.md#6-when-hayro-becomes-the-default-c1as-criteria) (the render-diff metric); [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (veraPDF and the readers stay external processes). Adds a paragraph to ADR 0018's [nightly implementation note](../../decisions/0018-test-strategy.md#nightly-lane-x3a-2026-10-29) |
| Branch | `feat/t1a-nightly-proof-figures` |
| PR title | `feat(xtask): cargo xtask proof records the nightly's robustness figures` |
| Size | L (≈ 500–700 lines with tests: a new xtask module, three small report writers, two nightly jobs). If it passes 800, move the three writers and the fuzz-matrix fix into a first PR |
| PR | [#311](https://github.com/laudebugs/laminapdf/pull/311) |

## Goal

Each night the lane measures how Lamina PDF does on 2,172 files from four projects' test suites. It compares Lamina PDF with pdf.js, PDFium, qpdf and pypdf, fuzzes every parser entry point and renders pages against PDFium. It also checks the examples with veraPDF and builds them in seven runtimes. Those numbers sit in job summaries and artefacts that expire, where nobody outside the repository sees them. This todo adds `cargo xtask proof`, which gathers them after a full nightly into one versioned record. The record is appended to the `bench-data` history beside the benchmark figures, so [T1b](../ready/T1b-how-we-test-page.md) can publish it.

Hacker News users judge PDF tools by named failures, not feature lists: one tester found that "Only three of them can process all 2500 files i tried" ([44785629](https://news.ycombinator.com/item?id=44785629)), after trying about 25 engines. Show HN saw 359 PDF launches in 2026, at a median of 2 points. Figures anyone can check are what a new engine has to offer against that ([report R1](../../research/hacker-news/pdf-users-on-hacker-news.md#r1-prove-correctness-in-public)).

## Context

- [Report R1](../../research/hacker-news/pdf-users-on-hacker-news.md#r1-prove-correctness-in-public) and the [extraction note](../../research/hacker-news/notes/extraction.md) (its recommendation 8: publish the share of a public corpus opened without error and the fuzzing record), the [format and security note](../../research/hacker-news/notes/format-and-security.md) (malformed files, readers that disagree).
- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) and its [nightly implementation note](../../decisions/0018-test-strategy.md#nightly-lane-x3a-2026-10-29): what each nightly job runs, the `record` input the release lane sets to `false`, and the issue rules.
- `.github/workflows/nightly.yml`: the jobs (`differential`, `fuzz`, `scaling`, `bench`, `pdf-runtimes`, `links`, `render-diff`, `verapdf`, `forced-failure`, `report`), the `RECORD` expression each recording job uses, and the artefacts they upload. Its `fuzz` matrix lists nine targets; `harness::FUZZ_TARGETS` has ten (`extract`, added in L3, is missing from the nightly).
- `tools/xtask/src/nightly.rs`: `history_record`, `append_record` and `read_history`, the record format every recording task shares (`commit`, `date`, `runner`, `quick`, `benches`), and `cargo xtask nightly`, the lane run locally.
- `.github/actions/bench-history/action.yml`: `mode: fetch` and `mode: push` of `history.jsonl` on the orphan `bench-data` branch, with retries on a refused push.
- What each task writes today, which the record reads:
  - `tools/xtask/src/live.rs` and `live/compare.rs`: `target/nightly/differential/report.json` (per tool: `files`, `timeouts`, `crashes` and an `agree`/`disagree`/`excused` tally per fact; `lamina_errors`, `missing`, `broken_readers`, `sample`) and each reader's own report, `lamina.json` included (`facts`: per file `outcome` `opened`, `repaired`, `failed` with an `error` code, `panic`, `crash` or `timeout`; files are named `<source>/<path>`, mutants have no `/`).
  - `tools/xtask/src/live/text.rs`: `target/nightly/differential-text/report.json` (`sources`: per source `files`, `pages`, `compared`, `agree`, per-tool tallies, `failures`).
  - `tools/xtask/src/render_diff.rs`: `target/nightly/render-diff/report.json` (`sampling`, `sources` with `rows`, `classes`, `pass_minor_percent`), and its own history records (`kind: "render-diff"`).
  - `tools/xtask/src/verapdf.rs`: only `target/xtask/verapdf/summary.md` (Markdown), from `Verdict`s it already holds.
  - `tools/xtask/src/harness.rs`: `fuzz_one` runs one target with its output inherited, and records nothing.
  - `tools/xtask/src/roundtrip.rs`: `target/xtask/roundtrip/versions.json` (case count, each tool's version) and `manifest.json`; the saved and refused counts are only printed by `crates/laminapdf-save/examples/roundtrip.rs`. The suite runs in the full lane on every push to `main`, not in the nightly.
  - `tools/xtask/src/pdf_runtimes.rs`: `target/xtask/pdf-runtimes/measurements.json`, which holds only the runtimes that passed, without saying which failed.
- `tools/xtask/src/workflows.rs`: the tests that `report` needs every nightly job and `bench` every job that doesn't depend on it; `tools/xtask/src/release.rs`'s `release_lane_should_call_the_nightly_lane_without_recording`, which fails on a recording step without a `RECORD` gate.
- Earlier todos: [X3a](../done/X3a-nightly-lane.md) (the lane and the history), [X3b](../done/X3b-render-diff-nightly.md) (render diff and its basis-point records), [B9f](../done/B9f-live-text-differential.md), [B8b](../done/B8b-conformance-profiles-and-verapdf.md), [B6c](../done/B6c-runtime-matrix.md), [B3c](../done/B3c-save-planner-and-roundtrip.md) (the round-trip suite), [W1e](../done/W1e-progress-snapshot.md) (the snapshot pattern T1b reuses).

## Scope

**In scope**

- **`cargo xtask proof [--from <dir>] [--jobs <json>] [--history <dir> --runner <label> [--record]]`** in a new `tools/xtask/src/proof.rs`. It reads each task's output at the path that task writes, under `--from` (the workspace root by default), and writes one record, `kind: "proof"`, `version: 1`, with snake_case keys like the other history records:
  - `commit`, `date` (the commit's), `ran` (the run's UTC date), `runner`, `quick: false`, `lamina` (the fixed group's version);
  - `full`: true only when the differential and the render diff weren't sampled, the render diff read 20 pages per file, and every target in `harness::FUZZ_TARGETS` ran at least 600 s. It is worked out from the reports themselves, so no workflow expression can get it wrong;
  - `jobs`: each nightly job's result (`success`, `failure`, `cancelled`, `skipped`), from `--jobs` (`toJSON(needs)` in the workflow);
  - `corpus`: per external source (`pdfjs`, `pdfbox`, `pdfium`, `qpdf`) and for `mutants`: `files`, `opened`, `repaired`, `refused` (`failed`), `refused_alone` (refused while at least one tool opened the file), `errors` (`panic`, `crash`, `timeout`), from `lamina.json` and the tools' reports;
  - `differential`: per tool its version, `files`, `timeouts`, `crashes` and the per-fact tallies, and the run's `failures` and `lamina_errors` counts;
  - `text`: per source `files`, `pages`, `compared` and `agree`, and each tool's version;
  - `render`: `dpi`, `pages_per_file`, and per source `rows`, the classes and `pass_minor_bp`;
  - `fuzz`: per target `seconds`, `executions`, `corpus_files` and `crashed`;
  - `verapdf`: the veraPDF version and per validation `file`, `profile`, `compliant`, `rules_passed`, `rules_failed`;
  - `roundtrip`: `cases`, `saved`, `refused`, and per tool (`qpdf`, `pdfjs`, `pdfium`) its version and whether every output passed;
  - `runtimes`: the runtimes whose example suite passed, byte for byte against the reference PDFs, and those that failed.

  A section whose input is missing (its job failed early, was skipped or was cancelled) is `null`, never zeros, and `jobs` says why. The record holds counts, versions and results: no file names and no text from any file. It has no `benches` key, so `nightly::regressions` and `viewer_perf`'s comparison skip it.
- The command writes `target/nightly/proof/proof.json` and a job-summary table. With `--record` it appends the record to `<history>/history.jsonl` through `nightly::append_record`, but only a `full` one; otherwise it prints a notice and appends nothing.
- **What the record needs from each task**, written next to what each already writes:
  - `cargo xtask verapdf`: `target/xtask/verapdf/report.json` from its `Verdict`s and `VERAPDF_VERSION`;
  - `cargo xtask fuzz <target> --time N`: libFuzzer's `-print_final_stats=1` added to `fuzz_run_args`, the output streamed to `target/xtask/fuzz/<target>.log` as `fuzz_logged` already does for `--all`, and `target/xtask/fuzz/<target>.json` with the seconds, `stat::number_of_executed_units`, the corpus folder's file count and whether the run crashed;
  - `cargo xtask roundtrip`: `saved` and `refused` in the example's `manifest.json`, and `target/xtask/roundtrip/summary.json`, written even when a tool fails;
  - `cargo xtask pdf-runtimes`: `target/xtask/pdf-runtimes/runtimes.json` with the runtimes that passed and failed.
- **`nightly.yml`:**
  - `extract` in the `fuzz` matrix, and each matrix job uploads its `target/xtask/fuzz/<target>.json` as `fuzz-summary-<target>`;
  - a `roundtrip` job (`cargo xtask roundtrip`), set up like `full.yml`'s `suites` job (qpdf, the round-trip venv with `tools/roundtrip/requirements.txt`, pdf.js from `tools/roundtrip/`), uploading `target/xtask/roundtrip/*.json` as `roundtrip`; it joins `bench`'s and `report`'s `needs`;
  - a `proof` job that needs `differential`, `fuzz`, `scaling`, `render-diff`, `verapdf`, `roundtrip` and `pdf-runtimes`, runs on `!cancelled()`, downloads each artefact back to the path its job wrote, and runs `cargo xtask proof --jobs "$NEEDS"`. When `RECORD` is true, it also passes `--history "$HISTORY" --runner "macos-arm64/${RUNNER_NAME%-*}" --record` between the `bench-history` fetch and push. It uploads `target/nightly/proof/` as `proof` and joins `report`'s `needs`;
  - the header comment lists both new jobs.
- `cargo xtask nightly` runs `proof` last over the local outputs, never recording.

**Out of scope** (and which todo covers it)

- The snapshot the website reads, the page and its copy → [T1b](../ready/T1b-how-we-test-page.md).
- Extraction scores, generation timings and module sizes → [T1c](../done/T1c-publish-performance-numbers.md).
- Turning any of these figures into a gate: they stay reported, as ADR 0018 tier 11 and C7c's split between nightly reporting and release gates keep them ([C7c](../ready/C7c-performance-gates-and-launch.md)).
- A larger or more representative corpus (SafeDocs was left out in X3a): a later todo, if T1b's page shows the need.

## Tasks

- [ ] Claim: the first commit moves this file to `ongoing/`.
- [ ] Add `extract` to `nightly.yml`'s `fuzz` matrix, and a `workflows.rs` test that the matrix lists exactly `harness::FUZZ_TARGETS`.
- [ ] `verapdf.rs`: write `report.json` beside `summary.md`; a unit test over the `REPORT` fixture string.
- [ ] `harness.rs`: `-print_final_stats=1`, the per-target log and `target/xtask/fuzz/<target>.json`; a unit test that parses `stat::number_of_executed_units` from a captured libFuzzer tail. The full lane's `--all` run writes the same files.
- [ ] `crates/laminapdf-save/examples/roundtrip.rs` and `roundtrip.rs`: the counts in the manifest and `summary.json`.
- [ ] `pdf_runtimes.rs`: `runtimes.json`.
- [ ] `proof.rs`: the `Record` type (serde `Serialize` and `Deserialize`, so T1b reads it with the same type; each field documented as in `progress.rs`), one reader per input, the `full` rule, the summary table, the arguments (`--record` needs `--history`, as `parse_bench_args` checks). Wire it into `main.rs`.
- [ ] `nightly.yml`: the `roundtrip` and `proof` jobs and the fuzz summary uploads; every new write step gated by `env.RECORD == 'true'`.
- [ ] `nightly.rs`: `proof` at the end of `cargo xtask nightly`.
- [ ] Run `cargo xtask nightly --quick` locally and attach the `proof.json` it writes (`full: false`) to the PR.
- [ ] Docs (below).

## Acceptance criteria

- [ ] From fixture reports of every input, `proof.rs`'s tests produce a record whose counts match hand-computed ones. Cases: a file `opened` by Lamina PDF, one `repaired`, one `failed` that a tool opened (`refused_alone`) and one every tool failed, plus a `panic` and a mutant.
- [ ] A missing input gives a `null` section and the job's result in `jobs`, never a zero figure (test).
- [ ] `full` is false for a sampled differential, a sampled render diff, a fuzz target under 600 s or a target missing from the fuzz summaries; `--record` appends nothing then (tests).
- [ ] The record has no `benches` key, and `nightly::regressions` over a history holding one returns no regression (test).
- [ ] `workflows.rs`: `report` needs `proof` and `roundtrip`, `bench` needs `roundtrip`, and the fuzz matrix equals `FUZZ_TARGETS`. `release.rs`'s recording test passes with the new steps.
- [ ] `cargo xtask nightly --quick` ends with a `proof.json` whose sections are all filled, and `full: false`.

## Post-merge checks

- [ ] The first scheduled nightly on `main` uploads a `proof` artefact with every section filled and `full: true`, and appends one `kind: "proof"` record to `history.jsonl` on `bench-data`.
- [ ] A dispatched nightly with short budgets appends no `proof` record.
- [ ] The release lane's next run (`record: false`) runs `roundtrip` and `proof` and pushes nothing to `bench-data`.

## Tests

Unit tests in `proof.rs` over inline fixture JSON (one per input, the way `render_diff.rs` and `live.rs` test their reports); the libFuzzer stats parser in `harness.rs`; `verapdf.rs`'s `report.json`; `workflows.rs` and `release.rs` workflow tests. No new fixture files.

## Docs to update

- `CLAUDE.md`: `cargo xtask proof` in the commands; the nightly lane paragraph (the `roundtrip` and `proof` jobs, `extract` in the fuzz matrix, the proof record in the history).
- [ADR 0018](../../decisions/0018-test-strategy.md#nightly-lane-x3a-2026-10-29)'s nightly implementation note: one paragraph on the proof record, what it holds and that only full runs on `main` append it.
- The header comments of `nightly.yml` and `.github/actions/bench-history/action.yml` (its callers now include `render-diff` and `proof`).
- `fuzz/README.md`: the per-target summary files.
- [Trust and proof §11](../../plans/trust-and-proof.md#11-to-dos): leave T1 unticked until T1b–T1d merge.

## Risks and open questions

- **Artefact paths.** `upload-artifact` strips the common prefix of its paths, so each download must land where the reading task expects it. The `proof` job's first run on a branch dispatch (short budgets, nothing recorded) checks this before the record matters.
- **Failed nights.** A full night that failed still appends a record, with the failed job in `jobs` and its section `null`. T1b decides what the page shows for such a night; the history keeps the truth either way.
- **The Mac's load.** `roundtrip` adds minutes of work to the night. It is in `bench`'s `needs`, so it never runs beside the benchmarks.
- **History size.** A record is a few kilobytes, so about 1.5 MB a year. Every recording job reads the whole history each night, which stays cheap at that size.
- **Workflow files.** If the implementer's GitHub access can't push workflow files, the PR carries `nightly.yml` for the owner to commit (as [W1b](../done/W1b-site-scaffold-and-deploys.md) did).

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
