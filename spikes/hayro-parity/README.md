# Spike: `hayro-parity` (Round C1a)

- **Date:** 2026-10-28
- **Status:** done. The recommendation below is input to ADR 0004 ([C2a](../../docs/todos/ongoing/C2a-render-adrs.md)); it decides nothing on its own.
- **Plan:** [architecture §4](../../docs/plans/architecture.md#4-spikes-to-close-open-questions) (`hayro-parity`), [web-app plan §6 C1](../../docs/plans/web-app.md#c1-rendering-spikes)
- **Todo:** [C1a](../../docs/todos/done/C1a-hayro-parity-spike.md)
- **Code:** throwaway-quality, outside both workspaces, never imported by product code ([code organisation §12](../../docs/guidelines/code-organization.md#12-spikes-vs-product-code)).

## Question

**How close is hayro to PDFium today?** Render every page of the pdf.js and PDFBox test corpora with hayro (pure Rust) and with PDFium (native, through `pdfium-render`), compare them with a perceptual metric, and report the pass rate, the failure classes, the speed and the memory. This is the baseline for "PDFium is the default, hayro is the CI oracle and the future default" (architecture principle 4), and it defines the metric the nightly diff ([X3b](../../docs/todos/ready/X3b-render-diff-nightly.md)) tracks.

## Answer in one paragraph

**hayro matches PDFium on 82 % of pages (91 % counting small differences) and opens almost everything PDFium opens, but it is not ready to be the default renderer.** On 2,196 pages from 1,188 documents at 96 DPI, 1,797 pages **pass** (no visible difference beyond anti-aliasing), 196 are **minor** (small shifts, ≤ 2 % of pixels), 153 **fail**, and 50 are errors or timeouts. Errors are rare, and PDFium has more of them on its own: it fails alone on 14 pages (mostly malformed files that hayro opens, plus one crash), hayro alone on 5 (one of them a panic), both on 26 (mostly password-protected or broken files). Timeouts are 2 for hayro and 3 for PDFium. The real failures are concentrated in a few areas. Where it is clear which engine matches the file's intent (39 of the triaged pages), hayro is the one that looks wrong in 36. The areas are: **fonts** (embedded fonts decoded to the wrong or missing glyphs; no fallback for non-embedded CJK, Cyrillic or symbol fonts), **annotations without appearance streams** and widget/hidden-flag handling, **knockout and non-isolated transparency groups**, and **optional-content visibility expressions**. One more area, **CalRGB/CalGray colour**, differs a lot but has no verdict: it needs a reference rendering before anyone can say which engine is wrong. Speed is at rough parity. Rendering the same pages side by side, the median page takes 5.5 ms in both engines, the p95 is 264 ms vs 241 ms, and hayro is faster on 58 % of pages. hayro's tail is worse: a few pathological pages (decompression bombs, tiling patterns) take seconds and up to 0.9–1.6 GB. The laptop was shared, so ratios measured in separate runs were not reproducible. **Recommendation: PDFium stays the default on every surface. hayro becomes the CI oracle now, using the metric below. Revisit switching the default when the font, annotation and optional-content buckets are empty and the pass+minor rate is ≥ 98 % at 96 and 150 DPI.**

## Method

One standalone Cargo project (`[workspace]` table, no lamina crates). `cargo run --release` runs a driver that starts **one child process per document** (`--one <file>`, the same binary re-executed), 6 at a time. Inside the child, each page is rendered with hayro and then PDFium, compared, and reported on stdout. Before every render the child announces `begin <page> <engine>` (binding, warm-up and loading count as page 0 of that engine), so when a child crashes, or stays silent for **30 s** (the per-page timeout), the driver records that page as a crash or timeout for that engine, then restarts the child at the next page. If an engine crashes or hangs while loading, it is disabled for that document and the other engine continues. hayro panics are caught in process (`catch_unwind`). A crash after the child has reported every page is recorded for the document in `rss.csv`, not as a page failure. The child classifies each compared page on the unrounded metric values, and the CSV keeps full precision. No single bad file can end the run.

| Setting | hayro 0.8.0 | PDFium chromium/7881 via `pdfium-render` 0.9.4 |
| --- | --- | --- |
| Page area | CropBox ∩ MediaBox with `/Rotate` (`render_dimensions`, `initial_transform(true)`) | `FPDF_GetPageWidthF/HeightF` (CropBox, rotated), `FPDF_RenderPageBitmap` |
| Pixel size | `floor(points × dpi / 72)` per axis from each engine's own page size (a disagreement shows up as a size mismatch, classed `fail`); longest side capped at 3000 px by the same rule in both | same |
| Background, annotations, forms | white, `render_annotations: true` | white, `FPDF_ANNOT` on, form data on (`FPDF_FFLDraw`) |
| Anti-aliasing | as shipped (no settings) | as shipped: smooth text, images and paths on, no LCD text |
| Fonts | `embed-fonts` defaults: Foxit standard-14 CFF fonts, standard font as the fallback for everything else, no system fonts, embedded CMaps | built-in Foxit fonts plus the system font mapper (macOS) |
| Caching | a fresh `RenderCache` per page (pages independent) | one document handle per child |
| Warm-up | one untimed render of `fixtures/corpus/hello.pdf` per child: PDFium's first text render initialises its system font mapper (≈ 0.6 s on macOS), which would otherwise be billed to page 1 of every document | same |

**Comparison** (`src/compare.rs`). Both renders are box-blurred 3×3 first. Without the blur, two correct rasterisers of the same glyphs disagree on every glyph edge at 96 DPI. Two numbers are computed per page:

- **diff %**: the share of pixels whose CIE76 ΔE (sRGB → CIELAB, D65) exceeds **10** and that are not explained in *both* directions. A pixel is excused only if hayro's colour occurs within ΔE 10 in PDFium's 3×3 neighbourhood of that pixel *and* PDFium's colour occurs within ΔE 10 in hayro's 3×3 neighbourhood; if either direction fails, it counts. This tolerates anti-aliasing and 1-px placement. It doesn't tolerate a missing glyph or 1-px line (the missing side's background sits right next to the line, so only one direction finds a match), a wrong colour or a moved object.
- **SSIM** on BT.601 luma, downsampled 2×2, with an 8×8 sliding window (constants K1 = 0.01, K2 = 0.03), averaged over the page.

| Class | Rule (default thresholds) |
| --- | --- |
| `pass` | diff ≤ 0.5 % and SSIM ≥ 0.90 |
| `minor` | diff ≤ 2 % and SSIM ≥ 0.85 |
| `fail` | otherwise, or the page sizes differ |
| `hayro-error` / `pdfium-error` / `both-error` | load or render error, panic, crash (signal), or a page that only one engine has |
| `timeout` | an engine was silent for 30 s on that page |

**Thresholds** were calibrated by hand. The todo's starting point (pass: SSIM ≥ 0.98 and ≤ 0.5 % pixels over ΔE 10, no blur, no neighbourhood) classed **83 % of a 60-document sample as `fail`**, and the failing pages were visually identical text pages (e.g. PDFBox `cweb.pdf`). The reason is that SSIM between two different rasterisers of the same text lands at 0.95–0.98 even after the blur. I hand-checked ~40 pages across the SSIM × diff % grid (`HAYRO_PARITY_DUMP_ALL=1` writes a triptych for every page). Pages with diff ≤ 0.5 % were indistinguishable at 100 %. 0.5–2 % were small shifts of glyphs, barcodes or table rules. Above 2 % were real differences (wrong glyphs, thinner strokes). SSIM < 0.9 with diff ≈ 0 was shading banding. diff % is therefore the main criterion and SSIM only a guard. The sensitivity table below reports the pass rate at stricter and more lenient settings, so ADR 0004 can choose differently.

**Corpora** (`scripts/fetch-corpora.sh`, pinned commits, SHA-256 per file in [`corpora.manifest.toml`](corpora.manifest.toml), fetched into gitignored `corpora/`):

| Corpus | Commit | Files used | Notes |
| --- | --- | ---: | --- |
| pdf.js `test/pdfs` (Apache-2.0 repository) | [`87b369c`](https://github.com/mozilla/pdf.js/tree/87b369cd9d4cfe415b587b26a8d81f486db02874/test/pdfs) (2026-10-28) | 1,013 | Only PDFs stored in the repository; the **461 `*.link` files** (remote PDFs, some gone) are **not fetched**: more coverage but more churn and unpinnable content. Listed as `skipped_links` in the manifest. |
| PDFBox (Apache-2.0 repository) | [`43e0c58`](https://github.com/apache/pdfbox/tree/43e0c588a76c66a7e738b6632fe245d886ce0826) (2026-10-28) | 175 | Every `*.pdf` in the repository (test resources of all modules). PDFBox's own rendering regression corpus is downloaded from Jira at test time and is not used. |

Individual test PDFs come from bug reports and carry no licence of their own. Nothing is committed, and none may be copied into `fixtures/`: per ADR 0018 §3, a regression is re-created synthetically.

**Machine:** Apple M1 Pro (6 performance + 2 efficiency cores), 16 GB RAM, macOS 27.0.1, rustc 1.99.1, `--release` (default profile). Six documents at a time; the machine was also running other builds, so the wall times are noisy. **C1b must report on the same machine and corpus.**

### Reproduce

```sh
cd spikes/hayro-parity
scripts/fetch-pdfium.sh          # PDFium chromium/7881 for the host, SHA-256 checked, into pdfium/
scripts/fetch-corpora.sh         # pdf.js + PDFBox at the pinned commits into corpora/; fails unless every file
                                 #   matches the committed manifest (--update-manifest rewrites it after a pin bump)
# Smoke check: hello.pdf must classify `pass`, and the broken file must not crash the harness.
cargo run --release -- --out out/smoke --files ../../fixtures/corpus/hello.pdf ../../fixtures/corpus/hello-broken.pdf
cargo run --release                             # 96 DPI, both engines → out/dpi96/{results.csv,summary.md,rss.csv,fail/}
cargo run --release -- --dpi 150                # → out/dpi150/
cargo run --release -- --engine hayro           # one engine alone (clean speed and RSS) → out/dpi96-hayro-only/
cargo run --release -- --engine pdfium
# Quick runs: --sample 10 (every 10th file), --limit 50, --corpus pdfjs|pdfbox, --jobs N, --timeout S
```

The smoke check gives `hello.pdf` → `pass` (SSIM 0.9999, 0.03 %) and `hello-broken.pdf` → `pass` (it is repaired by both engines). Each run writes its own folder under `out/`: `results.csv` (corpus, file, page, class, SSIM, diff %, both times, both sizes, encrypted, errors, hayro warnings, harness errors), `rss.csv` (peak RSS and any crash after the last page, per document), `summary.md`, and a hayro | PDFium | diff triptych PNG for every `fail` page. The harness only deletes an `--out` folder that it created (marked with `.hayro-parity-out`). The summaries of the runs reported here are committed in [`results/`](results/), and the triage in [`results/triage-dpi96.tsv`](results/triage-dpi96.tsv).

## Results

### Classes (96 DPI, default thresholds)

| Corpus | Files | Pages | pass | minor | fail | hayro-error | pdfium-error | both-error | timeout |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdf.js | 1,013 | 1,879 | 1,520 (80.9 %) | 176 (9.4 %) | 143 (7.6 %) | 5 (0.3 %) | 13 (0.7 %) | 17 (0.9 %) | 5 (0.3 %) |
| PDFBox | 175 | 317 | 277 (87.4 %) | 20 (6.3 %) | 10 (3.2 %) | 0 | 1 (0.3 %) | 9 (2.8 %) | 0 |
| **Total** | **1,188** | **2,196** | **1,797 (81.8 %)** | **196 (8.9 %)** | **153 (7.0 %)** | **5 (0.2 %)** | **14 (0.6 %)** | **26 (1.2 %)** | **5 (0.2 %)** |

"Pages" counts one row per page, plus one row for each of the 23 documents that neither engine could open. By document: in 858 of 1,188 documents (72 %) every page passes, and in 1,019 (86 %) every page is pass or minor.

**150 DPI:** pass 1,719 (78.3 %), minor 267 (12.2 %), fail 160 (7.3 %), hayro-error 5, pdfium-error 14, both-error 26, timeout 5 ([summary](results/summary-dpi150.md)). The result barely depends on resolution. pass+minor is 90.8 % at 96 DPI and 90.4 % at 150 DPI. At 150 DPI, more pages move from pass to minor, because the blur and the neighbourhood are fixed in pixels and so are relatively smaller.

### Threshold sensitivity (share of all pages)

| Thresholds | pass 96 DPI | pass+minor 96 DPI | pass 150 DPI | pass+minor 150 DPI |
| --- | ---: | ---: | ---: | ---: |
| strict (pass ≤ 0.1 % and SSIM ≥ 0.95; minor ≤ 1 % and SSIM ≥ 0.90) | 63.5 % | 87.1 % | 36.2 % | 84.3 % |
| **default** (pass ≤ 0.5 % and SSIM ≥ 0.90; minor ≤ 2 % and SSIM ≥ 0.85) | **81.8 %** | **90.8 %** | **78.3 %** | **90.4 %** |
| lenient (pass ≤ 1 % and SSIM ≥ 0.85; minor ≤ 5 % and SSIM ≥ 0.80) | 87.1 % | 94.9 % | 84.3 % | 94.4 % |

Over the pages both engines rendered at the same size (that is, excluding errors), the default pass+minor rate is 93.0 % at 96 DPI.

### Errors, crashes and timeouts (96 DPI)

| | hayro | PDFium |
| --- | --- | --- |
| Crashes (signal) | 0 | 1 (`jpx_numcomps.pdf`, SIGTRAP in the JPX decoder) |
| Panics | 1 (`tiling-pattern-large-steps.pdf`, `Option::unwrap()` on `None`) | n/a |
| Timeouts > 30 s | 2 (`bug1978317.pdf`, `issue16263.pdf`) | 3 (`operator_list_cycle.pdf`, `tiling-pattern-tiny-steps*.pdf`) |
| Cannot open, other opens | 3 (`encrypted-attachment.pdf`, `issue15590.pdf`: `Invalid`; `auth-event-ef-open.pdf`: an encrypted embedded-file stream reported as password-protected) | 9 (`FormatError` on files hayro opens, e.g. `bug1250079`, `bug1606566`, `bug1980958`, `PDFBOX-6040-nodeloop`; 0 pages in two `issue19484_*` files and a fuzzed file; `crypt_filters.pdf` and `pr6531_2.pdf` password errors) |
| Page missing or empty | 2 (`issue22011.pdf` p57 beyond hayro's page count; `Pages-tree-refs.pdf` p2, which PDFium also fails) | 1 (`boundingBox_invalid.pdf` p3: empty box) |

**Encryption** (40 pages from files containing `/Encrypt`): 12 pass, 4 minor, 18 both-errors, 2 hayro-only errors and 4 PDFium-only errors.

- **Both fail:** password-protected files (no password was given, so both fail correctly) and PDFBox's AES samples, which both engines reject (hayro: `UnsupportedAlgorithm`; PDFium: `SecurityError`).
- **hayro only:** `auth-event-ef-open.pdf` (only the embedded files are encrypted, and hayro treats the document as password-protected) and `encrypted-attachment.pdf` (`Invalid`).
- **PDFium only:** `crypt_filters.pdf` and `pr6531_2.pdf` (`PasswordError`), and the two `issue19484_*` files, where PDFium finds no pages.

**hayro 0.8.0 decrypts documents with an empty user password** (12 encrypted pages pass; the rendering notes listed encryption as unverified). Encryption is therefore a small gap on both sides (2 pages vs 4), not a reason to hold hayro back.

### Failure buckets (manual triage)

Every `fail` page at 96 DPI was looked at in its triptych: 153 pages in 123 documents, of which 150 pages in 122 documents were triaged (one 3-page PDFBox form was not). Where it was clear which engine matches the PDF's intent (usually from the test file's own text, e.g. "the colour should be red", or from what pdf.js expects), it is recorded.

| Bucket | Pages (docs) | Closer to intent | Examples |
| --- | ---: | --- | --- |
| **Comparison noise** (no visible difference; rasteriser and glyph placement at small sizes, mostly tiny 266×66 px pdf.js test pages where a few glyph edges are > 2 %) | 31 (31) | — | `issue7115`, `franz`, `knockout_nested`, `issue4402_reduced` |
| **Fonts: embedded font decoded to wrong or missing glyphs** (encoding/cmap, seac accents, recursive composite glyphs) | 19 (19) | PDFium in 14 | `issue11578_reduced` ("Xqnwo gu" vs "Volumes"), `issue2840` (text missing), `endchar` (accent), `recursiveCompositGlyf` |
| **Colour** (CalRGB/CalGray, Indexed out-of-range, transfer functions, one DeviceN logo): **unclear, needs a reference rendering**. The two engines disagree strongly on CalRGB/CalGray, and nothing in the files says which is right | 16 (6) | PDFium in 0, hayro in 1, unclear in 15 | `calrgb` (9 pages), `calgray` (3), `transfer_maps`, `issue6931_reduced` (PDFium black where the file says red) |
| **Annotations and forms** (no appearance stream → hayro draws nothing; hidden/NoView flags; widget text; highlight blending) | 16 (16) | PDFium in 8 | `annotation-square-circle-without-appearance`, `rc_annotation`, `annotation_hidden_noview`, `bug1865341`, `issue12337` |
| **Standard-14 substitution** (Courier/Helvetica glyph shapes and widths differ: Foxit CFF in hayro vs PDFium's mapping) | 15 (2) | unclear | `standard_fonts` (14 pages), `issue5747` |
| **Fonts: non-embedded fallback** (CJK and vertical CJK missing in hayro, Cyrillic/Polish missing, symbol fonts, bold/italic substitution) | 14 (13) | PDFium in 9, hayro in 1 | `issue13343`, `XiaoBiaoSong`, `vertical_rotated_fallback_glyphs`, `issue19550`, `issue7454`; `bug920426` (PDFium shows boxes) |
| **Strokes and hairlines** (zero-width and thin lines, stroked text, table rules) | 11 (9) | PDFium in 2 | `issue12295`, `issue20294_reduced`, `eu-001` (PDFBox) |
| **Patterns** (tiling-pattern phase and spacing, pattern-filled text) | 7 (7) | unclear | `pattern_fill_pending_state`, `issue16038`, `tiling_patterns_variations` |
| **Transparency** (knockout groups, non-isolated groups with soft masks, blend in groups) | 5 (5) | PDFium in 1 | `knockout_blend_multiply`, `knockout_inner_backdrop`, `nonisolated_blend_smask` |
| **Images** (JPX subsampling and `SMaskInData`, resampling edges) | 5 (5) | unclear | `jpx_subsampling`, `jpx_smaskindata`, `large_jpeg_downscale` |
| **Shading** (CMYK function-based shading, radial extend) | 3 (2) | hayro in 1 | `function_based_shading_cmyk`, `radial_gradients` p2 (PDFium paints a stray band) |
| **Type 3 fonts** (`d0` colour, glyph errors, word spacing) | 3 (3) | unclear | `type3_d0_alpha`, `type3_glyph_error` |
| **Page geometry** (empty or invalid boxes → different page sizes) | 3 (2) | unclear | `boundingBox_invalid`, PDFBox `PDFBOX-6018-…-OrphanPopups` |
| **Optional content** (visibility expressions ignored by hayro) | 1 (1) | PDFium | `visibility_expressions` |
| Other (hayro blank page) | 1 (1) | PDFium | `bug2086012` |
| Not triaged | 3 (1) | — | PDFBox `PDFBOX-3656-SF1199AEG (Complete).pdf` |

So about 20 % of the `fail` class is metric noise on tiny pages. Of the real failures, **fonts are the largest group** (51 pages across the four font buckets, 37 documents). The font bucket matters most, because one missing CJK fallback can make a whole document unreadable. Most other differences are local. hayro's own warning sink rarely fires on failing pages: `UnresolvedAnnotationAppearance` fired on 4 failing pages, so the warnings can't stand in for the diff.

### Speed and memory

**Paired runs** (both engines render the same page one after the other in the same process, six documents at a time; time per page rendered without error). These are the most comparable numbers, because both engines see the same load at the same moment:

| | hayro 96 DPI | PDFium 96 DPI | hayro 150 DPI | PDFium 150 DPI |
| --- | ---: | ---: | ---: | ---: |
| Median page | **5.5 ms** | **5.5 ms** | 12.2 ms | 13.3 ms |
| p95 page | 264 ms | 241 ms | 303 ms | 316 ms |
| Total, all pages | 157 s | 127 s | 206 s | 163 s |
| Pages where hayro is faster | 57.8 % | | 57.5 % | |

**Single-engine runs** (`--engine hayro|pdfium`, 96 DPI) measure peak RSS per child process (= one document) without the other engine loaded. They were run twice on the shared laptop, and their *times* did not reproduce: hayro's median was 4.9 ms then 7.9 ms, PDFium's 3.5 ms then 6.6 ms, and the p95s moved by about 3×. Only the memory figures are stable enough to quote:

| Peak RSS per document | hayro | PDFium |
| --- | ---: | ---: |
| Median / p95 | 15.7 / 26.7 MiB | 26.7 / 44.2 MiB |
| Worst document | 0.9–1.6 GB (`bomb_giant.pdf`, `tiling-pattern-box.pdf`; depends on where the 30 s timeout stops it) | 325–449 MiB (`issue19517.pdf`) |

hayro's tail is heavier. In the paired 96 DPI run it takes 16 s on `bomb_giant.pdf` and 9.5 s on `tiling-pattern-box.pdf`, where PDFium takes 4–9 ms. It is more than 10× slower than PDFium on 178 of 2,150 pages, and PDFium is more than 10× slower than hayro on 173. The worst memory cases are hayro's (a decompression bomb and a tiling pattern), and lamina's `Limits` (ADR 0003) would have to bound them if hayro ever renders untrusted input in a worker. PDFium's per-process baseline is higher (≈ 27 MiB vs ≈ 16 MiB), but that is native code and isn't comparable with WASM, which C1b measures. **Speed doesn't decide anything today. Measure it again on a quiet machine (the CI runner) before using it as a gate.**

## Limitations

- **96/150 DPI only.** Thin-line and glyph-hinting differences change with resolution (the strict pass rate falls from 64 % to 36 % at 150 DPI, while pass+minor barely moves). Print-resolution parity is not measured.
- **PDFium is the reference, not the truth.** Most triaged failures have no clear verdict, and in 3 of the 39 that have one, PDFium is the one that is wrong (a DeviceN logo drawn black, boxes for glyphs, a stray radial band); it also crashed once on a JPX file. A `fail` means "different from PDFium".
- **Fonts depend on configuration.** hayro used its embedded defaults (standard-14 fallback for everything). Wiring a CJK/Noto fallback through `InterpreterSettings::font_resolver`, as hayro's own example does, would likely close most of the non-embedded bucket. That is product configuration for C2b, not a hayro change. PDFium on macOS also reads system fonts; on Linux CI it has fewer to choose from, so X3b numbers will differ slightly.
- **The `*.link` half of the pdf.js corpus is excluded**, and PDFBox contributes only its 175 in-repository files. The corpus is weighted towards pdf.js regression files: small, deliberately odd, and often one page.
- **Timing ran on a shared, busy laptop.** Only the paired run's ratios are comparable; separate single-engine runs gave medians that differed by up to 2× between repetitions. hayro used a fresh cache per page, which costs it on multi-page documents.
- **Native only.** WASM size, memory and speed are C1b's job.

## Licences

`cargo deny --manifest-path spikes/hayro-parity/Cargo.toml --config deny.toml check licenses bans sources` with the repository's `deny.toml`: **bans ok, licenses ok, sources ok** (warnings only for duplicate `fearless_simd`, `miniz_oxide` and `syn` versions).

| Component | Version | Licence |
| --- | --- | --- |
| `hayro` (+ `hayro-syntax`, `-interpret`, `-jbig2`, `-jpeg2000`, `-ccitt`, `-cmap`, `-postscript`), `vello_cpu`, `kurbo`, `peniko` | 0.8.0 (vello_cpu 0.3.0) | MIT OR Apache-2.0 |
| `pic-scale`, `moxcms`, `pxfm` (hayro image scaling and colour) | — | BSD-3-Clause OR Apache-2.0 |
| `pdfium-render` | 0.9.4 (`pdfium_7881`, `thread_safe`, no `image` feature) | **MIT OR Apache-2.0** (confirmed on crates.io; the rendering notes had it as unconfirmed) |
| `libloading` (dynamic binding) | — | ISC |
| PDFium binary: [`bblanchon/pdfium-binaries`](https://github.com/bblanchon/pdfium-binaries) `chromium/7881` (`pdfium-mac-arm64.tgz`, SHA-256 `52e94ca5…2e40`; Linux x64 and macOS x64 hashes pinned in the script) | PDFium 151.0.7881.0 | PDFium **BSD-3-Clause**; packaging MIT; bundled third-party code with its own notices in `pdfium/licenses/`: FreeType (FTL), libjpeg-turbo (IJG + BSD-3-Clause), OpenJPEG (BSD-2-Clause), lcms2 (MIT), zlib, libpng, libtiff, AGG 2.3, ICU (Unicode), Abseil (Apache-2.0), fast_float, simdutf, llvm-libc. Shipping PDFium (C2b, `@lamina/render-pdfium`) must carry these notices. |
| `png` | 0.18.1 | MIT OR Apache-2.0 |
| `anyhow`, `libc` | 1.0.104, 0.2.191 | MIT OR Apache-2.0 |
| Rest of the tree | — | MIT OR Apache-2.0 / MIT / Unicode-3.0 / Zlib / BSD-3-Clause / ISC (all on the allow-list) |

The comparison metric is our own code (SSIM and ΔE in `src/compare.rs`). `dssim` was deliberately avoided because it is AGPL-3.0. MuPDF was not used even as a comparison (ADR 0001).

## Recommendation for ADR 0004

1. **Default renderer per surface: PDFium everywhere** (web reader via PDFium-WASM, desktop and mobile via native PDFium through `pdfium-render`, server-side thumbnails via native PDFium). hayro is not the default on any surface in v1. **hayro is the CI oracle** (`RenderBackend` adapter behind a feature, used by the nightly diff and by tests), and it is selectable in the web reader's debug menu as planned.
2. **The parity metric** for X3b and C2b's goldens is the one in this spike: 3×3 box blur, then the share of pixels with ΔE76 > 10 and no ΔE ≤ 10 match in the other image's 3×3 neighbourhood (in both directions), with SSIM (2×2-downsampled luma, 8×8 window) as a guard. Classes: `pass` ≤ 0.5 % and SSIM ≥ 0.90; `minor` ≤ 2 % and SSIM ≥ 0.85; else `fail`; size mismatch = `fail`; errors and timeouts classed separately.
3. **The nightly number X3b tracks** is the **pass+minor share of all pages** on the pinned pdf.js + PDFBox corpus at **96 DPI**. Track the pass share, each error class and the per-bucket fail counts next to it, and alert on regressions, not on absolute values. **The figures here (90.8 % pass+minor, 81.8 % pass; 150 DPI: 90.4 %) are macOS figures and only indicative.** PDFium's output depends on the system fonts it finds, and C2b will give hayro a Noto fallback. X3b must therefore establish its own baseline on the CI runner (Linux, with whatever fallback fonts C2b ships) in its first run, and compare against that. Corpus commits and manifest hashes are pinned (`scripts/fetch-corpora.sh` fails on any mismatch); bumping them is a deliberate PR that resets the baseline.
4. **Switch criteria** for making hayro the default on a surface (each is necessary):
   - **pass+minor ≥ 98 % of all pages at both 96 and 150 DPI**, on the pinned corpus plus lamina's own `fixtures/`, with **zero hayro-only incidents** (today 7: 5 hayro-only errors, one of which is a panic, and 2 hayro timeouts);
   - **empty buckets** (after excluding pages where hayro is verified correct): *fonts* (embedded decoding, non-embedded fallback with a shipped CJK/Cyrillic/symbol fallback, Type 3), *annotations without appearance streams and annotation flags* (the reader renders annotations itself as overlays, so this can be met by lamina generating appearances), *optional content*;
   - **colour** (CalRGB/CalGray, transfer functions) is not a gate yet: first settle which engine is right against a reference rendering, then turn whichever engine is wrong into a bucket that must be empty;
   - tolerated if documented: patterns, strokes, shading, transparency/knockout and image-resampling differences under 1 % of pages;
   - **speed and memory**: median and p95 page time within 1.5× of PDFium in a *paired* run on a quiet machine (the CI runner). On this laptop the paired run shows parity (medians equal, p95 1.1×), but separate runs were not reproducible, so this needs a clean measurement before it can be a gate. Every pathological page (bombs, tiling patterns) must also be bounded by `Limits`, rather than taking seconds and gigabytes;
   - for the web surface, C1b's WASM size and memory numbers must also favour hayro, which is the reason to switch at all (a smaller download).
5. **Configuration to do in C2b, not a hayro change:** wire a font resolver with Noto CJK/Cyrillic/symbol fallbacks for hayro, because most of the non-embedded font bucket is that. Measure again with this harness before deciding anything.
