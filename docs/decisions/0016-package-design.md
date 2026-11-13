# 0016 — Package design: ESM-only, export conditions, split WASM modules, size budgets

- Status: Accepted (owner, 2026-10-30: the module split and the budgets in §6, option A of §5)
- Date: 2026-10-30
- Amended 2026-10-31 (owner): the TypeScript API package is **`@laminapdf/sdk`** in `packages/sdk` (first `@lamina/pdf`, then `@laminapdf/pdf` under the new scope; [P0](../todos/done/P0-npm-scope-laminapdf.md)). Its subpaths are `@laminapdf/sdk/explain` and `@laminapdf/sdk/worker`; everything else in this ADR is unchanged.
- Amended 2026-11-04 ([L2b](../todos/done/L2b-disposal-and-memory.md)): §8, a memory ceiling linked into each module and a fresh instance after a trap.

## Context

[ADR 0002](0002-binding-strategy.md) fixed the binding: `crates/laminapdf-wasm` (wasm-bindgen + tsify), the `cargo xtask wasm` pipeline (wasm-bindgen CLI pinned to Cargo.lock, binaryen's `wasm-opt` pinned with rustc's feature set) and the committed `@laminapdf/wasm` manifest with a gitignored `dist/`. It left the package design to this ADR: module format, export conditions, how `init()` takes the module, whether the WASM is split, and the size budgets.

The budgets needed measuring first. The [`krilla-fit` spike](../../spikes/krilla-fit/README.md#measurements) put the builder stack (pdf-writer, skrifa, subsetter, no shaper) at 377 KB gzipped, already over the plan's 350 KB for `@laminapdf/sdk` *with* shaping. [ADR 0005 §6](0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) point 4 asked B6 to decide the split and re-measure with `wasm-opt`. On 2026-10-29 the owner decided the budget is **per module**, 350 KB gzipped for the base module ([pdf-library §4](../plans/pdf-library.md#non-functional), [shareable-libraries §8](../plans/shareable-libraries.md#8-changes-to-existing-todos)).

Constraints:

- One `.wasm` can't be tree-shaken by the caller: what is linked is downloaded. Splitting features into separate modules is the only way a user who doesn't draw text avoids paying for HarfRust and ICU4X ([repo notes §4](../research/architecture/notes/repo-and-packaging.md#4-package-design-esm-only-exports-maps-runtime-compatibility-lazy-wasm-bundlers-jsr)).
- Two WASM instances share no memory. A document living in one can't hand a font program to the other without copying bytes through JavaScript.
- `laminapdf-build` writes fonts into the document's overlay when the build finishes (`FontRegistry::finish_fonts`): text isn't a pure function from (font, string) to bytes that a second module could compute on the side.
- Workers (workerd) can't fetch or compile WASM at run time: they import a `WebAssembly.Module`. Browsers want `compileStreaming` on a URL; Node and Deno read bytes ([report: one `.wasm` file per export condition](../research/libraries/shareable-libraries.md#one-wasm-file-resolved-per-export-condition-with-an-escape-hatch)).

## Decision

### 1. ESM-only

`@laminapdf/wasm` and `@laminapdf/sdk` are ES modules only (`"type": "module"`, no CommonJS build), as the [TypeScript guideline](../guidelines/typescript.md#modules-and-packaging) requires for every package. `require(esm)` works in every Node we support (≥ 22), so CommonJS callers aren't locked out.

### 2. Export conditions

`@laminapdf/sdk` (B6b) resolves the loader per runtime: `browser`, `workerd`, `node`, `deno`, `default`, each with `types` first. Each condition has its own small loader for the same `.wasm` files:

| Condition | How the `.wasm` is obtained |
| --- | --- |
| `browser` | `WebAssembly.compileStreaming(fetch(new URL("…/lamina_pdf_base.wasm", import.meta.url)))`, so bundlers emit the asset |
| `workerd` | a static `import wasm from "…/lamina_pdf_base.wasm"` (a `WebAssembly.Module`, as Workers require) |
| `node`, `deno` | the file next to the module, read as bytes |
| `default` | as `browser` (Bun, other runtimes with `fetch` of `file:` URLs) |

`@laminapdf/wasm` itself has no conditions: it exports the raw glue per module (`@laminapdf/wasm/base`, `@laminapdf/wasm/text`) and the `.wasm` files as assets (`@laminapdf/wasm/lamina_pdf_base.wasm`, …). It stays private; whether it is published or bundled into `@laminapdf/sdk` is decided with the first release (P1/P2). Until then `@laminapdf/sdk` depends on it with `workspace:*`.

### 3. `init()` input forms; synchronous after it

`init(input?)` accepts a URL (or string), a `Response` (or a promise of one), bytes (`ArrayBuffer`, `Uint8Array`) or a compiled `WebAssembly.Module`, and is idempotent. With no input, the export condition's loader (§2) picks the file. The wasm-bindgen glue provides exactly these forms (`--target web`: the default `init` export and `initSync({ module })`), so `@laminapdf/sdk`'s `init` is a thin wrapper. After `init`, **every call is synchronous**: long work is a job stepped by the caller ([ADR 0028](0028-facade-rules-jobs-and-cancellation.md) §3), never a promise.

### 4. Three modules: base, text and extract, one crate built three times

| Module | Built as | Contains |
| --- | --- | --- |
| `lamina_pdf_base.wasm` | `laminapdf-wasm` without features (`laminapdf` with `build`) | Open, read, edit with ops, undo/redo, save (save jobs, warnings); the builder without text: pages and boxes, paths, colour, opacity and blend modes, clipping, images (JPEG, PNG, RGBA), links, destinations, outline, page labels, metadata, viewer preferences |
| `lamina_pdf_text.wasm` | `laminapdf-wasm --features text` | Everything in base, plus fonts (`fontBytes`, `standardFont`), shaped `text` runs and `textBox` (laminapdf-text: skrifa, subsetter, HarfRust, ICU4X, the standard 14), and text extraction (`WasmPage.text`); flowing layout, Markdown and JSON documents once [ADR 0032](0032-flowing-layout-and-document-inputs.md#5-placement) lands |
| `lamina_pdf_extract.wasm` | `laminapdf-wasm --features extract,reflow` (B9c; `reflow`, U4b) | Everything in base, plus text extraction (`WasmPage.text`: laminapdf-text's extractor, CMaps, ICU4X's bidi data) and reader mode (`WasmDocument.reflow`, which the text module leaves out), without fonts for building or shaping |

The text module is a **superset**, not an add-on: a document that needs text is built entirely in the text module. A document moves between the two as PDF bytes (save, then open), which is the narrow ABI between them. `@laminapdf/sdk` loads the base module by default and the text module on its first text call or with `init({ text: true })` (B6b); a page that loaded only base never downloads text.

To make the base module exclude text, `laminapdf-build` gained a `text` feature (default on): without it, the builder has no fonts, no text runs or boxes, and doesn't link `laminapdf-text`. The façade's `text` feature now implies a new `build` feature (the builder alone).

**Text extraction (B9c, owner decision 2026-10-30): a third module.** [B9c](../todos/done/B9c-text-extraction-bindings-and-differential.md) measured three placements (gzip): base with extraction 523 KB (+143 KB: ICU4X's bidi data and the font parsing join base), the text module with extraction 1,008 KB (+56 KB, since it already has both), and a third `extract` module, base plus extraction, about 525 KB. The owner chose the third: a reader's text layer and search load about half the text module, and nobody who doesn't extract pays for it. The text module has extraction too (it stays a superset). `@laminapdf/sdk` opens documents in the richest module loaded (text, then extract, then base), and `page.text()` on a document opened in base is `extract-module-not-loaded`. The façade gained an `extract` feature (laminapdf-text's extractor without the builder's text), which `text` implies. Two consequences: documents opened while the text module is loaded now live in its memory, not base's (so `wasmMemory()` and the nightly memory history move to `text` for runs that load it), and loading the extract module while the text module is loaded or loading does nothing. In workerd, where each module is a static import that every Worker bundles, the loader leaves the extract module out (about 526 KB): a Worker that extracts text imports `@laminapdf/wasm/lamina_pdf_extract.wasm` and passes it to `loadExtract`.

### 5. Measurements and the shrinking pass

All sizes are of the base and text modules as `cargo xtask wasm` builds them (rustc 1.99.1, `wasm-release`: `lto = true`, `codegen-units = 1`, `panic = "abort"`), gzip −9, bytes. "bindgen" is the wasm-bindgen output before `wasm-opt`.

| Step | Base (bindgen) | Base (after wasm-opt) | Text (bindgen) | Text (after wasm-opt) |
| --- | ---: | ---: | ---: | ---: |
| First build: base links laminapdf-text, unexported (`opt-level = "s"`, serde-wasm-bindgen records, `-Oz`) | 882,047 | 931,949 | 1,147,049 | 1,206,285 |
| `laminapdf-build` `text` feature: base without laminapdf-text | 527,264 | 554,594 | 1,147,045 | 1,206,275 |
| Records as JSON through `serde_json` (already linked) instead of serde-wasm-bindgen | 525,591 | 550,644 | 1,148,485 | 1,204,246 |
| `opt-level = "z"` instead of `"s"` | 495,619 | 529,679 | 1,064,042 | 1,135,095 |
| SASLprep optional (`saslprep` feature; stringprep's Unicode tables out of the WASM) | 425,053 | 457,932 (−Oz) | 994,993 | 1,064,873 (−Oz) |
| Flat drawing ABI (paths as `Float64Array`, colours as component arrays, names as strings) | 417,160 | 444,964 (−Oz) | — | — |
| Own PNG decoder instead of the `png` crate (`fdeflate`, a second `miniz_oxide`, `crc32fast`) | ≈ 369,000 | **367,904** (−O1, final) | — | **933,367** (−O1, final) |

For comparison, the read/edit/save core alone (no builder) was 363,785 B before the SASLprep step and **293,076 B** after it; the spike's laminapdf-cos-only module was 59,851 B. The rest of the base module is the builder (≈ 75 KB with `pdf-writer`, images, navigation and metadata), the save planner and writers, decryption (AES, RC4, MD5, SHA-2), `serde_json` (op logs and the ADR 0026 object) and the hint and summary tables of `codes.json`.

What `wasm-opt` does, on the base module of the JSON-records row (bindgen output: 1,423,835 B raw, 525,591 B gzip, 415,829 B brotli):

| `wasm-opt` | Raw | gzip | brotli |
| --- | ---: | ---: | ---: |
| `-Oz` | 1,305,138 | 550,637 | 437,722 |
| `-Oz --converge` | 1,304,580 | 550,840 | 436,306 |
| `-Os` | 1,313,613 | 550,292 | 436,891 |
| `-O2` | 1,327,952 | 553,761 | 439,829 |
| `-O1` | 1,364,900 | **519,355** | **414,161** |
| strip only | 1,424,166 | 525,522 | 416,094 |

On the `opt-level = "z"` build the same holds (`-Oz` 529,672 B, `-O1` 489,939 B gzip; `-O1` without inlining 536,948 B).

`-Oz` gives the smallest raw file and a 6–8 % **larger** gzip on these modules (skipping its function merging or reordering passes didn't change that). Downloads are compressed, so `cargo xtask wasm` uses **`-O1`**, recorded with the rest of the pipeline in `tools/xtask/src/wasm.rs`. The shrinking pass also checked the todo's other items: no `anyhow` in any core crate (only xtask and tests), `panic = "abort"`, `codegen-units = 1` and fat LTO were already the `wasm-release` profile.

**The budget question.** The base module came out at 367.9 KB, 5 % over the owner's 350 KB. Three ways forward were put to the owner:

- **A (chosen):** revise the base budget to 380 KB (the measurement plus ≈ 3 % headroom), keep the two-module split.
- **B:** keep 350 KB and cut further: pass the remaining option records flat instead of as JSON, serve the hint texts from `codes.json` in TypeScript instead of the WASM (ADR 0026 §7 already keeps hints out of the core crates), and possibly move the outline, page labels and viewer preferences into the text module. Estimated 15–20 KB, not certain, and a clumsier ABI.
- **C:** three modules: core (read/edit/save, 293 KB) under 350 KB, a builder module (≈ 368 KB) and text.

The owner chose **A** on 2026-10-30.

### 6. Budgets per module

Gzip bytes as `cargo xtask size` measures them (deflate level 9 plus the gzip header; a few hundred bytes more than `gzip -9`), enforced in the fast lane's `wasm` job from `packages/wasm/size-budgets.json`, which holds the same numbers:

| Module | Measured (2026-10-30, after the review fixes and B9d; unchanged by B6b, whose TS-only codes stay out of the WASM) | Budget | Headroom |
| --- | ---: | ---: | ---: |
| base (`lamina_pdf_base.wasm`) | 650,089 on `main` (with the six editor PRs below merged, 2026-11-13); 691,975 with cross-document pages (E2c) | **735,000** (amended 2026-11-13 for E2d; 727,000 for E2c; 658,000 from E3d, 2026-11-13; 657,000 earlier that day; 464,000 from 2026-11-12; 460,000 from 2026-11-10; was 410,000) | 5 % over the open PRs |
| text (`lamina_pdf_text.wasm`) | 1,452,686 on `main` (2026-11-13); 1,734,152 with cross-document pages (E2c) and links (E3c) | **1,780,000** (amended 2026-11-13 for E2c and E3c together, the owner's call; 1,735,000 for E2d; 1,708,000 earlier that day; 1,460,000 from 2026-11-11; 1,400,000 with T4j alone, 1,375,000 with E1c alone; was 1,360,000) | 5 % over the open PRs |
| JS glue per module (wasm-bindgen output, minified by esbuild) | 3,769 base, 5,158 text (T4i); extract 5,011 with `imagesToPdf` (E3m, 2026-11-13) | **5,000** each; extract **5,500** (amended 2026-11-13 for E2d; 5,300 for E2c; 5,200 from E3m that day); text **6,000** (amended 2026-11-10) | |
| `@laminapdf/sdk` JS: both glues + the TS API, bundled and minified by esbuild (B6b, `packages` in `size-budgets.json`) | 22,192 on `main` (2026-11-13); about 22,900 with the open PRs below | **24,500** (amended 2026-11-13 for E2d; 24,000 earlier that day; 22,500 from 2026-11-11; 22,000 from 2026-11-10; was 21,000) | 5 % over the open PRs |
| `@laminapdf/sdk/worker/entry` JS: the worker file, which bundles the whole TS API (`packages` in `size-budgets.json`) | 20,242 on `main` (2026-11-13); about 20,900 with the open PRs below | **22,500** (amended 2026-11-13 for E2d; 22,000 earlier that day; 21,000 from 2026-11-11; was 20,000) | 5 % over the open PRs |
| extract (`lamina_pdf_extract.wasm`, B9c) | 974,364 on `main` (2026-11-13); 1,017,601 with E2c | **1,069,000** (amended 2026-11-13 for E2c; 1,003,000 from 2026-11-13; 784,000 from 2026-11-12; 780,000 from 2026-11-10; was 725,000) | 5 % over the open PRs |
| tiff (`lamina_pdf_tiff.wasm`, E3m): the web reader's TIFF writer, loaded when a TIFF export starts | 62,374 (2026-11-13) | **70,000** | 12 % |
| render (C2) | — | placeholder: decided by C2's ADRs | |
| sign (E10) | — | placeholder: decided by E10a | |

The `@laminapdf/sdk` TS layer has its own budget (glue + API ≤ 20 KB min+gzip, plan §4), enforced by B6b. A module without a budget fails the check, so a new module can't ship unmeasured. A module that grows more than 5 % over the baseline in `size-budgets.json` gets a twiggy top-items diff against the baseline in the job summary; `cargo xtask size --update-baseline` records a new baseline when a growth is accepted. Raising a budget is an amendment of this table, approved by the owner.

**Amendment (owner, 2026-11-13, [E2d](../todos/done/E2d-merge-and-split.md)):** the base module's budget rises from 727,000 to 735,000 bytes, the text module's from 1,708,000 to 1,735,000, the extract module's JavaScript glue from 5,300 to 5,500, `@laminapdf/sdk`'s JS from 24,000 to 24,500 and the worker file's from 22,000 to 22,500. Merging and splitting (`mergeDocuments`, `Document.merge`, `split`) ship in every module, since every module can edit: the outline copy, the layer and page-label merges and the split. Measured on the PR's branch: the text module 1,732,406 bytes, the extract glue 5,410, the SDK's JS 24,360 and the worker file 22,260. With `main` merged in, the base module measured 728,434 against its 727,000, so its budget rises to 735,000 too, about 1 % above that, approved separately. The extract module (1,061,328) stays under its budget. The new budgets leave a little room above these figures rather than L3e's 5 %, since the other modules' budgets weren't raised.

**Amendment (owner, 2026-11-13, E2c's glue):** the extract module's JavaScript glue budget rises to 5,300 bytes, minified and gzipped. E2c adds two bindings to it, `addSource` and `extractPages`, and with E3d's on `main` the glue measured 5,007 bytes; 5,300 is about 5 % over that. E3m's raise to 5,200 for `imagesToPdf` (5,011 bytes) landed on `main` meanwhile; the two together stay under 5,300, which CI's size job checks. The base and text glue budgets stay as they are.

**Amendment (owner, 2026-11-13, [E2c](../todos/done/E2c-cross-document-pages.md)):** the base module's budget rises from 658,000 to 727,000 bytes and the extract module's from 1,003,000 to 1,069,000. Copying pages between documents (`addSource`, `insertPagesFrom`, `replacePages`, `extractPages`, `exportRanges`) wasn't among the six PRs the raise below covered; it adds 41,886 bytes to the base module, 43,237 to the extract module and 44,275 to the text module, gzipped, mostly the cross-document page copier and the destination fixing it needs. On `main` with the six PRs merged, the base module reaches 691,975 and the extract module 1,017,601; the new budgets are those plus 5 % (L3e's rule). The text module, at 1,689,852, still fits its 1,708,000. E3d (#516), merged after these figures were taken, adds about 7,400 bytes to the base module, which leaves it about 4 % under its new budget.

**Amendment (owner, 2026-11-13, E3d after E3b):** the base module's budget rises from 657,000 to 658,000 bytes. Once E3b (#535) merged, `main` measured 650,089 bytes, more than the shared raise below expected, because E3b grew past its 20,000-byte allowance. E3d (#516) then measured 657,446 bytes, 446 over the budget, though it adds 7,357 bytes, under the 8,463 set aside for it. The extract and text modules stay within their budgets.

**Amendment (owner, 2026-11-13, the link ops):** the base module's budget rises from 658,000 to 700,000 bytes and the extract module's from 1,003,000 to 1,040,000. With `AddLink`, `UpdateLink` and `DeleteLink` (E3c, #542), base measured 671,392 against `main`'s 650,089 (+21,303), extract 1,001,906 against 974,364 (+27,542) and text 1,674,302 against 1,645,577 (+28,725), so base was over its budget and extract within about 1 KB of it. The owner chose room for the next editor todos, which add ops to every module the same way, over a raise for E3c alone.

**Amendment (owner, 2026-11-13, one raise for the open editor PRs):** the base module's budget rises from 464,000 to 657,000 bytes, the extract module's from 784,000 to 1,003,000, the text module's from 1,460,000 to 1,708,000, `@laminapdf/sdk`'s JS from 22,500 to 24,000, the worker file's from 21,000 to 22,000 and the worker client's from 5,000 to 5,200. `main` itself was over budget after E2r (base 464,334, extract 784,655), and six open PRs each raised the same numbers for themselves, so the owner asked for one raise that covers them all, each PR's own amendment then dropped. Every module can edit, so each new op joins all three modules. The growth against `main`, gzipped, as each PR's CI or its session measured it:

| PR | base | extract | text |
| --- | ---: | ---: | ---: |
| E2b page structure ops (#534; with the model's `edit` feature and `page-checks`) | +79,235 | +72,998 | +73,354 |
| E3e general XMP editing (#518) | +32,819 | +32,637 | +33,566 |
| E4b annotation views and ops (#526) | +17,014 | +24,290 | +25,308 |
| E2q page labels (#533; the tree writers, which #534's `edit` also links) | +8,807 | +9,456 | +11,139 |
| E3d initial view and article threads (#516) | +8,463 | +15,539 | +14,830 |
| E3b outline editing (#535), not yet measured: an allowance | +20,000 | +20,000 | +20,000 |

Their sum, less about 5 KB for the tree writers counted twice, comes to about 626,000, 955,000 and 1,626,000 bytes. The owner chose that plus 5 % (L3e's rule) so the PRs merge in any order without another amendment. The JS budgets take the same 5 % over `main` plus the PRs' JS growth (about +700 bytes for the API, +670 for the worker file, +100 for the client). This is a large step, about 40 % on the base module, and it is a ceiling for these six PRs, not a target: the next round measures again against what they actually add once merged.

**Amendment (owner, 2026-11-12, [E3h](../todos/done/E3h-images-to-pdf-and-page-export.md)):** the base module's budget rises from 460,000 to 464,000 bytes and the extract module's from 780,000 to 784,000. On `main` they measured 459,804 and 779,723 bytes gzipped, 196 and 277 bytes under budget; with E3h they measure 461,213 and 781,324, about 1.5 KB more each. The builder's image loading, which every module links, now reads a JPEG's JFIF and Exif segments and a PNG's `pHYs` and `eXIf` chunks for the image's orientation and resolution, and the error table gains `export-failed` with its explanation. Page export itself stays out of the WebAssembly (`laminapdf-render`'s `export` feature). The owner chose 4 KB over the old budgets, which leaves about 2.8 KB of room in each.

**Amendment (owner, 2026-11-11, E1c and T4j together):** the text module's budget rises to 1,460,000 bytes and `@laminapdf/sdk`'s JS to 22,500. Each of the two raised the budgets for itself (1,375,000 and 1,400,000); together the module is 1,446,960 bytes and the JS 22,091. The new budgets leave about 1 % of room.

**Amendment (owner, 2026-11-11, [T4j](../todos/done/T4j-documents-in-typescript.md)):** the text module's budget rises to 1,400,000 bytes. `WasmBuilder.flow` links the `laminapdf.document/1` format: reading it (serde), checking it with `didYouMean`, filling templates and converting its blocks into flow elements. That is 74,707 bytes gzip in all (1,391,637), no single item larger than 7 KB, against the 43 KB T4i's budget left.

**Amendment (owner, 2026-11-11, [E1c](../todos/done/E1c-metadata-ops.md)):** the budget of `@laminapdf/sdk/worker/entry`, the worker file, rises from 20,000 to 21,000 bytes; the table now lists it. The file bundles the whole TypeScript API, so every new `Document` method grows it. With the edit session ([E1d](../todos/done/E1d-edit-session-undo-ui.md)) on `main` and the metadata methods (E1c: `metadata`, `setMetadata`, `removeMetadata`, `plan`, `wouldViolate`) merged into it, it measured 20,025 bytes, and 20,015 after trimming the metadata helpers. The new budget is that measurement plus about 5 % headroom, the rule of L3e's amendment, so the next few methods fit without an amendment each.

**Amendment (owner, 2026-11-13, [E3m](../todos/done/E3m-web-image-export.md)):** the extract module's JS glue budget rises from 5,000 to 5,200 bytes. `imagesToPdf` and the image list it takes, which only the extract and text modules hold (the reader loads base at startup), put extract's glue at 5,011. The new `tiff` module, the TIFF writer the web reader's worker loads on first use, gets a budget of 70,000 bytes and the usual 5,000 for its glue.

**Amendment (owner, 2026-11-10, [E1c](../todos/done/E1c-metadata-ops.md)):** the base module's budget rises from 410,000 to 460,000 bytes, the extract module's from 725,000 to 780,000 and the text module's from 1,360,000 to 1,375,000. The metadata ops (`SetMetadata`, `RemoveMetadata`, and the planner's save stamp) put the XMP reader and writer into every module for the first time, since each module can edit. On `main` the modules measured 402,520, 723,826 and 1,316,825 bytes gzipped; with E1c they measure 458,144, 777,477 and 1,370,896, about 54 KB more each. twiggy counts raw bytes, before gzip: on the base module it puts about 50 KB in the XMP reader and writer, 12 KB in quick-xml, 36 KB in the serde code for the new ops and their patch, and 18 KB in the metadata module. A façade feature that left metadata out of the base module would still leave extract and text over budget, and slimmer deserialisers would save an estimated 10–15 KB. The owner chose to raise the budgets to the measurement plus a few kilobytes rather than L3e's 5 % headroom, so the next feature that grows a module will need its own amendment.

**Amendment (owner, 2026-11-10, [T4i](../todos/done/T4i-build-command-and-bindings.md)):** the text module's budget rises to 1,360,000 bytes, its JS glue's to 6,000 and `@laminapdf/sdk`'s JS to 22,000. ADR 0032 §9's 1,280,000 was never written into `size-budgets.json`, because nothing in WebAssembly called the flow layer until `WasmBuilder.markdown`, so the linker left it out. That call links the flow layer (about 60 KB gzip, measured before `wasm-opt`) and the Markdown parser and converter (about 90 KB) at once: 1,316,930 bytes against ADR 0032's estimate of about 80 KB of growth. The budget leaves room for JSON documents and templates ([T4e](../todos/done/T4e-json-documents-and-templates.md)) and `builder.flow` ([T4j](../todos/done/T4j-documents-in-typescript.md)).

**Amendment (owner, 2026-11-09, C6a):** the extract module's budget rises from 690,000 to 725,000 bytes and `@laminapdf/sdk`'s JS from 20,000 to 21,000. The outline and named destinations ([C6a](../todos/done/C6a-thumbnails-outline-page-labels.md)) link `laminapdf-model`'s outline and destination code into the extract and text modules for the first time: on `main` with the text layer (C5a) the extract module measured 681,541 and the JS 19,879; with C6a, 691,643 and 20,265. The new budgets are the measurement plus about 5 % headroom (the JS's, about 3.5 %), the rule of L3e's amendment. The text module fits its budget unchanged (1,178,372 of 1,200,000).

**Amendment (owner, 2026-11-08, [ADR 0032](0032-flowing-layout-and-document-inputs.md#9-budgets)):** the text module's budget rises from 1,200,000 (T3a's amendment below) to 1,280,000 bytes for flowing layout ([T4b](../todos/done/T4b-flow-layout-core.md) and the todos after it), the Markdown front end and JSON documents with their templates, which ADR 0032 §5 places in the text module. The [`flow-layout` spike](../../spikes/flow-layout/README.md) measured a minimal paginator at +19 KB and `pulldown-cmark` at +60 KB gzip on top of the text module; the rest is headroom for tables, notes and the document schema. The base and extract modules don't change.

**Note (owner, 2026-11-08, T2a):** the `covered` flag ([T2a](../todos/done/T2a-covered-text-flag.md)) adds about 10 KB gzip to both modules: on its own branch it measured extract 653,461 and text 1,139,945. The owner approved a raise for it to 685,000 and 1,190,000, but the T3a amendment below landed first with higher budgets, which hold T2a as well (main plus T2a, T3b and T3c comes to about text 1,152,000 and extract 668,000), so T2a keeps them.

**Amendment (owner, 2026-11-08, T3a):** the text module's budget rises from 1,130,000 to 1,200,000 bytes and the extract module's from 650,000 to 690,000. Three Round T extraction todos were open at once, each over a budget on its own: the text-layer status ([T3a](../todos/done/T3a-text-layer-status.md)), the column-order checks ([T3b](../todos/done/T3b-reading-order-confidence.md)) and tables across pages ([T3c](../todos/done/T3c-tables-across-pages.md)) add about 1.8, 3.4 and 7.8 KB to the text module and 1.4, 4.3 and 7.7 KB to the extract module. On `main` before them the modules measured text 1,128,561 and extract 643,140, so the three together come to about 1,141,600 and 656,500. The new budgets are that sum plus about 5 % headroom, the rule of L3e's amendment, so the next extraction features don't each need an amendment.

**Amendment (owner, 2026-11-06, L3g):** the extract module's budget rises from 620,000 to 650,000 bytes. Structure-tree reading order ([L3g](../todos/done/L3g-extract-structure-tree.md)) links `laminapdf-model`'s structure-tree walk into the module, along with the tagged-element blocks and the clip and contrast checks: it measured 623,634 bytes, 3.6 KB over the old budget. The new budget leaves about 4.5 % headroom for the next extraction features. The text module fits its budget unchanged (1,109,063 of 1,130,000).

**Amendment (owner, 2026-11-05, L3e):** the extract module's budget rises from 560,000 to 620,000 bytes and the text module's from 1,070,000 to 1,130,000. `doc.extract()` ([L3e](../todos/done/L3e-extract-bindings-and-benchmark.md)) links `laminapdf-extract` into both modules: lines, blocks, XY-Cut++ reading order, ruled and unruled tables, and the Markdown, JSON and chunk writers. With it the modules measured extract 588,769 and text 1,074,722, 28.8 KB and 4.7 KB over the old budgets. That is after L3e cut about 6 KB from each by writing `ErrorCode::from_code` and `WarningCode::from_code` as a scan rather than a string `match`. The new budgets are those measurements plus about 5 % headroom. The owner chose this over a fourth module for retrieval extraction: extraction for an LLM belongs with text extraction, and a page that never reads text still downloads only the base module.

**Amendment (owner, 2026-10-31):** every module's budget rises by about 5 %: base from 390,000 to 410,000 bytes, extract from 535,000 to 560,000 and text from 1,020,000 to 1,070,000. With L1b (generated error codes), B9c (text extraction) and B8a (tagged PDF) together, the modules measured base 389,668, extract 534,032 and text 1,019,875: 332, 968 and 125 bytes under their budgets, so the next feature would fail the check. The table's headroom is against its last recorded measurements; against the combined ones it is 5.0 %, 4.6 % and 4.7 %. The raise buys headroom for Round B's remaining features (B8b's conformance profiles first) without a budget amendment per PR; growth is still visible on every PR as a twiggy diff once a module passes 5 % over its baseline.

**Amendment (owner, 2026-10-30, B9c):** the extract module's budget is 535,000 bytes, and the text module's rises from 960,000 to 1,020,000, both for text extraction (§4).

**Amendment (owner, 2026-10-30, L2c):** the base budget rises from 380,000 to 390,000 bytes. Range loading (`WasmDocument.openSource` over a sparse source, [L2c](../todos/done/L2c-range-byte-source.md)) links `laminapdf-cos`'s windowed reader (64 KiB windows, chunked repair, xref-entry probes), which the base module never used while it only opened in-memory bytes: +9.3 KB gzip (372,688 → 382,000). Range loading belongs with reading, so it stays in the base module rather than moving to a third one.

### 7. API naming and shape at the WASM boundary

- **Classes** are the façade's types with a `Wasm` prefix (`WasmDocument`, `WasmPage`, `WasmSaveJob`, `WasmWarningCursor`, `WasmBuilder`, `WasmPageBuilder`, `WasmImage`, `WasmFont`), so the raw layer is never confused with `@laminapdf/sdk`'s `Document`, `Builder` and so on. **Methods** are the façade's, camelCase (`page_info` → `pageInfo`, [ADR 0028](0028-facade-rules-jobs-and-cancellation.md) F12); parameters too.
- **Bytes** are `Uint8Array` in and out (F7); paths are `Float64Array`s in a flat encoding (verb, numbers…), colours arrays of 1, 3 or 4 components: no record conversion on the drawing path.
- **Records** (options, results) are plain JS objects with `.d.ts` types from tsify (F5, F6): every option optional, unknown keys refused with `unknown-option` and `didYouMean`. They cross as JSON (`JSON.stringify` → `serde_json` and back), because the core already links `serde_json`; serde-wasm-bindgen's generic (de)serialiser cost more (§5). This is a measured exception to ADR 0002 §1's "tsify `js` mode": tsify still writes the types.
- **String unions** are camelCase, as the op JSON (`"setInfo"`, `"fullRewrite"`, `"evenOdd"`). Standard 14 fonts are named by their PostScript names (`Helvetica`, `Times-Roman`).
- **Errors** are thrown as the [ADR 0026](0026-error-and-diagnostic-contract.md) §1 object (a plain object), which `@laminapdf/sdk` turns into its `LaminaPdfError` (ADR 0026 §7). The binding's own codes are `bindingErrors` in `fixtures/codes.json`: `invalid-argument`, `unknown-option`, `builder-finished`, `source-budget-exceeded` and `disposed` (§8).
- **The builder's binding form** (left to B6a by B5c): `WasmBuilder.page(width, height)` returns an owned `WasmPageBuilder` that draws on its own `Surface` and is written into the builder by `finish()`, with the same bytes as the Rust API. No method consumes its handle: a finished page or builder answers every later call with `builder-finished` (the §1 object), never with wasm-bindgen's null-pointer error. A call that fails draws nothing (a text run's options are all checked before it is drawn).
- **Unknown keys:** the record's own field list (serde's, read once per type) decides which key is unknown and what `didYouMean` suggests; serde's message, which quotes the caller's key, is never parsed (ADR 0026 §4). A key unknown to a nested record gives `unknown-option` without suggestions.
- **Passwords:** without the `saslprep` feature, the WASM modules apply SASLprep's mapping step to an R5/R6 password (RFC 4013 §2.1: non-ASCII spaces become U+0020, the "commonly mapped to nothing" characters of RFC 3454 B.1 are removed; a few code points, in every build) but not its NFKC normalisation or its prohibited-character and bidi checks. `@laminapdf/sdk` passes the password as given and, when that fails with `wrong-password` and its NFKC form (`String.prototype.normalize`) differs, tries the NFKC form (B6b review): R2–R4 passwords reach the core unchanged, and every password SASLprep accepts is prepared the same way for R5/R6; one it would reject is tried mapped instead of refused.

### 8. Memory: a ceiling linked into each module, a fresh instance after a trap

WebAssembly memory grows as a module needs it and never shrinks, and a trap leaves the instance's memory half-written. The bullets below say how Lamina PDF bounds both, and how it tests that.

- **A ceiling.** `cargo xtask wasm` links Lamina PDF's three modules with a maximum memory of 2 GiB (`MAX_MEMORY` in `tools/xtask/src/wasm.rs`) and reads it back from every module it ships. Engines allow a wasm32 memory up to 4 GiB and map pages only as the module grows, so the ceiling costs nothing until it is reached; past it, an allocation fails. `Limits` ([ADR 0003](0003-reader-tolerance-and-limits.md) §3) remain the first defence, and most hostile files fail there long before. The maximum is fixed at link time, so a caller can't lower it: an app that needs a tighter bound runs the SDK in a Web Worker and ends the Web Worker.
- **A fresh instance after a trap.** Stable wasm32 aborts on a panic or a failed allocation, and the abort is a trap. `laminapdf-wasm`'s global allocator records a failed allocation first, so `@laminapdf/sdk` throws `out-of-memory` for one and `wasm-trap` for any other trap. Either way, the next call replaces the instance with a fresh one of the same compiled module, through wasm-bindgen's `__wbg_reset_state` (the `--experimental-reset-state-function` flag of the pinned wasm-bindgen-cli). `reset()` replaces every loaded module's instance on request, which returns memory to its size after `init()`. Every handle of a replaced instance throws `instance-reset`, and freeing one is a no-op, so nothing reaches the new instance's memory.
- **Per-call checks removed.** The flag also makes the glue check every call's objects and call a guard first. `@laminapdf/sdk` checks each handle's instance itself, so `cargo xtask wasm` removes those per-call lines (`tools/xtask/src/wasm_glue.rs`), which cost about 2 KB gzip over the three modules. Code that calls `__wbg_reset_state` on the raw `@laminapdf/wasm` glue must make the same check.
- **Children freed with their document.** `WasmPage` and `WasmSaveJob` hold their document weakly. Freeing a `WasmDocument` frees the document's memory at once, and a page or save job used afterwards throws `disposed`, now a binding code as well as one `@laminapdf/sdk` raises.
- **No test hooks.** The maintainability guideline keeps them out of `src/`, so the recovery is tested without them. `cargo xtask wasm` also links the base module with a 64 MiB ceiling into `target/wasm/small-memory/`, never shipped, where opening a larger file runs out of memory for real. Other traps come only from bugs, so `@laminapdf/sdk`'s tests throw one through the guard every call runs through.

## Consequences

- A page that only reads, edits, saves or draws vectors and images downloads ≈ 368 KB gzipped; one that draws text downloads ≈ 937 KB, once. Both are measured on every PR, and growth shows up as a twiggy diff before it becomes a budget failure.
- Text and non-text documents are built in different instances; moving a document between them costs a save and an open. The TS layer hides the choice.
- `laminapdf-build`, `laminapdf-syntax`/`laminapdf-cos` (`saslprep`) and the façade (`build`) have features whose combinations `cargo xtask portability` builds; the workspace dependencies on `laminapdf-build`, `laminapdf-syntax` and `laminapdf-cos` are `default-features = false`, and crates ask for `text` and `saslprep` where they need them.
- `png` is now a dev-dependency only (it writes test PNGs and is the reference the new decoder is compared with).
- The WASM pipeline needs wasm-bindgen-cli and twiggy (cargo installs, pinned in `doctor.rs`) and binaryen 133, which `cargo xtask wasm` downloads into `target/tools/` with a pinned SHA-256 when `PATH` has none.
- ADR 0005 §6's budget question is answered: per module, as §6 says.

## Alternatives considered

- **One module with everything** (≈ 937 KB): every user pays for shaping and ICU4X data. Rejected; the split is the point of the per-module budget.
- **A text add-on module linked at run time through JavaScript** (the base module owns the document, the text module shapes and subsets and returns bytes): font embedding writes into the overlay at `finish` and shaping needs the font registry the document owns, so every call would copy font programs and glyph tables across instances, and `laminapdf-build` and `laminapdf-text` would have to be split along a new internal ABI. The superset is simpler and costs the text user only the base module's download once more, if both are loaded. Revisit when a measurement shows the double download matters.
- **Options B and C of §5**: see §5.
- **`wasm-opt -Oz`**: smallest raw size, larger gzip (§5).
- **serde-wasm-bindgen records** (ADR 0002's default): more code than JSON through the already linked `serde_json` (§5); the types come from tsify either way.
- **wasm-pack**: rejected in ADR 0002; the xtask pipeline pins each tool.
