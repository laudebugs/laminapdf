# T1c — Publish extraction scores and speed, and generation timings

| Field | Value |
| --- | --- |
| ID | `T1c` |
| Status | `ready` |
| Plan | [trust and proof §T1](../../plans/trust-and-proof.md#t1-publish-the-proof) |
| Round / milestone | Round T (trust and proof) |
| Depends on | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md#1-tiers) tier 11 (timings are reported, never asserted), [0016 §6](../../decisions/0016-package-design.md#6-budgets-per-module) (the module sizes and their budgets), [0030 §7](../../decisions/0030-extraction-output-and-model-plugins.md#7-versioning-and-determinism) (extraction output); [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md#fonts-icc-profiles-and-test-corpora) for the benchmark data's licence |
| Branch | `feat/t1c-publish-performance-numbers` |
| PR title | `feat(docs): a benchmarks page with extraction scores and speed, generation timings and module sizes` |
| Size | L (≈ 450–600 lines: the snapshot command, a machine helper, two new timing cases, the page's data reader and tables). If it grows past 600, split the generation half (timings and sizes) into its own todo |
| PR | — |

## Goal

Put Lamina PDF's measured numbers on one docs page, each with its method, machine, version and date:

- how well extraction does on olmOCR-bench, and how many pages a second it reads;
- how long it takes to make a first PDF, a one-page invoice and a 100-page report in each runtime and in Rust;
- how large the WebAssembly modules are.

The numbers come from a committed data file that one command writes from the benchmark runs, never from hand-copied figures ([trust and proof §3](../../plans/trust-and-proof.md#3-rules-for-this-round), rule 1).

On Hacker News, extraction users ask for benchmarks they can rerun: 163 comments, a rate that rose from 3 to 41 per 1,000 extraction comments, and 265 comments on throughput. Generation users quote speed (about 350 comments, rising to 67.5 per 1,000) and edge bundle limits: "on cloudflare workers, the bundle limit makes 226KB massive." ([46336747](https://news.ycombinator.com/item?id=46336747)). A Typst user reports "I need to generate a 2 page invoice and i can generate it under 100 ms." ([45697377](https://news.ycombinator.com/item?id=45697377)). Lamina PDF measures all of this already, but publishes only the olmOCR-bench scores, in a crate README ([report R4](../../research/hacker-news/pdf-users-on-hacker-news.md#r4-make-extraction-checkable), [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json)).

## Context

- [Report R4 and R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r4-make-extraction-checkable); the [extraction note](../../research/hacker-news/notes/extraction.md) (recommendations 1 and 8: publish pages per second and olmOCR-bench scores) and the [generation note](../../research/hacker-news/notes/generation.md) (§1.5 speed and scale, recommendation 2: milliseconds per invoice, module size, cold start).
- [L3f](../done/L3f-extraction-benchmark.md) and `tools/xtask/src/extract_bench.rs`: `cargo xtask extract-bench` fetches olmOCR-bench's three born-digital subsets (`tools/bench/extract-datasets.toml`, SHA-256 per file in the lock), runs `crates/laminapdf/examples/differential/extract-bench.rs` under the differential runner, scores with `tools/bench/olmocr_score.py`, and writes `target/bench/extract/<dataset>/<candidate>-score.json` and `-report.md`. The example already times each file (`millis`: open, extract and Markdown); `render_report` prints only the total. Every benchmark file is one page.
- The scores today: [`crates/laminapdf-extract/README.md`](../../../crates/laminapdf-extract/README.md), "How it scores on a public benchmark", typed in by hand from one run (0.2.0-alpha.2, 2026-11-05), with no machine and no speed.
- `tools/xtask/src/pdf_runtimes.rs` and `packages/sdk/test/suite/examples.ts`'s `measure()`: per runtime `hello-first` (the first PDF after `init()`), `hello` (median of 20), `report` (the 100-page report, median of 3) and `wasm-memory`, written to `target/xtask/pdf-runtimes/measurements.json`, against pdf-library §4's targets (5 ms, 500 ms, 64 MB).
- `crates/laminapdf/benches/report.rs` ([B5d](../done/B5d-text-report-generation-performance.md)): `report_100_pages`, checked against the TypeScript suite's `REPORT_SHA256`, and `report_100_pages_text_boxes`; `cargo xtask bench` writes `target/nightly/bench/results.json`. B5d's notes record how noisy the shared CI Mac is (load average 170–250 on 8 cores).
- `cargo xtask size` (`tools/xtask/src/wasm.rs`): raw, gzip and brotli per module and the JS, in `target/size-report.json`, against `packages/wasm/size-budgets.json`.
- The invoice: `packages/sdk/examples/invoice.ts` and `crates/laminapdf/examples/invoice.rs`, one tagged A4 page with an embedded Noto Sans subset and a table, and its reference PDF `fixtures/golden/examples/invoice.pdf`. `crates/laminapdf/tests/examples.rs` passes its fixed options.
- The docs site: `apps/docs/plugins/sidebar.mjs` (every page listed, in story order), `src/lib/codes.ts` and `src/components/CodeTable.astro` (repository data read at build time), `plugins/markdown.mjs` (each data component needs its Markdown twin), the [docs-writing guideline](../../guidelines/docs-writing.md) (no "CI" or "golden" in reader text) and `tools/xtask/docs-prose.toml`.
- Pages that link here rather than repeat: [U5a](./U5a-pricing-and-promises-page.md) (the reader's speed targets), [U5b](./U5b-reader-benchmark.md) (the reader against other readers, at `laminapdf.com/performance/`), [T1b](./T1b-how-we-test-page.md) (the robustness figures). [C7c](./C7c-performance-gates-and-launch.md) owns turning timings into gates.

## Scope

**In scope**

- **Speed in `extract-bench`.** The command also writes `target/bench/extract/<dataset>/<candidate>-results.json` with the dataset, its revision and licence, the per-subset tests, passes and scores, and the overall score. It adds the pages extracted, failed and panicked, the summed and median milliseconds per page, and the pages per second on one thread, with the Lamina PDF version, commit, date and machine. `render_report` prints the pages per second too.
- **A machine description**, new in `tools/xtask/src/machine.rs`: OS and version, CPU model, core count, memory and the load average at the start of the run. It reads `sysctl` and `sw_vers` on macOS and `/proc/cpuinfo`, `/proc/meminfo` and `uname` on Linux; an unknown value is `null`.
- **An invoice timing.** `measure()` gains an `invoice` case: the median of 20 builds of the invoice example with Noto Sans (already in `REPORT_INPUTS`). It is recorded in the nightly history as `pdf/<runtime>/invoice` and gets a column in `pdf_runtimes.rs`'s table. `benches/report.rs` gains `invoice_one_page`, the Rust twin, checked against `fixtures/golden/examples/invoice.pdf` with the options `tests/examples.rs` passes, so both measure the reference PDF's bytes. No new two-page invoice: the one-page invoice and the 100-page report bracket it and are tested documents already (see Risks).
- **`cargo xtask benchmarks [--no-run]`**, new in `tools/xtask/src/benchmarks.rs`. It runs `size` (after `wasm`), `pdf-runtimes` (every runtime) and `bench` unless `--no-run`, then writes `apps/docs/src/data/benchmarks.json` (`version: 1`). Each section carries its own commit, date, Lamina PDF version and machine:
  - `generation`: per runtime the four cases and the invoice, with each runtime's version (Node on `PATH`; Deno, Bun, Playwright and wrangler from the pnpm catalog); for Rust, criterion's estimates for `report_100_pages` and `invoice_one_page`;
  - `sizes`: gzip and brotli bytes per module and for the JS;
  - `extraction`: copied from the newest `*-results.json` of the default candidate (`laminapdf`), and the `laminapdf-stream` candidate when present. A missing file keeps the committed section, with a warning.
- **The page**: `apps/docs/src/content/docs/reference/benchmarks.mdx`, "Benchmarks", in the sidebar's Reference group after "Stability policy". Its tables come from a `Benchmarks` component over `src/lib/benchmarks.ts`, which checks the file's shape and fails the build on a file it doesn't describe. The page explains each figure in plain English: what was timed, from where to where, how many runs and which statistic. It says what the figures leave out: one thread, single-page benchmark files, scanned pages that give no text, a machine that also runs other jobs if it does. The page's Markdown twin shows the same tables (a case in `plugins/markdown.mjs`).
- **One home for the scores.** The README's benchmark section keeps its explanation of the failures and links to the docs page for the table, so the numbers live in one place.
- Links to the page from the report recipe (`recipes/report.mdx`) and from the extraction recipe (`recipes/extract-markdown.mdx`).

**Out of scope** (and which todo covers it)

- Timings as release gates → [C7c](./C7c-performance-gates-and-launch.md); the reader's speed against other readers → [U5b](./U5b-reader-benchmark.md).
- The robustness figures (corpus, agreement, fuzzing) → [T1a](./T1a-nightly-proof-figures.md), [T1b](./T1b-how-we-test-page.md).
- Refreshing the numbers on a schedule. They change with releases, and `docs.laminapdf.com` builds from each release ([P4](../done/P4-docs-match-the-release.md)), so one refresh before a release is enough.
- A new benchmark dataset, or tables scored with TEDS: L3f's manifest says why none was usable.
- Flowing-layout timings: they follow T4b–T4e (T4b's flowing-report bench, T4e's two-page `invoice-from-json` case in `measure()`).

## Tasks

- [ ] Claim: the first commit moves this file to `ongoing/`.
- [ ] `machine.rs` with unit tests over captured `sysctl` and `/proc` text.
- [ ] `extract_bench.rs`: `*-results.json` and the pages per second in the report; a unit test over a fixture `facts` object (median, sum, pages per second).
- [ ] The `invoice` case in `measure()`, `history_entries` and the table in `pdf_runtimes.rs`; `invoice_one_page` in `benches/report.rs`.
- [ ] `benchmarks.rs`: run, read, write; `--no-run`; unit tests over fixture outputs; wire it into `main.rs`.
- [ ] On the quietest machine available (the owner's Mac when the runners are idle, or another Apple-silicon Mac), run `cargo xtask wasm`, `cargo xtask extract-bench`, then `cargo xtask benchmarks`; commit the first `benchmarks.json`.
- [ ] `src/lib/benchmarks.ts`, the `Benchmarks` component and its Markdown twin case; the page, drafted with `/writing`; the sidebar entry; the links from the two recipes.
- [ ] Move the README's table out; check that the docs page and the README say the same about why tests fail.
- [ ] `/writing-review` on the page.

## Acceptance criteria

- [ ] Every number on the page comes from `benchmarks.json`. A test of `src/lib/benchmarks.ts` rejects a file with a missing field and accepts the committed one; the built page shows each runtime's invoice and report timings, each module's gzip size and each subset's score.
- [ ] Each section on the page states its machine, Lamina PDF version, commit (short) and date.
- [ ] `extract-bench` reports pages per second, and `benchmarks.json` holds it. For the committed run, the pages per second equal the pages extracted divided by the summed seconds (unit test on the arithmetic).
- [ ] `invoice_one_page` builds the bytes of `fixtures/golden/examples/invoice.pdf`, and every runtime's `invoice` case builds the invoice example; `cargo xtask pdf-runtimes` prints the invoice column.
- [ ] The page passes `cargo xtask check docs-prose` strict, and its Markdown twin holds the same tables.
- [ ] No number on the page or in the README is typed by hand.

## Post-merge checks

- [ ] The next nightly records `pdf/<runtime>/invoice` for every runtime and `invoice_one_page` in the benchmark history.
- [ ] The docs deploy built from `main` (or the next release, once P4 has landed) shows the page.

## Tests

Unit tests in `machine.rs`, `extract_bench.rs` and `benchmarks.rs` over captured text and fixture JSON; the TypeScript suite's `invoice` case runs in every runtime through `cargo xtask pdf-runtimes`; `invoice_one_page`'s bytes check against the reference PDF; the docs build, with the reader's checks and the links validator.

## Docs to update

- `CLAUDE.md`: `cargo xtask benchmarks` in the commands; `extract-bench`'s results file; the docs site's new page and data file.
- `crates/laminapdf-extract/README.md` (the table moves to the docs page), `tools/bench/README.md` (the results file), `apps/docs/README.md` (the data file and how to refresh it).
- [Git workflow §2](../../guidelines/git-workflow.md#pick-work-from-the-todo-board): a refresh of `benchmarks.json` is an exempt PR, like the progress snapshot's.
- [pdf-library plan](../../plans/pdf-library.md) §4: link the measured numbers to the page.

## Risks and open questions

- **A noisy machine.** The CI Mac runs other jobs; B5d measured under a load average of 170–250. Run the refresh on an idle machine, record its load average in the file's machine section, and say on the page that the numbers are one machine's.
- **No two-page invoice.** HN's figure is for a two-page invoice made by a Typst binary, start-up included, so no in-process timing compares directly. The page states what each Lamina PDF figure includes: `init()` once, then the build. Adding a two-page example would add an example pair, a reference PDF and a recipe for little new information. If the owner wants the exact case, it is a follow-up todo.
- **Cold start.** `hello-first` measures the first PDF after `init()`, not `init()` itself. Timing `init()` needs each runner to read the clock around its own `init` call (workerd's clock moves only across I/O). Leave it out unless every runner can do it without changing the suite's order, and say on the page what `hello-first` covers.
- **Benchmark files are single pages.** Pages per second on olmOCR-bench includes opening each file. The page says so, and names the per-page median beside the rate.
- **Ageing numbers.** Each section shows its version and date, and P4's per-release docs keep old numbers with old versions.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
