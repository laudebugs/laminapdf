# C1a — `hayro-parity` spike: hayro vs PDFium pass rate on the pdf.js and PDFBox corpora

| Field | Value |
| --- | --- |
| ID | `C1a` |
| Status | `done` |
| Plan | [web reader §6 C1. Rendering spikes](../../plans/web-app.md#c1-rendering-spikes) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | None |
| Blocks | [C1b](./C1b-pdfium-wasm-spike.md), [C2a](../ongoing/C2a-render-adrs.md) |
| Dependency notes | None (needs only the merged Round A workspace; it does not load any lamina crate, so it can start before B6 merges) |
| ADRs | Informs ADR 0004 (`RenderBackend`; PDFium default; hayro oracle; MuPDF prohibited). Must follow [0001](../../decisions/0001-core-language-and-licence.md) (no AGPL/GPL, so no MuPDF even as a comparison), [0018](../../decisions/0018-test-strategy.md) §3 (corpora stay out of git, pinned by SHA-256), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `spike/c1a-hayro-parity` |
| PR title | `spike(hayro-parity): measure hayro vs pdfium render parity on the pdf.js and pdfbox corpora` |
| Size | M (200–600 lines of spike code + README; corpora are fetched, not committed) |
| PR | [#24](https://github.com/laudebugs/lamina/pull/24) |

## Goal

Answer the [architecture §4](../../plans/architecture.md#4-spikes-to-close-open-questions) question **"how close is hayro to PDFium today?"** with numbers. Render every page of the pdf.js and PDFBox test corpora with hayro (pure Rust, MIT/Apache) and with PDFium (native, via `pdfium-render`), compare them with a perceptual metric, and report the pass rate, the failure classes and the speed. The result is the baseline ADR 0004 records for "PDFium is the default, hayro is the CI oracle and the future default", and it defines the threshold the nightly hayro-vs-PDFium diff ([X3b](../ready/X3b-render-diff-nightly.md), on top of C2b's `lamina-render`) will track. Because it touches no lamina crate, this half of C1 can run before B6 merges.

## Context

- [Architecture research report: "PDFium ships now, hayro is the destination, and a trait decides"](../../research/architecture/app-and-package-architecture.md#pdfium-ships-now-hayro-is-the-destination-and-a-trait-decides) — why PDFium is the default and why a corpus-diff harness is the instrument for switching. No neutral fidelity benchmark exists; this spike creates lamina's.
- [Rendering notes](../../research/architecture/notes/rendering.md) §1 (engine versions and licences: hayro 0.8.0, `pdfium-render` 0.9.4, `@embedpdf/pdfium` 2.15.1) and §5 (risks: hayro has no glyph cache, no published pass/fail table, unknown encryption/ICC support).
- [Architecture plan §2 principle 4](../../plans/architecture.md#principles-each-becomes-part-of-an-adr) (rendering is swappable) and [§4 spikes](../../plans/architecture.md#4-spikes-to-close-open-questions).
- [ADR 0018 §1 tier 8](../../decisions/0018-test-strategy.md#1-tiers) (render goldens with perceptual thresholds) and [§3](../../decisions/0018-test-strategy.md#3-corpora-licensing-and-provenance) (external corpora: fetched, pinned by SHA-256, licence recorded, never committed).
- [`spikes/krilla-fit/README.md`](../../../spikes/krilla-fit/README.md) — the house style for a spike: standalone crate outside both workspaces, `cargo run --release` prints the measurements, README answers the question and recommends.
- [Code organization §12](../../guidelines/code-organization.md#12-spikes-vs-product-code) — spikes are throwaway, outside every workspace, never imported.

## Scope

**In scope**

- `spikes/hayro-parity/`: a standalone Rust binary (own `Cargo.toml`, own `Cargo.lock`, not a workspace member) that renders each page with hayro and with PDFium at a fixed DPI and compares them.
- A fetch script for the pdf.js (`test/pdfs`, Apache-2.0) and PDFBox (Apache-2.0) corpora into a gitignored folder, pinned by commit and SHA-256 manifest with licences.
- Perceptual comparison (e.g. per-page SSIM or a `dssim`/`image-compare`-style metric, plus % of pixels over a ΔE threshold) and a classification of each page: `pass`, `minor`, `fail`, `hayro-error`, `pdfium-error`, `both-error`, `timeout`.
- Timing per page for both renderers (release build, same machine) and peak RSS.
- A written result in `spikes/hayro-parity/README.md` with a recommendation for ADR 0004.

**Out of scope** (and which todo covers it)

- PDFium-WASM size, memory and reload-after-incremental-save measurements in the browser → [C1b](../done/C1b-pdfium-wasm-spike.md).
- The product `RenderBackend` trait and adapters, render goldens on `fixtures/` → [C2b](../ready/C2b-lamina-render-crate.md); the nightly diff job → [X3b](../ready/X3b-render-diff-nightly.md).
- Writing ADR 0004 itself → [C2a](../ongoing/C2a-render-adrs.md).
- Any MuPDF comparison (AGPL, excluded by ADR 0001).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then create a fresh worktree off the updated `main`: `git worktree add -b spike/c1a-hayro-parity spike-c1a-hayro-parity main`.
- [x] Check licences before adding anything: `hayro` (MIT OR Apache-2.0), `pdfium-render` (confirm MIT OR Apache-2.0 on crates.io; the rendering notes flag it as unconfirmed), the PDFium binary source (e.g. `bblanchon/pdfium-binaries`, Apache-2.0/BSD) and the comparison crate. Record each in the README.
- [x] Create `spikes/hayro-parity/` with `Cargo.toml` (edition 2024, `publish = false`, empty `[workspace]` table so it is not pulled into the root workspace), `src/main.rs`, `scripts/fetch-corpora.sh`, `scripts/fetch-pdfium.sh`, `corpora.manifest.toml` (source URL, pinned commit, licence, SHA-256 per file), `.gitignore` (`corpora/`, `out/`, `pdfium/`).
- [x] `fetch-pdfium.sh`: download a pinned PDFium native build for the host OS and verify its SHA-256; `pdfium-render` binds it dynamically.
- [x] `fetch-corpora.sh`: shallow-fetch pdf.js `test/pdfs` (only the files that are present in the repo, not the `*.link` remote ones, unless they are fetched and hashed too) and the PDFBox test PDFs at pinned commits; write the manifest; skip and list files whose licence forbids redistribution (they are never committed anyway).
- [x] Renderer wrappers in `src/render.rs`: `render_hayro(bytes, page, dpi) -> Result<RgbaImage, Error>` and `render_pdfium(bytes, page, dpi) -> Result<RgbaImage, Error>`, both rendering the page's CropBox at 96 DPI and 150 DPI, white background, annotations on (PDFium `FPDF_ANNOT`), no anti-alias differences configured away silently (document the settings).
- [x] Per-page timeout and isolation: render each document in a child process (re-exec the binary with `--one <file>`) so a hayro or PDFium crash or hang (timeout 30 s per page) is recorded as `*-error`/`timeout` instead of killing the run.
- [x] Comparison in `src/compare.rs`: size mismatch → `fail`; otherwise compute SSIM (greyscale) and the share of pixels whose colour difference exceeds a threshold. Start with thresholds `pass`: SSIM ≥ 0.98 and ≤ 0.5 % differing pixels; `minor`: SSIM ≥ 0.95; else `fail`. Report the sensitivity (pass rate at 3 threshold settings) so ADR 0004 can choose.
- [x] Output: `out/dpi96/results.csv` (file, page, class, ssim, diff %, hayro ms, pdfium ms, error), `out/dpi96/summary.md` (totals, per-corpus pass rate, top failure classes), and for every `fail` a side-by-side PNG triplet (hayro, PDFium, diff) for manual triage of the first 50.
- [x] Manually triage at least 50 failing pages into buckets (fonts/Type3, shading, blend modes/transparency groups, JBIG2/JPX images, annotations, encryption, ICC/colour, broken files) and record counts.
- [x] Record the machine (CPU, RAM, OS, rustc version) and the exact versions/commits of hayro, `pdfium-render`, PDFium and both corpora. C1b must report on the same machine.
- [x] Write `spikes/hayro-parity/README.md`: question, method, how to reproduce (`scripts/fetch-pdfium.sh && scripts/fetch-corpora.sh && cargo run --release`), results table, failure buckets, speed comparison, limitations, and a **Recommendation for ADR 0004** (default renderer per surface, the parity threshold that would justify switching the default to hayro, which failure buckets must be zero first, the nightly metric X3b tracks).
- [x] Add the spike to the `CLAUDE.md` layout tree and the spikes notes (one line each, like `krilla-fit`), and to [architecture §4](../../plans/architecture.md#4-spikes-to-close-open-questions) as "Done" with a link to the result.

## Acceptance criteria

- [x] `cargo run --release` in `spikes/hayro-parity/` (after the two fetch scripts) renders every page of both pinned corpora with both renderers and writes `out/dpi96/results.csv` and `out/dpi96/summary.md` (one folder per run: `out/dpi<N>[-<engine>-only]/`) without aborting on any single bad file.
- [x] The README reports, for each corpus and in total: number of files and pages, pass/minor/fail/error/timeout counts and percentages, median and p95 render time per page for each renderer, and peak RSS.
- [x] At least 50 failing pages are triaged into named buckets with counts.
- [x] The README states the machine, all versions and commit SHAs, and the thresholds used, so the run is reproducible.
- [x] The README ends with a recommendation section that C2a can copy into ADR 0004 (default per surface, switch criteria, nightly metric).
- [x] No corpus file, PDFium binary or rendered PNG is committed (checked with `git ls-files spikes/hayro-parity`).
- [x] No new dependency in the root workspace or `pnpm-lock.yaml`; `cargo xtask check` is unaffected.

## Outcome

Result in [`spikes/hayro-parity/README.md`](../../../spikes/hayro-parity/README.md). On 2,196 pages (1,188 documents, 96 DPI), hayro matches PDFium on 81.8 % of pages (pass+minor 90.8 %; 150 DPI: 78.3 % / 90.4 %). Where hayro is clearly wrong, it is mostly fonts, annotations without appearance streams and transparency groups; CalRGB/CalGray colour differs a lot but needs a reference rendering to judge. Recommendation for ADR 0004: PDFium is the default everywhere and hayro is the CI oracle. X3b tracks pass+minor at 96 DPI and sets its own baseline on the CI runner (the macOS 90.8 % is only indicative). Deviations from this todo:

- **Thresholds:** the starting point (SSIM ≥ 0.98) failed visually identical text pages, so the metric gained a 3×3 blur and a 3×3 anti-aliasing neighbourhood, and the thresholds were recalibrated by hand (pass ≤ 0.5 % differing pixels and SSIM ≥ 0.90; minor ≤ 2 % and SSIM ≥ 0.85). The README explains this and reports the sensitivity.
- **pdf.js `*.link` files** (461) were not fetched; only in-repository PDFs are used.
- **Triage** covers all 153 `fail` pages but one 3-page PDFBox form (150 pages in 122 documents).
- **Output paths:** each run writes to its own folder, `out/dpi<N>/` (or `out/dpi<N>-<engine>-only/`), instead of `out/results.csv` and `out/summary.md`, so the 96 DPI, 150 DPI and single-engine runs don't overwrite each other.
- **Results** live in `out/` (gitignored); the four run summaries and the triage table are committed in `spikes/hayro-parity/results/`.
- **PR title:** `spike(c1a): hayro vs PDFium parity on the pdf.js and PDFBox corpora`.

## Post-merge checks

None.

## Tests

Spike code is throwaway, so no product tests. Add a smoke check in the README's reproduce steps: rendering `fixtures/corpus/hello.pdf` with both renderers must classify `pass`; rendering a deliberately broken file (`fixtures/corpus/hello-broken.pdf`) must not crash the harness. Not wired into CI (like `krilla-fit`).

## Docs to update

- `spikes/hayro-parity/README.md` (the result).
- `CLAUDE.md` layout tree and commands (one line for the spike).
- [architecture.md §4](../../plans/architecture.md#4-spikes-to-close-open-questions): mark `hayro-parity` done with a result link; tick the C1 to-do "Build `spikes/hayro-parity`…" in [web-app.md §9](../../plans/web-app.md#9-to-dos).
- `docs/todos/README.md` board (move this todo).

## Risks and open questions

- **pdf.js corpus has many `.link` files** pointing to remote PDFs (some gone). Decide during the spike whether to fetch them (more coverage, more churn) or use only in-repo files; record the choice.
- **Anti-aliasing and font hinting differ** between renderers, so pixel diffs over-report; the perceptual threshold must be tuned on a hand-checked sample and the sensitivity reported, not hidden.
- **hayro and encrypted files**: support in 0.8.0 is unverified; classify them separately so encryption gaps don't masquerade as fidelity gaps.
- **Corpus licences**: both are Apache-2.0 repos, but individual PDFs may carry their own terms; nothing is committed, so this only affects whether minimised regressions can later go into `fixtures/` (ADR 0018 §3: re-create synthetically if not).
- **Runtime**: the corpora are thousands of pages; parallelise across processes and keep a `--limit`/`--sample` flag for quick runs.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
