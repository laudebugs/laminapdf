# 0016 — Package design: ESM-only, export conditions, split WASM modules, size budgets

- Status: Accepted (owner, 2026-10-30: the module split and the budgets in §6, option A of §5)
- Date: 2026-10-30

## Context

[ADR 0002](0002-binding-strategy.md) fixed the binding: `crates/lamina-wasm` (wasm-bindgen + tsify), the `cargo xtask wasm` pipeline (wasm-bindgen CLI pinned to Cargo.lock, binaryen's `wasm-opt` pinned with rustc's feature set) and the committed `@lamina/wasm` manifest with a gitignored `dist/`. It left the package design to this ADR: module format, export conditions, how `init()` takes the module, whether the WASM is split, and the size budgets.

The budgets needed measuring first. The [`krilla-fit` spike](../../spikes/krilla-fit/README.md#measurements) put the builder stack (pdf-writer, skrifa, subsetter, no shaper) at 377 KB gzipped, already over the plan's 350 KB for `@lamina/pdf` *with* shaping. [ADR 0005 §6](0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) point 4 asked B6 to decide the split and re-measure with `wasm-opt`. On 2026-10-29 the owner decided the budget is **per module**, 350 KB gzipped for the base module ([pdf-library §4](../plans/pdf-library.md#non-functional), [shareable-libraries §8](../plans/shareable-libraries.md#8-changes-to-existing-todos)).

Constraints:

- One `.wasm` can't be tree-shaken by the caller: what is linked is downloaded. Splitting features into separate modules is the only way a user who doesn't draw text avoids paying for HarfRust and ICU4X ([repo notes §4](../research/architecture/notes/repo-and-packaging.md#4-package-design-esm-only-exports-maps-runtime-compatibility-lazy-wasm-bundlers-jsr)).
- Two WASM instances share no memory. A document living in one can't hand a font program to the other without copying bytes through JavaScript.
- `lamina-build` writes fonts into the document's overlay when the build finishes (`FontRegistry::finish_fonts`): text isn't a pure function from (font, string) to bytes that a second module could compute on the side.
- Workers (workerd) can't fetch or compile WASM at run time: they import a `WebAssembly.Module`. Browsers want `compileStreaming` on a URL; Node and Deno read bytes ([report: one `.wasm` file per export condition](../research/libraries/shareable-libraries.md#one-wasm-file-resolved-per-export-condition-with-an-escape-hatch)).

## Decision

### 1. ESM-only

`@lamina/wasm` and `@lamina/pdf` are ES modules only (`"type": "module"`, no CommonJS build), as the [TypeScript guideline](../guidelines/typescript.md#modules-and-packaging) requires for every package. `require(esm)` works in every Node we support (≥ 22), so CommonJS callers aren't locked out.

### 2. Export conditions

`@lamina/pdf` (B6b) resolves the loader per runtime: `browser`, `workerd`, `node`, `deno`, `default`, each with `types` first. Each condition has its own small loader for the same `.wasm` files:

| Condition | How the `.wasm` is obtained |
| --- | --- |
| `browser` | `WebAssembly.compileStreaming(fetch(new URL("…/lamina_base.wasm", import.meta.url)))`, so bundlers emit the asset |
| `workerd` | a static `import wasm from "…/lamina_base.wasm"` (a `WebAssembly.Module`, as Workers require) |
| `node`, `deno` | the file next to the module, read as bytes |
| `default` | as `browser` (Bun, other runtimes with `fetch` of `file:` URLs) |

`@lamina/wasm` itself has no conditions: it exports the raw glue per module (`@lamina/wasm/base`, `@lamina/wasm/text`) and the `.wasm` files as assets (`@lamina/wasm/lamina_base.wasm`, …). It stays private; whether it is published or bundled into `@lamina/pdf` is decided with the first release (P1/P2). Until then `@lamina/pdf` depends on it with `workspace:*`.

### 3. `init()` input forms; synchronous after it

`init(input?)` accepts a URL (or string), a `Response` (or a promise of one), bytes (`ArrayBuffer`, `Uint8Array`) or a compiled `WebAssembly.Module`, and is idempotent. With no input, the export condition's loader (§2) picks the file. The wasm-bindgen glue provides exactly these forms (`--target web`: the default `init` export and `initSync({ module })`), so `@lamina/pdf`'s `init` is a thin wrapper. After `init`, **every call is synchronous**: long work is a job stepped by the caller ([ADR 0028](0028-facade-rules-jobs-and-cancellation.md) §3), never a promise.

### 4. Three modules: base, text and extract, one crate built three times

| Module | Built as | Contains |
| --- | --- | --- |
| `lamina_base.wasm` | `lamina-wasm` without features (`lamina` with `build`) | Open, read, edit with ops, undo/redo, save (save jobs, warnings); the builder without text: pages and boxes, paths, colour, opacity and blend modes, clipping, images (JPEG, PNG, RGBA), links, destinations, outline, page labels, metadata, viewer preferences |
| `lamina_text.wasm` | `lamina-wasm --features text` | Everything in base, plus fonts (`fontBytes`, `standardFont`), shaped `text` runs and `textBox` (lamina-text: skrifa, subsetter, HarfRust, ICU4X, the standard 14), and text extraction (`WasmPage.text`) |
| `lamina_extract.wasm` | `lamina-wasm --features extract` (B9c) | Everything in base, plus text extraction (`WasmPage.text`: lamina-text's extractor, CMaps, ICU4X's bidi data), without fonts for building or shaping |

The text module is a **superset**, not an add-on: a document that needs text is built entirely in the text module. A document moves between the two as PDF bytes (save, then open), which is the narrow ABI between them. `@lamina/pdf` loads the base module by default and the text module on its first text call or with `init({ text: true })` (B6b); a page that loaded only base never downloads text.

To make the base module exclude text, `lamina-build` gained a `text` feature (default on): without it, the builder has no fonts, no text runs or boxes, and doesn't link `lamina-text`. The façade's `text` feature now implies a new `build` feature (the builder alone).

**Text extraction (B9c, owner decision 2026-10-30): a third module.** [B9c](../todos/done/B9c-text-extraction-bindings-and-differential.md) measured three placements (gzip): base with extraction 523 KB (+143 KB: ICU4X's bidi data and the font parsing join base), the text module with extraction 1,008 KB (+56 KB, since it already has both), and a third `extract` module, base plus extraction, about 525 KB. The owner chose the third: a reader's text layer and search load about half the text module, and nobody who doesn't extract pays for it. The text module has extraction too (it stays a superset). `@lamina/pdf` opens documents in the richest module loaded (text, then extract, then base), and `page.text()` on a document opened in base is `extract-module-not-loaded`. The façade gained an `extract` feature (lamina-text's extractor without the builder's text), which `text` implies. Two consequences: documents opened while the text module is loaded now live in its memory, not base's (so `wasmMemory()` and the nightly memory history move to `text` for runs that load it), and loading the extract module while the text module is loaded or loading does nothing. In workerd, where each module is a static import that every Worker bundles, the loader leaves the extract module out (about 526 KB): a Worker that extracts text imports `@lamina/wasm/lamina_extract.wasm` and passes it to `loadExtract`.

### 5. Measurements and the shrinking pass

All sizes are of the base and text modules as `cargo xtask wasm` builds them (rustc 1.99.1, `wasm-release`: `lto = true`, `codegen-units = 1`, `panic = "abort"`), gzip −9, bytes. "bindgen" is the wasm-bindgen output before `wasm-opt`.

| Step | Base (bindgen) | Base (after wasm-opt) | Text (bindgen) | Text (after wasm-opt) |
| --- | ---: | ---: | ---: | ---: |
| First build: base links lamina-text, unexported (`opt-level = "s"`, serde-wasm-bindgen records, `-Oz`) | 882,047 | 931,949 | 1,147,049 | 1,206,285 |
| `lamina-build` `text` feature: base without lamina-text | 527,264 | 554,594 | 1,147,045 | 1,206,275 |
| Records as JSON through `serde_json` (already linked) instead of serde-wasm-bindgen | 525,591 | 550,644 | 1,148,485 | 1,204,246 |
| `opt-level = "z"` instead of `"s"` | 495,619 | 529,679 | 1,064,042 | 1,135,095 |
| SASLprep optional (`saslprep` feature; stringprep's Unicode tables out of the WASM) | 425,053 | 457,932 (−Oz) | 994,993 | 1,064,873 (−Oz) |
| Flat drawing ABI (paths as `Float64Array`, colours as component arrays, names as strings) | 417,160 | 444,964 (−Oz) | — | — |
| Own PNG decoder instead of the `png` crate (`fdeflate`, a second `miniz_oxide`, `crc32fast`) | ≈ 369,000 | **367,904** (−O1, final) | — | **933,367** (−O1, final) |

For comparison, the read/edit/save core alone (no builder) was 363,785 B before the SASLprep step and **293,076 B** after it; the spike's lamina-cos-only module was 59,851 B. The rest of the base module is the builder (≈ 75 KB with `pdf-writer`, images, navigation and metadata), the save planner and writers, decryption (AES, RC4, MD5, SHA-2), `serde_json` (op logs and the ADR 0026 object) and the hint and summary tables of `codes.json`.

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
| base (`lamina_base.wasm`) | 382,000 (brotli 304,955; L2c, 2026-10-30) | **390,000** (amended 2026-10-30; was 380,000) | 2.1 % |
| text (`lamina_text.wasm`) | 1,008,844 (brotli 770,741; with extraction, B9c) | **1,020,000** (amended 2026-10-30; was 960,000) | 1.1 % |
| JS glue per module (wasm-bindgen output, minified by esbuild) | 3,769 base, 3,978 text | **5,000** each | |
| `@lamina/pdf` JS: both glues + the TS API, bundled and minified by esbuild (B6b, `packages` in `size-budgets.json`) | 10,041 | **20,000** | 50 % |
| extract (`lamina_extract.wasm`, B9c) | 526,184 (brotli 414,641) | **535,000** (owner, 2026-10-30) | 1.7 % |
| render (C2) | — | placeholder: decided by C2's ADRs | |
| sign (E10) | — | placeholder: decided by E10a | |

The `@lamina/pdf` TS layer has its own budget (glue + API ≤ 20 KB min+gzip, plan §4), enforced by B6b. A module without a budget fails the check, so a new module can't ship unmeasured. A module that grows more than 5 % over the baseline in `size-budgets.json` gets a twiggy top-items diff against the baseline in the job summary; `cargo xtask size --update-baseline` records a new baseline when a growth is accepted. Raising a budget is an amendment of this table, approved by the owner.

**Amendment (owner, 2026-10-30, B9c):** the extract module's budget is 535,000 bytes, and the text module's rises from 960,000 to 1,020,000, both for text extraction (§4).

**Amendment (owner, 2026-10-30, L2c):** the base budget rises from 380,000 to 390,000 bytes. Range loading (`WasmDocument.openSource` over a sparse source, [L2c](../todos/done/L2c-range-byte-source.md)) links `lamina-cos`'s windowed reader (64 KiB windows, chunked repair, xref-entry probes), which the base module never used while it only opened in-memory bytes: +9.3 KB gzip (372,688 → 382,000). Range loading belongs with reading, so it stays in the base module rather than moving to a third one.

### 7. API naming and shape at the WASM boundary

- **Classes** are the façade's types with a `Wasm` prefix (`WasmDocument`, `WasmPage`, `WasmSaveJob`, `WasmWarningCursor`, `WasmBuilder`, `WasmPageBuilder`, `WasmImage`, `WasmFont`), so the raw layer is never confused with `@lamina/pdf`'s `Document`, `Builder` and so on. **Methods** are the façade's, camelCase (`page_info` → `pageInfo`, [ADR 0028](0028-facade-rules-jobs-and-cancellation.md) F12); parameters too.
- **Bytes** are `Uint8Array` in and out (F7); paths are `Float64Array`s in a flat encoding (verb, numbers…), colours arrays of 1, 3 or 4 components: no record conversion on the drawing path.
- **Records** (options, results) are plain JS objects with `.d.ts` types from tsify (F5, F6): every option optional, unknown keys refused with `unknown-option` and `didYouMean`. They cross as JSON (`JSON.stringify` → `serde_json` and back), because the core already links `serde_json`; serde-wasm-bindgen's generic (de)serialiser cost more (§5). This is a measured exception to ADR 0002 §1's "tsify `js` mode": tsify still writes the types.
- **String unions** are camelCase, as the op JSON (`"setInfo"`, `"fullRewrite"`, `"evenOdd"`). Standard 14 fonts are named by their PostScript names (`Helvetica`, `Times-Roman`).
- **Errors** are thrown as the [ADR 0026](0026-error-and-diagnostic-contract.md) §1 object (a plain object), which `@lamina/pdf` turns into its `LaminaError` (ADR 0026 §7). The binding's own codes are `bindingErrors` in `fixtures/codes.json`: `invalid-argument`, `unknown-option`, `builder-finished`.
- **The builder's binding form** (left to B6a by B5c): `WasmBuilder.page(width, height)` returns an owned `WasmPageBuilder` that draws on its own `Surface` and is written into the builder by `finish()`, with the same bytes as the Rust API. No method consumes its handle: a finished page or builder answers every later call with `builder-finished` (the §1 object), never with wasm-bindgen's null-pointer error. A call that fails draws nothing (a text run's options are all checked before it is drawn).
- **Unknown keys:** the record's own field list (serde's, read once per type) decides which key is unknown and what `didYouMean` suggests; serde's message, which quotes the caller's key, is never parsed (ADR 0026 §4). A key unknown to a nested record gives `unknown-option` without suggestions.
- **Passwords:** without the `saslprep` feature, the WASM modules apply SASLprep's mapping step to an R5/R6 password (RFC 4013 §2.1: non-ASCII spaces become U+0020, the "commonly mapped to nothing" characters of RFC 3454 B.1 are removed; a few code points, in every build) but not its NFKC normalisation or its prohibited-character and bidi checks. `@lamina/pdf` passes the password as given and, when that fails with `wrong-password` and its NFKC form (`String.prototype.normalize`) differs, tries the NFKC form (B6b review): R2–R4 passwords reach the core unchanged, and every password SASLprep accepts is prepared the same way for R5/R6; one it would reject is tried mapped instead of refused.

## Consequences

- A page that only reads, edits, saves or draws vectors and images downloads ≈ 368 KB gzipped; one that draws text downloads ≈ 937 KB, once. Both are measured on every PR, and growth shows up as a twiggy diff before it becomes a budget failure.
- Text and non-text documents are built in different instances; moving a document between them costs a save and an open. The TS layer hides the choice.
- `lamina-build`, `lamina-syntax`/`lamina-cos` (`saslprep`) and the façade (`build`) have features whose combinations `cargo xtask portability` builds; the workspace dependencies on `lamina-build`, `lamina-syntax` and `lamina-cos` are `default-features = false`, and crates ask for `text` and `saslprep` where they need them.
- `png` is now a dev-dependency only (it writes test PNGs and is the reference the new decoder is compared with).
- The WASM pipeline needs wasm-bindgen-cli and twiggy (cargo installs, pinned in `doctor.rs`) and binaryen 133, which `cargo xtask wasm` downloads into `target/tools/` with a pinned SHA-256 when `PATH` has none.
- ADR 0005 §6's budget question is answered: per module, as §6 says.

## Alternatives considered

- **One module with everything** (≈ 937 KB): every user pays for shaping and ICU4X data. Rejected; the split is the point of the per-module budget.
- **A text add-on module linked at run time through JavaScript** (the base module owns the document, the text module shapes and subsets and returns bytes): font embedding writes into the overlay at `finish` and shaping needs the font registry the document owns, so every call would copy font programs and glyph tables across instances, and `lamina-build` and `lamina-text` would have to be split along a new internal ABI. The superset is simpler and costs the text user only the base module's download once more, if both are loaded. Revisit when a measurement shows the double download matters.
- **Options B and C of §5**: see §5.
- **`wasm-opt -Oz`**: smallest raw size, larger gzip (§5).
- **serde-wasm-bindgen records** (ADR 0002's default): more code than JSON through the already linked `serde_json` (§5); the types come from tsify either way.
- **wasm-pack**: rejected in ADR 0002; the xtask pipeline pins each tool.
