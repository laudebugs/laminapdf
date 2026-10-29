# 0018 — Test strategy: tiers, lanes, corpora and fuzzing

- Status: Accepted
- Date: 2026-10-28

## Context

The spike's strongest finding was about method: shared fixtures, byte-identical goldens, a corpus whose facts came from an independent tool (pypdf), and a differential fuzz (the same ~4,300 mutated files through two readers) caught bugs that single-implementation tests missed ([research ch. 07 §4](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned)). The architecture research proposes four CI lanes, differential testing against pdf.js, PDFium, qpdf and pypdf, cargo-fuzz in the OSS-Fuzz layout and veraPDF ([report: repository](../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus)). The parser guidelines ask for continuous fuzzing from day one ([ch. 04 §11.3](../research/pdf/04-interactive-security.md#113-guidelines-for-building-a-safe-parser-lamina)).

PR #8 also taught a lesson: a 1,000-page wall-clock test failed about half the time on a loaded machine in a debug build. It became a deterministic check plus an `#[ignore]`d release-mode scaling test (`crates/lamina-cos/tests/reader_cache.rs`).

The test layout rules (unit tests in-file, integration tests on the public API, `fixtures/` by path, goldens never rewritten by a test) are in [code organization §6](../guidelines/code-organization.md#6-tests-and-fixtures); fixture licensing is [ADR 0019](0019-supply-chain-and-licence-hygiene.md).

## Decision

### 1. Tiers

1. **Unit.** `#[cfg(test)] mod tests` in the same file; colocated `*.test.ts`. Fast, no fixtures beyond inline bytes.
2. **Integration.** `crates/*/tests/`, public API only, one file per topic, helpers in `tests/common/`. Read shared inputs from `fixtures/` by path. Random inputs use a fixed seed (the existing `tests/common` `Rng`) and print it on failure.
3. **Golden, byte-identical.** Expected outputs in `fixtures/golden/`. Every writer test that has a golden also runs twice and compares ([ADR 0001](0001-core-language-and-licence.md) determinism). Goldens change only through **`cargo xtask golden --update`** (added in B2; it regenerates every golden from its source and prints the list), and the diff is reviewed like code. A test never writes a golden.
4. **Corpus facts.** `fixtures/corpus/expected.json` holds facts extracted by independent tools, never by lamina: pypdf today (page count, revisions, xref kind, root keys, …), qpdf `--json` and pdf.js added as facts need them. A file where lamina deliberately differs is listed in an exceptions file with the [ADR 0003](0003-reader-tolerance-and-limits.md) rule that explains it.
5. **Property tests.** proptest for invariants: object serialise → parse is identity; for every op, apply then undo leaves the overlay as it was ([ADR 0006](0006-op-log-and-undo.md)); every saved document reopens with the same facts.
6. **Differential.** lamina against **pdf.js, PDFium, qpdf and pypdf** (MuPDF is excluded by [ADR 0001](0001-core-language-and-licence.md)): open/repair/fail outcome and page count first, then outline, text and round-trip as features land. Two sources: the large external corpus (nightly) and spike-style mutants of `fixtures/corpus` (a fixed-seed set of 500 in the full lane, compared with expectations precomputed by `cargo xtask differential --update`; the full set nightly with live tools).
7. **Round-trip.** Open → op → save → reopen in lamina, pdf.js, PDFium and `qpdf --check` (from B3).
8. **Render goldens** (Round C). PNGs per page rendered by PDFium and hayro, compared with perceptual thresholds; updated only by `cargo xtask render-golden --update`.
9. **Fuzzing.** cargo-fuzz targets in `fuzz/` with the layout OSS-Fuzz expects for Rust (`fuzz/Cargo.toml`, `fuzz/fuzz_targets/<name>.rs`), outside the main workspace like `spikes/` because it needs nightly Rust. Targets from B2: `lex`, `parse_object`, `flate_decode`, `open` (open and drive every read path, as `tests/common::exercise` does), `page_tree`; later `content` (B9), `ops_json` and `save_roundtrip` (B3). Invariants: no panic, no hang (`-timeout=10`), memory under `-rss_limit_mb=2048`, and default [`Limits`](0003-reader-tolerance-and-limits.md) respected. Seeds come from `fixtures/`. Every crash becomes a minimised file in `fixtures/regressions/` and a named regression test. Apply to OSS-Fuzz once the repository is public.
10. **Conformance.** veraPDF (Docker image, external process only per [ADR 0019](0019-supply-chain-and-licence-hygiene.md)) over every generated example nightly; from B8 any violation fails.
11. **Performance and scaling.** Release mode only, in `benches/` or `#[ignore]` tests, run nightly. Scaling tests assert ratios that separate linear from quadratic growth (`pages_should_scale_linearly_with_shared_resources`: 4× the pages must take under 9× the time), never absolute durations. Benchmark history is kept; a regression over 10% opens an issue rather than failing the build.

**No wall-clock assertions in the PR lanes.** Hangs are caught by the runner's timeout (nextest `slow-timeout` with `terminate-after`), not by asserting elapsed time. The 30 s budget asserted in `crates/lamina-cos/tests/robustness.rs` moves to nextest configuration in B2.

### 2. CI lanes

| Lane | When | Runs | Budget |
| --- | --- | --- | --- |
| **Fast** | Every PR and push to `main` (exists, `.github/workflows/ci.yml`) | fmt, clippy `-D warnings` (native + wasm32), portability, tiers 1–5, doc tests, wasm build and size budget, TS checks, cargo-deny and npm licences, MSRV | ~10 min |
| **Full** | Every PR and push to `main` (added in B2); a required check. Runs its suites only when the change touches `crates/`, `packages/`, `apps/`, `fixtures/`, `fuzz/`, `tools/` or `Cargo.lock`, otherwise passes in seconds | Fast lane on Linux, macOS and Windows; the 500-mutant differential set; each fuzz target for 2 min; round-trip with qpdf; render goldens and Playwright from Round C | ~30 min |
| **Nightly** | Scheduled on `main` | Differential against live pdf.js, PDFium, qpdf and pypdf over the external corpus and all mutants; fuzz 10 min per target; veraPDF; hayro vs PDFium render diffs; perf and scaling | hours |
| **Release** | Tags | Full + nightly suites on the release commit, packaging checks (publint, attw), clean-install smoke tests in each runtime | — |

The Full lane is never skipped by a workflow-level `paths` filter: GitHub keeps a required check whose workflow didn't run as "Expected — waiting", which would block docs-only PRs. Instead the workflow always runs; a first job computes the changed paths (`git diff --name-only` against the merge base) and a final gate job, the one marked required, succeeds when the suites passed or were not needed. Pushes to `main` always run everything.

A nightly failure opens an issue automatically and blocks the next release, not unrelated PRs.

**Implementation note (X4, full lane).** [`.github/workflows/full.yml`](../../.github/workflows/full.yml) implements the Full row as follows:

- **The filter is inverted.** Instead of listing the code paths above, `cargo xtask changes` (run by the `changes` job against the merge base with the PR's base branch, renames counted as both paths) calls a PR **docs-only** when every changed path is on a short allowlist: `docs/**`, Markdown and `LICENSE*` files at the repository root, `.github/ISSUE_TEMPLATE/**` and `.github/PULL_REQUEST_TEMPLATE.md`. Everything else is code: root config nobody listed (`.cargo/config.toml`, `.gitattributes`, `rust-toolchain.toml`, `.github/actions/**`, workflows, lockfiles, a new root file), and Markdown outside those places, which can be a build input (a crate README used as its rustdoc and doc tests, a docs site's pages). The allowlist is defined once, in `is_docs` in `tools/xtask/src/changes.rs`, with unit tests for every pattern.
- **The gate.** `changes` outputs `code`, exactly `true` or `false` (pushes to `main` are always `true`). Each suite runs under `if: ${{ !cancelled() && needs.changes.outputs.code == 'true' }}`. The `gate` job (`if: always()`) fails when `changes` didn't succeed or `code` is anything else, when any suite failed or was cancelled, or when a suite was skipped on a code change; it is the only check branch protection needs from this workflow. A new suite uses the same condition and joins `gate`'s `needs`. The workflow also runs on `edited`, so retargeting a PR to another base re-classifies it.
- **Fast lane on three OSes.** The full lane runs `cargo xtask check` on GitHub-hosted Ubuntu, macOS and Windows, whichever runner the fast lane picked, so every code PR is checked on all three.
- **Differential.** `cargo xtask differential --full` checks 36 mutants per corpus file (504 for today's 14 files) from their own seed (`FULL_SEED` and `FULL_PER_FILE` in `crates/lamina-model/tests/common/mutants.rs`) against `fixtures/differential/full.json`, under nextest so a hang is killed. A file's mutants don't depend on the other corpus files, so adding one adds only its own. The exceptions are in `full-exceptions.toml`, grouped by cause (`files` lists), each still naming its ADR 0003 rule; a pair listed twice, an empty list or an entry for a file outside the set fails the test. The fast set (four mutants per file) is unchanged.
- **Fuzz.** `cargo xtask fuzz --all --time 120` runs the targets in turn and reports each failure as a crash (new crash inputs saved) or a build/setup problem. Runs start from the corpus last saved on `main`; only pushes to `main` save it, minimised with `cargo xtask fuzz cmin`.
- **Path portability.** Docs-only PRs skip the Windows job, so `cargo xtask check docs` (fast lane, every PR) rejects tracked paths Windows or a case-insensitive file system can't check out.
- **No retries.** Both PR lanes set `NEXTEST_PROFILE=ci`: `retries = 0` and the same slow-timeouts as the default profile (§4).
- **Timing** (measured on PR #30): `changes` about 20 s on a cold cache, so a docs-only `gate` finishes in under a minute after the job starts; a code PR's full lane took 18 minutes, fuzzing 13 of them.

### 3. Corpora: licensing and provenance

- `fixtures/` holds small files we may redistribute, each listed in `fixtures/README.md` with origin, licence and what it exercises ([ADR 0019](0019-supply-chain-and-licence-hygiene.md)). Generated and mutated files are ours (`MIT OR Apache-2.0`). Nothing with personal data.
- Large external corpora (pdf.js, PDFium and qpdf test suites, SafeDocs, veraPDF and Arlington test files) stay out of git: `cargo xtask corpus fetch` downloads them into a gitignored folder, pinned by SHA-256 in a manifest that also records each source's licence. CI caches them; the R2-hosted copy (report) is a mirror, not a source of truth.
- A corpus file that a licence forbids redistributing is never committed, even minimised; regressions from it are re-created synthetically.

### 4. Flaky tests

A flaky test is a bug. Within a day of noticing it, mark it `#[ignore = "flaky: #<issue>"]` (or `it.skip` with the issue) and assign the issue; fix or delete it within two weeks. PR lanes never retry failures (nextest `retries = 0`). The nightly lane may retry once only to *report* flakiness, and a test that passes on retry is still reported. Tests must not depend on time, ordering, the network or unseeded randomness.

### 5. Shared expectations across languages

Fixtures, `expected.json`, error and warning codes (`fixtures/codes.json`, [ADR 0003](0003-reader-tolerance-and-limits.md)) and op samples (`fixtures/ops/`, [ADR 0006](0006-op-log-and-undo.md)) are read by the Rust tests and, from B6, by the TypeScript tests, so the bindings can't drift.

## Consequences

- PR feedback stays fast and deterministic; the expensive and environment-sensitive work runs nightly where a failure doesn't block unrelated work.
- External tools (pdf.js, PDFium, qpdf, pypdf, veraPDF) are needed only in full and nightly lanes; contributors don't need them for the fast lane.
- Goldens, differential expectations and render goldens each have one explicit update command; reviewers see every change.
- Maintaining the corpus manifest, the exceptions file and the fuzz regressions is ongoing work.
- Performance regressions are seen within a day, not at the PR that caused them.

## Alternatives considered

- **Wall-clock budgets in PR tests.** Flaky on shared runners (PR #8); ratios in release mode measure the same thing reliably.
- **Snapshot tools for PDF bytes** (`insta`). Fine for text, but PDF goldens need byte review and a regenerate step tied to their source.
- **Expected facts produced by lamina.** Circular: a bug would confirm itself.
- **Differential against external tools in every PR.** Slow and installs four toolchains; the precomputed mutant set catches most regressions.
- **Committing the large corpora.** Licences forbid it for much of them, and the repository would grow by gigabytes.
- **Retrying flaky tests automatically.** Hides real races.

## Implementation notes

### Nightly lane (X3a, 2026-10-29)

- **Workflow.** [`.github/workflows/nightly.yml`](../../.github/workflows/nightly.yml) runs on a cron on `main`, on `workflow_dispatch` and on `workflow_call`, so the release lane (B7b) can run the same suites. Its inputs are the fuzz seconds, the differential sample and `force-failure`. It replaced B2c's `fuzz.yml`. Each job is a `cargo xtask` task, and `cargo xtask nightly [--quick]` runs them all locally. A failed job, or a scaling test that passes only on retry, opens or comments on the issue "Nightly lane failing on <ref>" (label `nightly`). A benchmark regression opens "Nightly benchmark regression on <ref>" and never fails the run. Naming the ref keeps branch and tag runs out of main's issue. The differential and the benchmarks run on GitHub-hosted Ubuntu: the differential installs pinned Linux qpdf binaries, and benchmark history only compares within one machine class. Fuzz and scaling use the self-hosted runner when one is online.
- **Corpora (§3).** `fixtures/corpus-manifest.toml` lists pdf.js `test/pdfs`, PDFBox's PDFs, PDFium `testing/resources` and qpdf's `qtest` PDFs at pinned commits. It records each source's licence and every file's size and SHA-256 (2,172 files, 177 MB).
  - `cargo xtask corpus fetch` makes sparse, shallow, blob-filtered git checkouts. It always resets the remote to the manifest's URL. A checkout that no longer matches the manifest is restored from git (`reset --hard`, `clean`) before the run fails. A source that passed is stamped, so the differential only re-checks file sizes.
  - An optional mirror (`mirror` or `$LAMINA_CORPUS_MIRROR`, percent-encoded paths) is used only when git fails. A later fetch replaces the mirror download with a git checkout.
  - CI caches the checkouts keyed on the manifest hash.
  - Not yet included: SafeDocs (multi-GB archives with mixed provenance), veraPDF and Arlington test files (with B8b), and PDFium's `.in` templates (they need PDFium's generator script).
- **Differential (tier 6).** `cargo xtask differential --live` runs lamina, pdf.js (`pdfjs-dist`), PDFium (`pypdfium2`), qpdf (`--json=2`) and pypdf over the external corpus and every `fixtures/corpus` mutant both PR lanes know: 250 per file from the fast set's series (whose first 4 are the fast set) and the full lane's 36. It compares the outcome, the page count and the outline titles. pdf.js and PDFium repair silently, so against them only readable vs failed counts. `--sample` is stratified per source.
  - **Isolation.** All readers run at once. Each works through chunks of inputs, one process per chunk under a time limit. A chunk that crashes, hangs or reports different files is rerun one file per process, so a bad file becomes a `timeout` or `crash` entry instead of stopping the run (`tools/xtask/src/runner.rs`).
  - **What fails a run.** A fact fails when every tool that compared it disagrees with lamina and no exception excuses it. A run also fails when lamina panics, crashes or hangs on a file, or when a reader breaks or reports the wrong files. The report is written either way.
  - **Splits.** A disagreement where some tool agrees with lamina is a split, listed in the report but not a failure: the tools differ from each other at least as much. The first full run had 436 failures and 1,890 splits before triage.
  - **Exceptions.** On top of the per-file `[[exception]]` entries of both exceptions files, `exceptions.toml` gains value-matched `[[class]]` rules and `[[known]]` entries. Each class names its ADR 0003 rule (for example "lamina repaired, the tool failed": §1 Repair). Each known entry is a bug or open question tracked by an issue, like an ignored flaky test (§4). The PR lanes apply an entry only if its `tools` include pypdf.
  - Text (B9c) and round-trip (B3c) join the same comparison when they land.
- **Fuzzing (tier 9).** Each target runs 10 minutes with `-timeout=10 -rss_limit_mb=2048`. The run starts from the target's last nightly corpus, or from the full lane's corpus saved on `main`. Only runs on `main` save it back: one entry per target, minimised first with `cargo xtask fuzz cmin`. Crashes are minimised under the same limits (`cargo xtask fuzz tmin`, skipping `slow-unit` files), uploaded, and quoted in the issue (base64 up to 4 KiB).
- **Scaling and benchmarks (tier 11).** `cargo xtask scaling` runs `cargo test --release -- --ignored` single-threaded and retries a failure once. A pass on retry is reported as flaky, never hidden.
  - `cargo xtask bench` runs the criterion benches in `crates/lamina-cos/benches/`: open + page-tree walk, Flate decode and `Writer` serialisation; save benches come with B3b.
  - Each benchmark is compared with the median of its last five full (not `--quick`) records on the same runner. A benchmark over 10% slower is a regression.
  - The night's record is appended to `history.jsonl` on the `bench-data` branch. The token for that branch is passed per git command and only to the fetch and push steps, never to the step that builds and runs the benchmarks.
