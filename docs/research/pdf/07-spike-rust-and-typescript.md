# 07 — Spike: the Same PDF Core in Rust and TypeScript

> Part of the [PDF research series](README.md). Code: [`spikes/pdf-basics/`](../../../spikes/pdf-basics/) ([SPEC](../../../spikes/pdf-basics/SPEC.md) · [Rust](../../../spikes/pdf-basics/rust/README.md) · [TypeScript](../../../spikes/pdf-basics/ts/README.md)). Date: 2026-10-28.
>
> **Question tested:** chapter 05 §8 recommends a Rust core shipped to web (WASM), desktop and mobile, with TypeScript as the first binding. Before committing to that, we built the *same* minimal PDF core twice from one spec and ran both against the same fixtures, to compare what each language costs and buys, and to prove a Rust core can be called from TypeScript.

## 1. What was built

Both implementations cover the same scope (SPEC §1–4), all of it described in chapters [01](01-file-structure.md) and [02](02-imaging-model.md):

| Layer | What it does |
| --- | --- |
| Object model | All PDF object types; exact serialisation (name `#xx` escapes, string escapes, reals without exponents, Python-compatible half-even rounding) |
| Writer | Classic xref table (both legal 20-byte EOL forms), **incremental update** (§7.5.6), **compact mode** (object stream + xref stream, PDF 1.5) |
| Reader | Lexer/parser; xref tables and streams; `/Prev` chains; hybrid `/XRefStm`; object streams; Flate + PNG predictors; **repair** by scanning for `n g obj`; page-tree walk with inheritance; hard limits (depth 256, 64 MiB decompression, cycle detection) |
| Content builder | Path, colour, text, graphics-state operators; Bézier circle helper |

Hard rule for both: **bytes in, bytes out.** No file system, network, threads, clocks or platform APIs in library code. That's what makes one core portable to every target.

## 2. How it was tested

One shared test plan (SPEC "Tests"), implemented natively in each language against shared fixtures in `spikes/pdf-basics/fixtures/`:

1. **Golden, byte-identical:** rebuild `hello.pdf` (ch. 01) and `imaging-demo.pdf` (ch. 02) exactly. Both implementations also reproduce `hello-incremental.pdf` byte-for-byte from `hello.pdf` via their incremental writer.
2. **Raw-writer parity** with chapter 05's dependency-free Python writer.
3. **Corpus:** all 13 PDFs from the research examples (including pdf-lib, ReportLab, fpdf2, pyHanko-signed, AES-256-encrypted, broken-xref and xref-stream files). Facts checked against `expected.json`, produced independently by pypdf.
4. **Round-trip:** write → read → re-serialise; escaping edge cases; incremental updates over 2–3 revisions; compact mode.
5. **Robustness:** thousands of truncations, byte flips and garbage inputs, plus hostile cases (self-referencing `/Kids`, reference cycles, 10 000-deep nesting, Flate bomb, a crafted PNG-predictor row size). Must return an error, never crash or hang.
6. **Portability:** Rust `cargo check` for `wasm32-unknown-unknown`, `aarch64-apple-ios`, `aarch64-linux-android`; TS source scan for Node-only APIs, plus loading the built ESM.
7. **Rust core from TypeScript:** the Rust crate compiled to WASM with a 4-function C ABI, loaded in Node with plain `WebAssembly.instantiate` (no wasm-bindgen). Output is byte-identical, plus a **differential fuzz**: the same ~4 300 mutated files through both readers.

External validation: the compact and incremental outputs from both implementations open in **pypdf with `strict=True`**.

Process: both were built in parallel by separate agents, reviewed with `/code-review` (10 findings: 2 high-severity in TS, a memory-safety-adjacent allocation bug in both, a missing object cache in Rust, and several cross-implementation divergences), then fixed with a regression test per finding.

## 3. Results

| | Rust | TypeScript |
| --- | --- | --- |
| Tests | 169 (70 unit, 95 integration, 4 doc) | 222 |
| Library source | ~4 000 lines incl. doc comments and ~520 lines of unit tests | ~2 300 lines |
| Runtime deps | `miniz_oxide`, `thiserror` (MIT/Apache/Zlib) | `fflate` (MIT) |
| Shipped size | WASM 172 KB (74 KB gzipped); no `wasm-opt` | 16.4 KiB min+gz incl. fflate (11.6 KiB without) |
| Lint/type safety | clippy `-D warnings`, no `unwrap`/`expect`/`panic`/indexing in library code; `unsafe` only in the WASM shim | TS strict + `noUncheckedIndexedAccess` + `exactOptionalPropertyTypes`; biome |
| Platforms verified | native macOS; `cargo check` wasm32, iOS, Android | Node 24; ESM with no Node APIs (browser, Deno, Bun, React Native/Hermes ≥ 0.70 for BigInt) |
| Golden byte-identical | ✔ (+ incremental fixture) | ✔ (+ incremental fixture) |
| Corpus (13 files) | ✔ all facts match pypdf | ✔ all facts match pypdf |
| pypdf strict on outputs | ✔ 6 files | ✔ 8 files |

**Differential fuzz (Rust-in-WASM vs TS reader):** 3 783 / 4 305 agreed on the first run. After aligning tolerance policies and fixing review findings: 4 294 / 4 305. Then Rust was taught to repair when the stored xref is valid but `/Root` doesn't resolve to a dict, giving **4 304 / 4 305**. The one remaining divergence is fflate accepting corrupt Huffman tables that zlib and miniz reject.

## 4. What we learned

**The PDF-level findings apply to any implementation:**

- **Writers need an escape hatch.** Hand-formatted bodies (e.g. a dict split across lines) can't be reproduced by a canonical serialiser, so both grew `addRaw`. Content streams also need *per-operand* number precision (`0.20` vs `0.2`) to match existing producers byte-for-byte.
- **Number formatting is a cross-language trap.** JS `toFixed` rounds exact ties wrongly for this purpose (`0.125.toFixed(2)` → `"0.13"`; Python and Rust give `"0.12"`). Byte-stable output across bindings needs one specified rounding rule. We chose half-even on the exact binary value.
- **Tolerance has to be a written policy, not an accident.** The first differential run disagreed on 12% of damaged files, every one of them a policy difference: missing kids, missing header, which trailer wins in repair, duplicate xref entries, integer overflow, truncated deflate, hybrid free entries. lamina needs a written *reader tolerance policy* (an ADR) with a differential test suite against reference readers.
- **"Repeat all previous trailer keys" (§7.5.6) is wrong after an xref-stream base:** stream-specific keys (`/W /Index /Filter /DecodeParms /Length /XRefStm`) must be dropped. Copying `/DecodeParms` produced corrupt files in one implementation.
- **Hybrid files invert precedence for free entries:** classic in-use entries win, but entries the table marks *free* must be filled from `/XRefStm`. Otherwise every compressed object disappears.
- **Every number read from the file is an attacker-controlled allocation size.** A 4-byte stream claiming a 16-million-column predictor row asked for 1 GiB in both implementations. Bound every count by the bytes actually present.

**The language findings:**

- **Rust gave stronger guarantees for less review effort on safety:** clippy-enforced no-panic library code, overflow-checked arithmetic, and the type system made the review's Rust findings mostly about performance (missing cache) rather than correctness.
- **TypeScript was faster to write and smaller to ship** (~40% fewer lines; 16 KiB vs 74 KiB gzipped). But most of the review's correctness findings were on the TS side, and it depends on fflate's laxer inflater.
- **Rust → WASM → TS works with no glue library:** a 4-function C ABI (`alloc`, `dealloc`, `hello_pdf`, `page_count`) was enough to round-trip bytes. A real binding would use wasm-bindgen (web) and UniFFI (Swift/Kotlin for mobile) over the same core. The spike didn't test those generators.
- **Mobile:** the Rust core type-checks for iOS and Android with zero code changes, because the library is pure. It wasn't linked into an app; an Xcode/NDK build is the next step. The TS core runs on React Native/Hermes ≥ 0.70 (BigInt is needed for exact rounding).
- **The WASM size (74 KB gzipped) is mostly the inflater and formatting machinery.** It's acceptable for a document library but worth tracking. `wasm-opt` and dropping `fmt` from hot paths are the obvious next levers.

## 5. Implications for lamina (recommendation, for discussion)

The spike **supports** chapter 05's recommendation: **one Rust core**, exposed as WASM + TypeScript for web, native for desktop, and UniFFI/C-ABI for iOS and Android. A pure-TS implementation is viable for a small web-only library, but it would have to re-implement and re-verify every safety property the Rust compiler and clippy enforced here, separately in each language.

Concrete follow-ups:

1. **ADR: core language and binding strategy** (Rust core; wasm-bindgen for web/Node; UniFFI for Swift/Kotlin; C ABI for others).
2. **ADR: reader tolerance policy.** Codify the decisions from §4 and the spike READMEs, and keep a differential test against pdf.js, PDFium and pypdf.
3. **Keep the shared-fixture, two-implementation test style:** golden byte-identical files, a pypdf-derived corpus and a differential fuzz caught real bugs that single-implementation tests missed.
4. **Next spike:** link the Rust core into a minimal iOS app (UniFFI) and an Android app, and run the TS tests in a real browser (Playwright). Those are the two platform claims this spike only type-checked.
