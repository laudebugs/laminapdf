# Ship one hardened core through five front ends

lamina should build one handle-based Rust façade and keep it strict enough for UniFFI's rules from day one: `&self` methods only, `Send + Sync` handles, no generics, no borrowed returns, explicit cancellation. Thin edge crates then expose it as `@lamina/pdf` (wasm-bindgen + tsify), Swift and Kotlin SDKs (UniFFI), and a `lamina` CLI that doubles as an MCP server. The opening is real. In October 2026 no permissively licensed library covers parse, edit, incremental save, sign and render in one engine across web, Node, iOS and Android. pdf-lib has not shipped a release since November 2021. LibPDF is the strongest TypeScript successor, but it is beta and does not render. hayro is the best pure-Rust renderer, but it does not edit. The permissive CLIs stop at structure, because text extraction and rendering on the command line mean AGPL or GPL tools. What decides adoption in all three audiences is the contract, more than the feature list: one coordinate convention, one error shape with stable codes, versioned JSON schemas, deterministic output, and docs that ship with the package. Those choices serve a frontend engineer, a backend script and an LLM agent at the same time. The accepted decisions stand, but the evidence presses on three planned choices, which this report raises as open questions: the 350 KB gzip budget, the "UniFFI later" timing (ADR 0002) and the PDFium-default rendering plan (proposed ADR 0004). The last section turns all of this into proposed work items and ADRs.

Everything labelled **Recommendation** below is the author's proposal, not existing lamina code or an accepted decision. Claims marked **[unverified]** come from the researchers' prior knowledge and were not re-checked against a source in this round.

## Developers leave PDF tools over stalls, malformed files and licences

### The permissive field is fragmented, and every library is strong in only one area

The JavaScript ecosystem lost its default library. **pdf-lib's last release is 1.17.1 from 6 November 2021**, published by one npm maintainer ([npm](https://registry.npmjs.org/pdf-lib)). Its repository carries **279 open issues and 39 open PRs** ([GitHub](https://github.com/Hopding/pdf-lib)), and Snyk rates it "Inactive" ([Snyk](https://snyk.io/advisor/npm-package/pdf-lib)). Work has split across forks that are each scoped to a sponsor. @cantoo/pdf-lib (2.11.1, September 2026) states plainly that it "cannot guarantee the support for issues too far from our own roadmap" ([README](https://unpkg.com/@cantoo/pdf-lib@2.4.1/README.md)). pdfme ships its own fork ([npm](https://registry.npmjs.org/@pdfme/pdf-lib)). **LibPDF (@libpdf/core 0.5.3)** is a full rewrite backed by Documenso. It claims lenient parsing, RC4/AES encryption, PAdES B-B to B-LTA signing, forms, merge and split, and signature-preserving incremental saves. It is labelled "Beta Software" whose APIs "may change between minor versions". It does not verify signatures, does not support TTC fonts, and passes JBIG2 and JPX images through without decoding them ([GitHub](https://github.com/LibPDF-js/core)). It also saves incrementally only when asked with `save({ incremental: true })`, so a default save invalidates signatures ([docs](https://libpdf.documenso.com/docs/concepts/incremental-saves)). LibPDF's own README sums up why people move on: pdf-lib "chokes on slightly malformed documents", and PDF.js renders well but is built around the browser ([GitHub](https://github.com/LibPDF-js/core)).

Rust is busier but just as specialised. **hayro 0.8.0** (MIT/Apache, October 2026) has a visual regression suite of more than 1,600 PDFs and claims the highest correctness among Rust renderers. Editing is out of scope, its API is unsettled, and untrusted input can still cause timeouts and OOM ([GitHub](https://github.com/LaurenzV/hayro)). lopdf is where Rust editing usage concentrates, with 13.2M downloads in 90 days ([crates.io](https://crates.io/api/v1/crates/lopdf)). krilla generates PDFs but cannot write into an existing object graph, which lamina's own `krilla-fit` spike confirmed. There is also a bus-factor risk: Laurenz Stampfl is central to hayro, krilla and typst-pdf. Only PDF.js and qpdf have a long-standing institutional base.

Mobile developers face a different trade. **Apple's PDFKit has iOS 26 and 27 regressions that app developers cannot fix.** Reported problems include `removeAnnotation` stopping working, page contents appearing swapped, memory spikes when zooming with many annotations, and a tile-rendering crash in a new `PDFPageAnalyzerV2` that cannot be disabled ([804551](https://developer.apple.com/forums/thread/804551), [825803](https://developer.apple.com/forums/thread/825803), [847155](https://developer.apple.com/forums/thread/847155)). On Android, AndroidPdfViewer ships **"almost 16 MB" of native PDFium libraries**. Its fix for the 16 KB page alignment Google Play now requires exists only in a beta ([GitHub](https://github.com/barteksc/AndroidPdfViewer)). Commercial SDKs fill the gap at a price. Vendr reports a **median Apryse contract of $25,140 a year** ([Vendr](https://www.vendr.com/marketplace/apryse)), and Nutrient quotes privately ([G2](https://www.g2.com/products/pdf-sdk/pricing)).

On the command line, **qpdf is the only full-featured permissive manipulation tool** (Apache-2.0, [qpdf](https://qpdf.sourceforge.io)). mutool is AGPL ([Linux Pro Magazine, 2014](https://www.linuxpromagazine.com/Issues/2014/160/PDF-Tools/(offset)/6/(tagID)/155)) and poppler-utils is GPL ([CERN repoview](https://linuxsoft.cern.ch/scientific/7x/x86_64/updates/fastbugs/repoview/poppler-utils.html)). Text extraction and rendering in a shell script therefore pull in copyleft today. AI extractors are messier still. pymupdf4llm is AGPL ([file2markdown](https://www.file2markdown.ai/blog/is-pymupdf-free-for-commercial-use)). MinerU's licence is "based on Apache 2.0 with additional conditions" ([GitHub](https://github.com/opendatalab/MinerU)). Marker's Surya weights carry revenue thresholds ([Fossies MODEL_LICENSE](https://fossies.org/linux/marker/MODEL_LICENSE)).

| Why developers leave | Example | What lamina must do instead |
|---|---|---|
| Maintenance stall, single maintainer | pdf-lib (last release 2021) | Institutional cadence, lockstep releases (ADR 0017), more than one publisher |
| Fails on malformed files | pdf-lib per LibPDF README | ADR 0003 tolerance with coded warnings |
| Default save breaks signatures | LibPDF full save | ADR 0007 save planner: `AppendOnly` is automatic when signatures exist |
| OS-version regressions the app cannot fix | PDFKit on iOS 26/27 | Own the renderer and the viewer, versioned with the app |
| Native binary weight, ABI churn | AndroidPdfViewer, about 16 MB | Size-budgeted Rust core, arm64-first, 16 KB-aligned `.so` |
| Price and opacity | Apryse, Nutrient | MIT OR Apache-2.0 (ADR 0001) |
| Copyleft for extraction and rendering | mutool, poppler, PyMuPDF | One permissive CLI covering structure, text and render |

### Conflict to verify: the state of androidx.pdf

The research notes disagree about Google's first-party Android library. The ecosystem note says the latest release is **1.0.0-alpha19 (1 July 2026)** with no beta ([release notes](https://developer.android.com/jetpack/androidx/releases/pdf)). The mobile note, citing the same page, lists **1.0.0-beta01 on 26 August 2026** and gives its contents in detail. In beta01, public APIs are promoted to Beta, but `PdfViewer` (the Compose viewer), `EditablePdfViewerFragment` and `OcrProvider` remain under `@ExperimentalPdfApi` ([release notes](https://developer.android.com/jetpack/androidx/releases/pdf)). **beta01 is more likely current.** It is later, its entry is specific (artifact names, the annotation it introduces, the new OCR module), and the alpha19 reading probably came from a stale or localised copy of the page, since the ecosystem note itself cited locale variants. Either way the strategic point holds: Android now has a free first-party viewer with forms, ink and OCR on minSdk 28+, and its Compose and editing surfaces are still experimental. Check the English release-notes page before this lands in a plan. A smaller conflict on the same page: one note puts `EditablePdfViewerFragment` at alpha13 and another at alpha18. The English notes say alpha13.

### What "one contract" looks like across surfaces

Each audience expects a one-liner for the common job: open a file leniently, fill a form, save without breaking signatures. LibPDF's `pdf.getForm().fill({...})` and `PDF.merge([...])` set the bar for TypeScript ([GitHub](https://github.com/LibPDF-js/core)).

**Recommendation: the same task on three surfaces, with the same names, codes and defaults**

```rust
// Rust (lamina façade)
let doc = lamina::Document::open(bytes, OpenOptions::default())?;   // tolerant; warnings collected
doc.form().fill(&[("name", "Jane".into()), ("agreed", true.into())])?;
let out = doc.save(SaveOptions::auto())?;   // planner picks incremental when signatures exist
for w in doc.warnings() { eprintln!("{} {}", w.code, w.message); }
```

```ts
// TypeScript (@lamina/pdf, sync after init)
await init();
using doc = Document.open(bytes);
doc.form.fill({ name: "Jane", agreed: true });
const out = doc.save();                     // { mode: "auto" } default → incremental when signed
doc.warnings.forEach(w => console.warn(w.code, w.message));
```

```console
# CLI
$ lamina fill form.pdf --data '{"name":"Jane","agreed":true}' -o filled.pdf --json
{"schema":"lamina.fill/1","output":"filled.pdf","save":"incremental","warnings":[]}
```

## A UniFFI-shaped façade keeps one core honest across WASM and native

### Design the façade to the stricter binding now

ADR 0002 already requires a handle-based façade designed once, with UniFFI "later" in `lamina-ffi`. The 2026 evidence argues for applying UniFFI's rules to the façade **now**, while B5c is still unwritten, even if the binding comes later. **UniFFI objects live in an `Arc`, methods take `&self` only, `&mut self` is disallowed, and the implementation must be `Send + Sync`** ([UniFFI manual: interfaces](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/types/interfaces.md)). wasm-bindgen classes have no such constraint, so a façade written only for wasm would need reshaping later. No surveyed project gets both bindings from one attribute set. Bitwarden and Matrix share a core whose data types carry feature-gated derives, and put handles and methods in **separate thin leaf crates per binding** ([bitwarden_uniffi](https://sdk-api-docs.bitwarden.com/bitwarden_uniffi/index.html), [matrix-rust-sdk bindings](https://github.com/matrix-org/matrix-rust-sdk/tree/main/bindings)). Bitwarden also found that neither generator handles generics and added non-generic wrappers ([PR #488](https://github.com/bitwarden/sdk-internal/pull/488)). That matches ADR 0002 §3, which already rules out generics at the boundary.

The UniFFI version is a real trade-off. **UniFFI 0.32.2 (September 2026) adds zero-copy `&[u8]` arguments into Rust**, though not in async functions ([CHANGELOG](https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md)). That matters when a 50 MB PDF crosses the boundary. But uniffi-bindgen-react-native, the route to React Native, is pinned to `=0.31` and "doesn't yet support UniFFI 0.32" ([ubrn releases](https://github.com/jhugman/uniffi-bindgen-react-native/releases)). ADR 0002 already says to pin UniFFI and upgrade deliberately. The open choice is which version to pin at.

**Recommendation: façade, wasm edge and UniFFI edge (Rust)**

```rust
// crates/lamina (façade; no wasm-bindgen or uniffi dependency in the logic)
#[derive(Clone)] pub struct Document { inner: Arc<RwLock<Session>> }   // Send + Sync, cheap clone
#[derive(Clone, Debug)]
#[cfg_attr(feature = "tsify",  derive(serde::Serialize, tsify::Tsify), tsify(into_wasm_abi))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PageInfo { pub index: u32, pub width_pt: f32, pub height_pt: f32, pub rotation: u16 }

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
pub enum LaminaError {                     // codes from fixtures/codes.json; string payloads only
    #[error("{message}")] Coded { code: String, message: String, hint: Option<String>, offset: Option<u64> },
    #[error("cancelled")] Cancelled,
}
pub struct PageRef { doc: Document, index: u32 }   // owned handle, never Page<'doc>
pub struct CancelToken(Arc<AtomicBool>);           // polled by long loops

// crates/lamina-wasm (edge)
#[wasm_bindgen(js_name = Document)] pub struct WasmDocument(lamina::Document);
#[wasm_bindgen(js_class = Document)]
impl WasmDocument {
    pub fn open(bytes: &[u8], opts: JsOpenOptions) -> Result<WasmDocument, JsError> { … }
    #[wasm_bindgen(js_name = pageInfo)] pub fn page_info(&self, i: u32) -> Result<lamina::PageInfo, JsError> { … }
}

// crates/lamina-ffi (edge)
#[derive(uniffi::Object)] pub struct FfiDocument(lamina::Document);
#[uniffi::export] impl FfiDocument {
    #[uniffi::constructor] pub fn open(bytes: Vec<u8>, opts: OpenOptions) -> Result<Arc<Self>, LaminaError> { … }
    pub fn page_info(&self, i: u32) -> Result<PageInfo, LaminaError> { … }
}
```

The wrappers are mechanical. A small `macro_rules!` or proc-macro can generate both, the way Bitwarden and Matrix each keep a `*-ffi-macros` crate.

### The 350 KB budget fits parse, edit and save only

The researchers downloaded published packages and gzipped the `.wasm` files. Comparable Rust WASM libraries sit well above lamina's budget once they render or shape text: **typst-ts-renderer 355 KB, photon 669 KB, resvg-wasm 949 KB, matrix-sdk-crypto-wasm 2.1 MB, and PDFium compiled to WASM (@embedpdf/pdfium) 2.15 MB** ([typst-ts-renderer](https://registry.npmjs.org/@myriaddreamin/typst-ts-renderer/-/typst-ts-renderer-0.7.0.tgz), [resvg](https://registry.npmjs.org/@resvg/resvg-wasm/-/resvg-wasm-2.6.2.tgz), [embedpdf](https://registry.npmjs.org/@embedpdf/pdfium/-/pdfium-2.15.1.tgz)). lamina's own krilla-fit spike already measured **377 KB gz for the builder stack before any shaper**, which is why B6a plans a split base/text module. The research supports that split and goes further: render, shaping, signing and JPX/JBIG2 decoding should each be a Cargo feature and a separately loaded `.wasm`. Separate wasm-bindgen modules do not share memory, so documents cross between them as bytes. `wasm-split` (0.8.0-alpha, [crates.io](https://crates.io/crates/wasm-split-cli)) could split one build into lazy chunks that share memory, but it is too new to rely on. The standard size techniques are `opt-level="z"`, fat LTO, `codegen-units=1`, `panic=abort`, `wasm-opt -Oz`, and keeping `core::fmt` out of hot error paths **[unverified]**. `twiggy` diffs on PRs make regressions visible ([crates.io](https://crates.io/crates/twiggy)).

### Bytes cross once, ranges are pulled, and the core stays synchronous

On the web, a `Uint8Array` argument is always copied into wasm linear memory. A `save()` result has to be copied out too, because a view into linear memory is invalidated when the memory grows. PDF.js's loading model is the one to learn from: range requests, streaming and optional auto-fetch, which pay off fully only for linearized files ([pdfjs-dist api.d.ts](https://app.unpkg.com/pdfjs-dist@4.1.392/files/types/src/display/api.d.ts), [Nutrient on linearized downloads](https://www.nutrient.io/guides/web/performance/linearized-downloads.md)). The recommended shape for lamina keeps the core synchronous and bytes-in/bytes-out, in line with ADR 0001. The ADR 0005 `ByteSource` gets a fallible range read that can return `NeedData { ranges }`. The edge fetches those ranges and retries, the way PDFium's `FPDF_AVAIL` API works **[unverified]**. This is the windowed-reading path B2d already plans. The save planner then gives a second win: an incremental save is the original bytes plus an appended section, so the façade can return only the appended part and let the host concatenate. That avoids copying a large original back across FFI.

### Cancellation and progress belong in lamina's API, not the bindings'

Neither boundary gives cancellation for free. **UniFFI says "there's no builtin way to cancel a future"**, so libraries must supply their own flag ([UniFFI futures.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md)). Single-threaded wasm cannot run a JavaScript `cancel()` while a synchronous call is busy. ubrn's `forceAsync` "moves no work off the main thread" ([ubrn releases](https://github.com/jhugman/uniffi-bindgen-react-native/releases)). Wasm threads need SharedArrayBuffer and therefore COOP/COEP headers, which a library embedded in other people's pages cannot demand **[unverified; MDN]**. Draft ADR 0009 in C2a already defaults to no COOP/COEP. The pattern that works on every target is a **resumable job handle**. `job.step(budget)` returns progress or a result, and the core polls a `CancelToken` inside long loops. On native, parallelism such as rayon per page stays in edge or app crates, which the portability rules already require.

**Recommendation: one job model across surfaces**

```rust
// Rust
let job = doc.render_job(page, RenderOptions { scale: 2.0, ..Default::default() })?;
loop { match job.step(Budget::ops(50_000))? { Step::Progress(p) => report(p), Step::Done(bmp) => break bmp } }
// any thread: job.cancel();   → next step() returns Err(LaminaError::Cancelled)
```

```ts
// TypeScript (worker client wraps step() and AbortSignal)
const ac = new AbortController();
const bmp = await doc.page(0).render({ scale: 2, signal: ac.signal, onProgress: p => (bar.value = p.ratio) });
```

```console
# CLI: the same job loop with a timeout and progress on stderr (only when stderr is a TTY)
$ lamina render big.pdf --pages 1 --scale 2 --timeout 10s -o p1.png
```

### Rendering: hayro is the small-footprint candidate; PDFium wins on fidelity at about 2 MB

**hayro 0.8.0 renders on the CPU through `vello_cpu`, emits SVG, and has its own JPX, JBIG2 and CCITT decoders.** It lacks knockout groups, CMYK blending and glyph caching ([GitHub](https://github.com/LaurenzV/hayro), [deps](https://crates.io/api/v1/crates/hayro/0.8.0/dependencies)). It has no published wasm build, so its size is unknown. Its parser, hayro-syntax, is separate from `lamina-cos`, so lamina would either parse twice or adapt at hayro-interpret's `Device` trait. PDFium through `pdfium-render 0.9.4` has the best fidelity. It costs about **2.15 MB gz** on the web, or a native library per platform, and brings C++ memory-safety exposure ([crates.io](https://crates.io/api/v1/crates/pdfium-render/0.9.4/dependencies), [embedpdf](https://registry.npmjs.org/@embedpdf/pdfium/-/pdfium-2.15.1.tgz)). lamina's plan (proposed ADR 0004) makes PDFium the default and hayro the CI oracle, and the C1a hayro-parity and C1b pdfium-wasm spikes will measure both. The research does not overturn that plan. It does add two inputs. On mobile, PDFium means shipping the C++ binary weight that pushes developers away from AndroidPdfViewer. And a hayro wasm build may be the only renderer small enough for an edge-runtime `@lamina/pdf/render`. ADR 0004 should therefore decide per surface rather than once for all.

### Signing: prepare, hand off the digest, finish

The RustCrypto stack (`cms` 0.2.3 with 0.3.0-rc, `x509-cert` 0.3.0, `rsa`, `p256`, `ecdsa`) is pure Rust and the realistic choice for a wasm-compatible CMS builder ([docs.rs cms](https://docs.rs/cms), [crates.io rsa](https://crates.io/crates/rsa)). Its stable and release-candidate versions churn, so it should be pinned as one generation inside a `sign` feature. WebCrypto's `subtle.sign` is asynchronous, and the Secure Enclave, HSMs and remote signers sit behind platform or network calls. A **two-phase API** therefore keeps the core synchronous: prepare reserves `/Contents` and returns the bytes to sign, the host signs, and finish embeds the CMS. One detail to handle: WebCrypto ECDSA returns raw `r||s`, which must be DER-encoded for CMS. This fits the E10d `Signer` trait and the E10i platform signers.

**Recommendation: signing on three surfaces**

```rust
let pending = doc.prepare_signature(SignatureRequest { field: "Sig1".into(), level: Pades::BB, placeholder: 16_384 })?;
let sig = my_hsm.sign(pending.data_to_sign())?;              // keys never enter lamina
let signed: Vec<u8> = pending.finish(sig, chain, None)?;      // incremental append (ADR 0007)
```

```ts
const pending = doc.prepareSignature({ field: "Sig1", level: "B-B" });
const raw = await crypto.subtle.sign({ name: "ECDSA", hash: "SHA-256" }, key, pending.dataToSign);
const signed = pending.finish({ signature: raw, format: "p1363", chain });   // lamina DER-encodes
```

```console
$ lamina sign in.pdf --field Sig1 --p12 me.p12 --password-file pw.txt -o signed.pdf
$ lamina sign in.pdf --field Sig1 --external 'pkcs11-tool …' -o signed.pdf   # digest piped to an external signer
```

### Panic safety: no panics in core, a disposable instance on the web, unwinding at native edges

On stable wasm32 a panic aborts the instance. Since April 2026, wasm-bindgen can turn panics into a JavaScript `PanicError` and keep the instance usable. That needs nightly `-Zbuild-std` with `-Cpanic=unwind` and wasm exception handling, and OOM still cannot unwind ([Cloudflare](https://blog.cloudflare.com/making-rust-workers-reliable)). A March 2026 issue reported unwinding builds aborting at the boundary, and its fix is unconfirmed ([mirror of wasm-bindgen #4796](https://hub.harborframework.com/tasks/abundant/wasm-bindgen__wasm-bindgen-4796)). The policy should stay "no panics in core", enforced by lamina's lints, fuzzing and `Limits`, with `no-panic` on hot leaf functions where it is feasible ([crates.io](https://crates.io/crates/no-panic)). The TypeScript wrapper treats the instance as disposable: on `RuntimeError: unreachable` it reinstantiates and reopens from the original bytes. UniFFI scaffolding catches panics only under `panic=unwind` **[unverified]**, so the iOS and Android profiles must not inherit `wasm-release`'s `panic=abort`.

## The TypeScript package wins on loading, disposal and error codes

### One `.wasm` file, resolved per export condition, with an escape hatch

No single wasm loading mechanism works everywhere in October 2026. **Node unflagged Wasm ESM imports in v24.5.0 and v22.19.0**, but source-phase imports are still "release candidate" ([Node ESM docs](https://nodejs.org/api/esm.html)). wasm-bindgen's new `module` target uses `import source`, but it is "currently only supported in ESBuild and Node.js 24" ([wasm-bindgen deployment guide](https://raw.githubusercontent.com/wasm-bindgen/wasm-bindgen/main/guide/src/reference/deployment.md)). Cloudflare Workers bundle `.wasm` imports and recommend instantiating at the top level ([Cloudflare docs source](https://raw.githubusercontent.com/cloudflare/cloudflare-docs/production/src/content/docs/workers/runtime-apis/webassembly/javascript.mdx)). They also block compiling from raw bytes **[unverified]**, which rules out inline base64. Vite's dependency pre-bundling breaks `new URL(..., import.meta.url)` wasm resolution unless the package is excluded from `optimizeDeps` ([jSquash README](https://github.com/jamsinclair/jSquash)). ADR 0002 already requires that `init()` accept a URL or a precompiled `WebAssembly.Module`, and that the same file run everywhere. The research fills in the per-condition loaders that ADR 0016 should record.

**Recommendation: export map and init (package.json and TS)**

```jsonc
"sideEffects": false,
"exports": {
  ".": { "types": "./dist/index.d.ts",
         "workerd": "./dist/workerd.js",   // import wasm from "./lamina_bg.wasm"
         "node":    "./dist/node.js",      // readFile(new URL(..., import.meta.url))
         "deno":    "./dist/web.js", "browser": "./dist/web.js", "default": "./dist/web.js" },
  "./render": { … }, "./sign": { … }, "./extract": { … }, "./worker": { … },
  "./lamina_bg.wasm": "./dist/lamina_bg.wasm",      // self-host or pass to init()
  "./inline": "./dist/inline.js"                     // opt-in base64; not for workerd
}
```

```ts
await init();                                              // idempotent; no top-level await on import
await init({ module: await WebAssembly.compileStreaming(fetch("/lamina.wasm")) });  // escape hatch
```

Public signatures take `Uint8Array` (and so accept Node `Buffer` for free) and return `Uint8Array<ArrayBuffer>`, never `Buffer`. Streams are WHATWG `ReadableStream<Uint8Array>`, which is a global in every target runtime **[unverified]**.

### The worker client is the async twin of the sync API, and it should be shared with the viewer

PDF.js shows what a mandatory worker costs: users must set `GlobalWorkerOptions.workerSrc`, and the worker must be "the _exact_ same version" as the main library ([pdf.js FAQ](https://github.com/mozilla/pdf.js/wiki/Frequently-Asked-Questions)). unpdf goes the other way and inlines the worker, because serverless runtimes "can't load separate worker files" ([unpdf README](https://github.com/unjs/unpdf)). The recommendation keeps ADR 0002's synchronous core and adds an optional `@lamina/pdf/worker`. It exposes the same method names as Promises, typed by a mapped type generated from the sync interface. It transfers ArrayBuffers both ways, takes `AbortSignal` and `onProgress`, and does a version handshake on connect. Vite only detects a worker when `new URL()` sits literally inside `new Worker()` ([Vite features](https://raw.githubusercontent.com/vitejs/vite/main/docs/guide/features.md)), so the package should contain that literal and also accept `{ worker }` for CSP-restricted setups. lamina's C3a already plans exactly this kind of hand-written typed RPC (request IDs, priorities, transfer lists, cancellation, crash restart, no Comlink) inside `@lamina/viewer`. **Building it once in `@lamina/pdf/worker` and having the viewer consume it** avoids two protocols.

### Tree-shaking cannot remove wasm code, so subpaths must mean separate binaries

JavaScript tree-shaking drops unused glue, not unused wasm functions. jSquash's one-package-per-codec layout is the working precedent ([jSquash README](https://github.com/jamsinclair/jSquash)). `@lamina/pdf` should load the base module, and `/extract`, `/render` and `/sign` should each load their own `.wasm` on `init()`. The cost is duplicated core code in each binary, which twiggy should measure in B6a.

### Disposal: `using` first, FinalizationRegistry as a backstop, memory bounded three ways

Explicit resource management shipped in Chromium 134, Firefox 141 and Node 24 ([V8 blog](https://v8.dev/features/explicit-resource-management)). **Safari's status conflicts across sources.** One says desktop Safari 26.4 has `Symbol.dispose` and iOS does not ([OpenReplay](https://blog.openreplay.com/javascript-cleanup/)). Another says it exists only in Technology Preview ([ByteIota](https://byteiota.com/javascripts-using-keyword-is-now-cross-browser-act-now/)). Treat Safari as unsupported until the MDN compatibility table says otherwise, and polyfill `Symbol.dispose ??= Symbol.for("Symbol.dispose")`. wasm-bindgen's FinalizationRegistry cleans up eventually ([weak references guide](https://raw.githubusercontent.com/wasm-bindgen/wasm-bindgen/main/guide/src/reference/weak-references.md)). But wasm linear memory never shrinks **[unverified]**, so a single 500 MB render leaves the instance large. Bound memory three ways: enforce `Limits` in Rust, set a wasm maximum memory at link time so growth fails as a coded error, and offer `reset()` or a terminable worker to reclaim it. A call on a disposed handle should throw `LaminaError` with code `DISPOSED`, not a null-pointer panic from the glue.

### Strict types: branded units, discriminated options, codes generated from one file

Neither pdf-lib nor LibPDF documents error types or codes ([pdf-lib README](https://raw.githubusercontent.com/Hopding/pdf-lib/master/README.md), [LibPDF](https://github.com/LibPDF-js/core)). lamina already has `fixtures/codes.json` checked against the Rust enums, so it can generate a TypeScript string-literal union from the same file. Options should be discriminated unions, so an invalid combination is a type error. Units should be branded types. `publint` and `attw --profile esm-only` belong in `cargo xtask check ts` **[unverified tool behaviour]**.

**Recommendation: types on three surfaces**

```ts
export type Pt = number & { readonly [unit]: "pt" };
export const mm = (n: number) => (n * 72 / 25.4) as Pt;
export type ErrorCode = (typeof codes)[number]["code"];          // generated from fixtures/codes.json
export class LaminaError<C extends ErrorCode = ErrorCode> extends Error {
  readonly code: C; readonly hint?: string; readonly offset?: number; readonly docsUrl: string;
}
export type SaveOptions = { mode: "auto" } | { mode: "incremental" } | { mode: "full"; objectStreams?: boolean };
if (isLaminaError(e, "WRONG_PASSWORD")) askAgain();
```

```rust
#[derive(serde::Deserialize, tsify::Tsify)] #[serde(tag = "mode", rename_all = "kebab-case")]
pub enum SaveOptions { Auto, Incremental, Full { object_streams: Option<bool> } }   // same union, one source
```

```console
$ npx @lamina/cli info a.pdf --json      # CLI shipped the biome way: meta package + per-platform optionalDependencies
```

The npm route for the CLI follows `@biomejs/biome`: a meta package whose `optionalDependencies` point at per-platform binaries ([biome package.json](https://raw.githubusercontent.com/biomejs/biome/main/packages/%40biomejs/biome/package.json)). The CI matrix B6c plans (Vitest browser mode, workerd via vitest-pool-workers, Deno and Bun smoke tests against the packed tarball, and bundler fixture apps) matches what jSquash and unpdf use as integration checks ([unpdf README](https://github.com/unjs/unpdf)).

## Mobile SDKs must beat free platform viewers on parity, fidelity and editing

### The baseline moved: free viewers now annotate and fill forms

androidx.pdf (see the conflict above) now offers a Compose `PdfViewer`, `EditablePdfViewerFragment` with ink through `androidx.ink`, form filling with autofill, streamed search and an ML Kit OCR provider, backported to **minSdk 28, "an additional 2 billion active Android devices"** ([release notes](https://developer.android.com/jetpack/androidx/releases/pdf)). The framework `PdfRenderer` gained form widgets, search and selection in API 35, with `PdfRendererPreV` covering API 30–34 ([PdfRenderer.Page](https://developer.android.com/reference/android/graphics/pdf/PdfRenderer.Page)). On iOS, PDFKit has not added significant API since the iOS 16 overlay-view and PencilKit hooks ([WWDC22 10089](https://developer.apple.com/videos/play/wwdc2022/10089/)). Developers report that the overlay pattern is hard to make work, and Apple published no sample for it ([forums 716766](https://developer.apple.com/forums/thread/716766)). The ergonomic bar is Nutrient's SwiftUI `PDFView(document:)`, a native `View` with toolbar state scoped through `@PDFView.Scope` ([Nutrient SwiftUI guide](https://www.nutrient.io/guides/ios/getting-started/swiftui/)). Even Nutrient fixes page-recycling crashes on iOS 26 and 27 ([changelog](https://www.nutrient.io/guides/ios/changelog/)). lamina's mobile value therefore lies in what the platform viewers lack: identical behaviour on both platforms, tolerance of malformed files (ADR 0003), incremental save (ADR 0007), undo through the op log (ADR 0006), signing, redaction, generation, and a Compose API that is not experimental.

### Tiles, a page-size table and caller-owned pixels

The common pattern renders a low-resolution whole-page bitmap first, then high-resolution tiles for the visible region, on background threads, with off-screen jobs cancelled. PDFKit renders by tiles internally ([forums 825803](https://origin-devforums.apple.com/forums/thread/825803)). On Compose, Telephoto's subsampling `ZoomableImage` is the reference implementation (Apache-2.0, [klibs.io](https://klibs.io/project/saket/telephoto)). The Rust primitive should take a page-space rectangle and a scale, and write RGBA into a caller-owned buffer. A page-size table must be available as soon as the document opens, so a list of 1,000+ pages can be laid out without rendering anything.

### Memory: size caches from the live budget and shed on warnings

iOS publishes no per-device memory limits. `os_proc_available_memory()` is the only live signal ([Apple docs](https://developer.apple.com/documentation/os/os_proc_available_memory)). One developer reports that **iPadOS 26.4 cut per-app memory on an 8 GB iPad from 6 GB to 3 GB** without documentation ([forums 826325](https://developer.apple.com/forums/thread/826325)). App extensions have their own lower limits ([forums 852862](https://developer.apple.com/forums/thread/852862)). The SDK should keep no decoded bitmaps in Rust by default. The platform layer owns pixel buffers and the cache, and Rust exposes `trim(level)`, wired to `didReceiveMemoryWarningNotification` and `onTrimMemory`. Object caches bounded by `Limits` keep memory proportional to visible pages, not to the document.

### Stylus: accept platform strokes, write `/Ink` with appearance streams

Android has a complete first-party stylus stack. **androidx.ink 1.0 is stable** ([releases](https://developer.android.com/jetpack/androidx/releases/ink)), motion prediction 1.0 is stable ([androidx.input](https://developer.android.com/jetpack/androidx/releases/input)), and front-buffered low-latency rendering exists but is "not intended" for panning and zooming ([Android Developers](https://medium.com/androiddevelopers/stylus-low-latency-d4a140a9c982)). On iOS, PencilKit over `PDFPageOverlayViewProvider` is the sanctioned route ([WWDC22](https://developer.apple.com/videos/play/wwdc2022/10089/)). lamina should not build an ink engine. It should accept stroke points, write `/InkList` plus an `/AP` appearance stream (`/InkList` carries no pressure, so variable width has to live in the appearance), and keep the native stroke data under a private key so strokes can be re-edited without loss. The E4d ink-appearance work is the natural home.

### Concurrency: generated bindings need a hand-written idiomatic layer

UniFFI maps Rust `async fn` to Swift `async` and Kotlin `suspend` ([futures.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md)). It has only "partial support for Swift 6": all generated classes are marked `@unchecked Sendable` ([PR #2318](https://github.com/mozilla/uniffi-rs/pull/2318), [issue #2046](https://github.com/mozilla/uniffi-rs/issues/2046)). That marking is safe only if lamina's handles really are `Send + Sync`, which is one more reason to shape the façade now. Swift cancellation goes through `withTaskCancellationHandler` calling `job.cancel()`. Kotlin can rely on coroutine cancellation plus a Rust-side `Drop` that sets the flag. How UniFFI 0.30–0.32 drops the future on Kotlin cancellation was not verified.

### Packaging: SPM only, AARs on Maven Central

**CocoaPods trunk becomes read-only on 2 December 2026** ([CocoaPods blog](https://blog.cocoapods.org/CocoaPods-Specs-Repo/)). iOS distribution should be a Swift package with a binary XCFramework target plus a source target holding the idiomatic wrapper. `cargo-swift` 0.11.1 automates this ([crates.io](https://crates.io/crates/cargo-swift)). Android gets a `lamina-core` AAR (JNI `.so` for arm64-v8a and x86_64, 16 KB-aligned) and a `lamina-compose` AAR. The Compose path in androidx.pdf already needs AGP 9.2 and compileSdk 37 ([release notes](https://developer.android.com/jetpack/androidx/releases/pdf)), a sign of how quickly Android toolchain floors move. React Native is reachable through ubrn, subject to its UniFFI pin.

**Recommendation: drop-in viewers and headless use**

```swift
// SwiftUI
import Lamina
struct ReaderScreen: View {
    let doc: LaminaDocument                         // try await LaminaDocument.open(url:)
    @State private var viewer = LaminaViewerState() // @Observable: page, zoom, selection, tool
    var body: some View {
        LaminaView(document: doc, state: viewer)
            .laminaTool($viewer.tool)               // .none / .ink(.pen) / .highlight / .form
            .onLaminaSave { data in try data.write(to: url) }   // incremental when required
            .toolbar { LaminaAnnotationToolbar(state: viewer) }
    }
}
let png = try await doc.page(0).render(scale: 2)    // Task cancellation → job.cancel()
for try await hit in doc.search("invoice") { … }    // AsyncThrowingStream<SearchHit, Error>
```

```kotlin
// Compose
@Composable fun ReaderScreen(uri: Uri) {
    val doc by rememberLaminaDocument(uri)          // State<LoadState<LaminaDocument>>
    val state = rememberLaminaViewerState()
    LaminaViewer(document = doc, state = state, tool = LaminaTool.Ink(Brush.Pen), formFillingEnabled = true)
}
scope.launch {
    doc.search("invoice").collect { hit -> … }      // Flow<SearchHit>
    val bmp: Bitmap = doc.page(0).render(scale = 2f) // suspend; cancels the Rust job
}
```

```rust
// Rust primitive both wrappers sit on
pub fn render_tile(&self, page: u32, scale: f32, rect: RectPt, out: &mut [u8], cancel: &CancelToken) -> Result<TileInfo, LaminaError>;
pub fn page_sizes(&self) -> Vec<PageSize>;          // available at open, no rendering
pub fn trim(&self, level: TrimLevel);               // drop decoded images, font and content caches
```

```console
# CLI parity check used in mobile CI: the same tile, byte-compared against the device render
$ lamina render a.pdf --pages 1 --scale 2 --rect 0,0,256,256 -o tile.rgba --format rgba
```

Accessibility remains an open area the notes did not research. The inferred requirement: publish per-page accessibility elements from the structure tree, or from reading order when the file is untagged (§5), and offer a reflow view, because Dynamic Type cannot reflow a fixed layout.

## LLM-first design means guessable names, typed coordinates and provenance-rich output

### A new library is a rare API to every model

Code models hallucinate most on infrequently seen APIs, and a new library is entirely that. **CloudAPIBench found code LLMs struggle most with low-frequency APIs.** Documentation-augmented generation lifted performance on those APIs to 47.94%, but with a weak retriever it hurt frequent ones ([arXiv 2407.09726](https://arxiv.org/html/2407.09726v1)). Version drift is a separate failure: GitChameleon 2.0 baselines reach only 48–51% on version-conditioned problems ([arXiv 2507.12367](https://arxiv.org/abs/2507.12367v1)). lamina therefore needs both an API models can guess without docs and docs that reach the agent when guessing fails.

For guessability, keep the names models expect from pdf-lib, PDF.js and PyMuPDF where the semantics match (`addPage`, `drawText`, `getPage`). Use named options objects. Reject unknown option keys at runtime with a "did you mean" hint, because LLM-written JavaScript often bypasses the types. Never reuse a name with new semantics. Keep deprecated aliases whose warnings name the replacement, and publish renames in a machine-readable changelog. Avoid hidden global state, and make output deterministic (`/ID` derived from content, no timestamps unless asked), so agents can check their work by byte diff. That also fits lamina's golden fixtures.

For docs, the proven channels are an **llms.txt** index ([llmstxt.org](https://llmstxt.org/)) with `.md` twins of every docs page, as Stripe publishes ([Stripe](https://docs.stripe.com/building-with-llms.md)). The second channel is **docs inside the package**: the Vercel AI SDK ships version-matched MDX in its npm tarball, so agents can read them from `node_modules` ([unpkg ai@7.0.90](https://app.unpkg.com/ai@7.0.90/files/docs/00-introduction/index.mdx)). Context7 serves version-specific docs over MCP ([Upstash](https://upstash.com/blog/context7-mcp)). AGENTS.md is plain Markdown, now stewarded by the Linux Foundation's Agentic AI Foundation ([OpenAI](https://openai.com/index/agentic-ai-foundation/)). No controlled study shows that llms.txt itself improves agent correctness; the evidence is for doc retrieval in general. This is cheap to do, so it belongs in B7a rather than in its own round.

### Coordinates must be an explicit, typed field

The bottom-left origin is pdf-lib's best-known footgun: users write `y: height - 150` to place text near the top ([StudyRaid](https://app.studyraid.com/en/read/12496/404105/understanding-pdf-coordinate-systems)). Extraction vendors disagree among themselves. Textract uses ratios of the page from the top-left ([AWS](https://docs.aws.amazon.com/textract/latest/dg/text-location.html)). Azure uses inches for PDFs, with a clockwise polygon ([Microsoft](https://learn.microsoft.com/en-us/python/api/azure-ai-formrecognizer/azure.ai.formrecognizer.boundingregion?view=azure-python)). Docling defaults to top-left but supports both ([docling-core](https://github.com/docling-project/docling-core/blob/main/docling_core/types/doc/base.py)). OpenDataLoader emits `[left,bottom,right,top]` in PDF points ([GitHub](https://github.com/opendataloader-project/opendataloader-pdf)). **Recommendation:** high-level drawing and all extraction output use points with a top-left origin and y pointing down. Page indices are 0-based in the API, and a 1-based `number` is added for humans. Every output records the page box used (CropBox), the `/Rotate` applied and `/UserUnit`. A normalised `bboxNorm` in [0,1] supports highlighting independent of the renderer. A raw `UserSpace` type stays available for spec-faithful content-stream work.

### Errors and schemas as a single contract

rustc's diagnostics are the model: a stable code, a span, labels, and machine-applicable help in `--error-format=json` ([rustc JSON](https://doc.rust-lang.org/rustc/json.html)). lamina's errors should serialise as `{code, kind, message, hint, didYouMean, location:{offset, object}, docsUrl}` from every surface. Every options and output type should emit JSON Schema 2020-12 from the Rust types (schemars), versioned as `lamina.extract/1` and shipped both in the package and at a stable URL.

### Rule-based extraction in core, models as licence-clean plug-ins

Model pipelines lead document-parsing benchmarks. On olmOCR-Bench, Chandra scores 83.1, olmOCR 82.4 and Marker 76.1 ([allenai/olmocr](https://github.com/allenai/olmocr)). MinerU2.5-Pro reaches 95.69 on OmniDocBench v1.6 ([NeurIPS 2026](https://neurips.cc/virtual/2026/poster/139545)). On born-digital PDFs, rule-based extraction is fast and strong on reading order but weak on borderless tables. OpenDataLoader's own benchmark (vendor numbers, 200 PDFs) shows this ([GitHub](https://github.com/opendataloader-project/opendataloader-pdf)):

| Engine | Overall | Reading order | Table TEDS | s/page | Licence |
|---|---|---|---|---|---|
| ODL hybrid (rules + docling models) | 0.907 | 0.934 | 0.928 | 0.463 | Apache-2.0 |
| docling | 0.882 | 0.898 | 0.887 | 0.762 | MIT |
| ODL local (rules only) | 0.831 | 0.902 | 0.489 | 0.015 | Apache-2.0 |
| pymupdf4llm | 0.732 | 0.885 | 0.401 | 0.091 | AGPL-3.0 |

The table points to a clean split. **Rules alone reach about 0.90 reading-order accuracy at 15 ms a page**, and only borderless tables (TEDS 0.49) need a model. Tagged PDF would help, but Adobe reports that **fewer than 20% of PDFs are tagged and fewer than 5% are tagged well** ([pdfa.org slides](https://pdfa.org/wp-content/uploads/2025/10/1-1-11_45-MatthewHardy-Empowering_the_Future_of_PDF_with_AI.pdf)). The structure tree should therefore be the first source of reading order when present and plausible, with XY-Cut++ as the fallback ([arXiv 2504.10258](https://arxiv.org/abs/2504.10258)). The Rust core can do the rest without model weights and stay bytes-in/bytes-out and wasm-friendly. That means:

- headings from font-metric clusters;
- lists from bullets and numbering;
- lattice tables from vector ruling lines, which is easier than Camelot's raster approach because lamina has the paths directly ([pdfplumber](https://github.com/jsvine/pdfplumber#extracting-tables));
- removal of running headers and footers;
- de-hyphenation and ligature normalisation;
- Markdown and JSON serialisers.

Layout detection for complex untagged pages can use Docling's Heron model, which is Apache-2.0 with an ONNX export ([Hugging Face](https://huggingface.co/docling-project/docling-layout-heron/tree/main)), as an optional edge plug-in. OCR stays behind a trait, with Apple Vision and ML Kit at the mobile edges. Avoid MinerU, Marker/Surya weights and PyMuPDF, per ADR 0001. Docling's licence is MIT according to most sources but Apache-2.0 according to one ([Codesota](https://codesota.com/ocr/docling-vs-mineru)). Marker's reported move to Apache-2.0 at v2.0.0 rests on a single blog ([themenonlab](https://themenonlab.blog/blog/best-open-source-pdf-to-markdown-tools-2026)). Check both before relying on them.

Extraction output also has a security side. Extracted text is attacker-controlled, and invisible text (render mode 3, white on white, clipped, off-page) is a prompt-injection vector. Flag it per span (`invisible`, `clipped`, `offPage`, `lowContrast`), list what was dropped and why, and never copy document text into `message` or `hint` fields.

**Recommendation: extraction on three surfaces, one schema**

```rust
let out = doc.extract(ExtractOptions {
    reading_order: ReadingOrder::TaggedThenXyCut,
    coords: Coords { origin: Origin::TopLeft, unit: Unit::Pt, normalized: true },
    drop: Drop::INVISIBLE | Drop::RUNNING_HEADERS | Drop::ARTIFACTS,
    tables: TableMode::Lattice, layout: None, ocr: None, ..Default::default()
})?;
let md = out.to_markdown(MarkdownOptions::default());
for c in out.chunks(ChunkOptions { max_tokens: 512, overlap: 64, split_on: SplitOn::Headings }) {
    index(c.text(), c.provenance());          // pages, regions, heading_path, text_range
}
```

```ts
import { extract } from "@lamina/pdf/extract";
const res = extract(doc, { readingOrder: "tagged-then-xycut", drop: ["invisible", "running-headers"], tables: "lattice" });
for (const c of res.chunks({ maxTokens: 512, splitOn: "headings" }))
  await store.add({ text: c.text, meta: { pages: c.pages, regions: c.regions, path: c.headingPath } });
```

```console
$ lamina extract markdown report.pdf --drop invisible,running-headers > report.md
$ lamina extract json report.pdf --schema extract/1 > out.json
$ lamina extract chunks report.pdf --max-tokens 512 --jsonl | my-embedder
$ lamina explain W_TOUNICODE_GUESSED
```

```json
{ "schema": "https://lamina.dev/schema/extract/1.json",
  "generator": { "name": "lamina", "version": "0.4.0", "mode": "rules", "models": [] },
  "coord": { "unit": "pt", "origin": "top-left", "box": "CropBox", "rotationApplied": true },
  "source": { "sha256": "…", "tagged": true, "structSource": "tagged" },
  "blocks": [ { "id": "b12", "type": "heading", "level": 2, "text": "3.1 Reading order", "page": 0,
                "bbox": [72.0, 96.4, 310.2, 112.0], "bboxNorm": [0.1176, 0.1217, 0.5069, 0.1414],
                "structRef": { "obj": 145, "gen": 0, "mcids": [3, 4] }, "textRange": { "start": 10432, "end": 10449 } },
              { "id": "t3", "type": "table", "detector": "lattice", "page": 1,
                "cells": [ { "row": 0, "col": 0, "header": true, "text": "Tool", "bbox": [72, 200, 180, 220] } ] } ],
  "chunks": [ { "id": "c5", "blockIds": ["b12", "b13"], "headingPath": ["3 Algorithms", "3.1 Reading order"],
                "pages": [0, 1], "tokensApprox": 412 } ],
  "dropped": [ { "reason": "invisible-text", "page": 3, "chars": 2210 } ],
  "warnings": [ { "code": "W_TOUNICODE_GUESSED", "page": 4, "font": "F7" } ] }
```

## The CLI is the primary agent interface, and MCP is a thin adapter over it

### Borrow qpdf's contracts, mutool's verbs and pdftk's page algebra

qpdf is the model for machine contracts. **Its exit codes are 0 (clean), 2 (errors) and 3 (warnings only), and it reserves 1** because the shell uses 1 when it cannot run the command. It has a rich page-range grammar (`r3`, `z`, reverse ranges, `x` exclusions, `:odd`/`:even`), job-JSON input, and versioned JSON whose `--json-help` prints the schema by example. Consumers must ignore unknown keys, and the version is bumped only on breaking changes ([qpdf cli.rst](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/cli.rst), [json.rst](https://raw.githubusercontent.com/qpdf/qpdf/main/manual/json.rst)). Exit code 3 also surprises integrators: Stirling-PDF had to change its wrapper when qpdf repairs exited 3 ([Stirling-PDF commit](https://git.laurivan.com/Mirrors/Stirling-PDF/src/commit/507d21772d212299514f254471008433a64ed3ff)). One weakness to avoid: qpdf does not support reading from stdin. mutool contributes the verb-subcommand shape (`info`, `merge`, `extract`, `show`, `sign`, [mutool.1](https://raw.githubusercontent.com/ArtifexSoftware/mupdf/master/docs/man/mutool.1)). Its grammar can be borrowed even though its AGPL code cannot. pdftk contributes handle-based page algebra (`A=fronts.pdf B=backs.pdf shuffle A Bend-1`, [pdftk man](https://leancrew.com/all-this/man/man1/pdftk.html)). Ghostscript is the cautionary example: interactive unless passed two flags, and safe mode only became the default after sandbox escapes ([Use.rst](https://raw.githubusercontent.com/ArtifexSoftware/ghostpdl/master/doc/src/Use.rst)).

clig.dev states the conventions the CLI should follow. Data goes to stdout and everything else to stderr. `-` means stdin or stdout. Colour and animation turn off when the output is not a TTY or `NO_COLOR` is set. The CLI never prompts without a TTY. It supports `--json`, `--force`, `--dry-run` and `--no-input` ([clig.dev source](https://raw.githubusercontent.com/cli-guidelines/cli-guidelines/main/content/_index.md)). ripgrep's `--json` is the pattern for batch output: per-file `begin`/`end` messages and a final `summary`, one JSON object per line ([ripgrep defs.rs](https://raw.githubusercontent.com/BurntSushi/ripgrep/master/crates/core/flags/defs.rs)).

**Recommendation: exit codes.** `0` ok; `1` generic failure; `2` usage error; `3` completed with warnings (qpdf-compatible; `--warnings-ok` turns it into 0); `4` refused a destructive operation without `--force` or confirmation; `5` password required or wrong. This deliberately differs from gh, where `2` means cancelled and `4` means authentication required ([gh help_topic.go](https://raw.githubusercontent.com/cli/cli/trunk/pkg/cmd/root/help_topic.go)). `lamina help exit-codes` documents the list, and the JSON error carries `exit_code` so wrappers never have to guess.

**Recommendation: grammar, pipes and batch (CLI)**

```console
$ lamina info report.pdf
report.pdf  PDF 1.7 · 42 pages · 1.8 MB · not encrypted · tagged
warning[W0102]: xref offset off by 3 bytes (repaired)
$ echo $?                                                    # 3: success with warnings
$ lamina pages select report.pdf 1-5,z,r3 -o summary.pdf    # qpdf ranges; `end`/`N` accepted as aliases
$ lamina pages shuffle A=fronts.pdf B=backs.pdf A Bz-1 -o duplex.pdf
$ lamina pages split big.pdf --every 10 -o 'parts/{stem}-{n:03}.pdf'
$ curl -sL https://example.org/a.pdf | lamina extract text - --pages 1-3 | rg -i invoice
$ lamina fill form.pdf --data - -o filled.pdf < answers.json
$ lamina check docs/*.pdf --jobs 8 --json | jq -c 'select(.type=="result" and .status!="ok")'
{"type":"result","schema":"lamina.check/1","index":1,"input":"b.pdf","status":"error","error":{"code":"E0201","kind":"password_required"}}
{"type":"summary","ok":41,"warnings":0,"failed":1,"elapsed_ms":412}
$ lamina info report.pdf --json --fields pages,encrypted,warnings   # gh-style field selection keeps tokens down
$ lamina schema extract.json                                         # JSON Schema 2020-12 for input and output
```

Batch mode emits results in argument order even when they finish out of order. It supports `--fail-fast` and `--files-from FILE|-` with `-0` for xargs parity, refuses to overwrite without `--force`, and writes `--in-place` atomically through a temporary file and rename. Passwords come from `--password-file` or an environment variable, never from argv by default.

### MCP 2026-07-28 is stateless, and agents increasingly prefer CLIs

**MCP's current revision, 2026-07-28, removes the `initialize` handshake and protocol sessions, adds a mandatory `server/discover`, moves tasks to an extension, and deprecates Roots, Sampling and Logging.** Servers are told to take directories through tool parameters or configuration instead, and to log to stderr on stdio. `tools/list` SHOULD be deterministic ([2026-07-28 changelog](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2026-07-28/changelog.mdx)). Since 2025-11-25, input validation failures are tool errors (`isError: true`) that the model can correct, rather than protocol errors. Tools can declare an `outputSchema`, and servers "MUST provide structured results that conform" to it ([tools.mdx](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2025-11-25/server/tools.mdx)). Tool annotations default to `destructiveHint: true`, and clients must treat annotations as untrusted ([schema.ts](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/2025-11-25/schema.ts)). Meanwhile Playwright tells users that coding agents "increasingly favor CLI–based workflows exposed as SKILLs over MCP because CLI invocations are more token-efficient" ([playwright-mcp README](https://raw.githubusercontent.com/microsoft/playwright-mcp/main/README.md)). The conclusion: make the CLI the primary agent surface, and make `lamina mcp` a stdio adapter over the same command functions, using the same schemas as `inputSchema` and `outputSchema`. The filesystem MCP server's CVE-2025-53109/53110 came from symlinks and naive path-prefix checks ([OWASP MCP Top 10](https://owasp.org/www-project-mcp-top-10/2025/MCP05-2025%E2%80%93Command-Injection%26Execution)). `lamina mcp` must therefore require `--root`, canonicalise paths, reject symlink escapes, and refuse in-place writes unless started with `--allow-in-place`. github-mcp-server's `--read-only` flag, which "takes priority" over tool selection, is a good precedent ([README](https://raw.githubusercontent.com/github/github-mcp-server/main/README.md)).

**Recommendation: one command layer under clap and rmcp (Rust), consumed from TS**

```rust
// crates/lamina-cli (layer = "app"); tokio/rmcp only behind the `mcp` feature
pub mod commands {
    #[derive(clap::Args, serde::Deserialize, schemars::JsonSchema)] pub struct InfoArgs { pub input: Input, #[arg(long)] pub fields: Vec<String> }
    #[derive(serde::Serialize, schemars::JsonSchema)] pub struct InfoReport { pub schema: &'static str, pub pages: u32, pub encrypted: bool, pub warnings: Vec<Warning> }
    pub fn info(a: InfoArgs) -> Result<InfoReport, CliError> { … }    // the only implementation
}
#[tool(description = "Read page count, encryption, metadata and warnings", annotations(read_only_hint = true))]
async fn lamina_info(&self, Parameters(a): Parameters<commands::InfoArgs>) -> Result<Json<commands::InfoReport>, McpError> {
    self.roots.check(&a.input)?; Ok(Json(commands::info(a)?))
}
```

```ts
// A Node script consuming the CLI's NDJSON with types generated from `lamina schema check.json`
import type { CheckResult, Summary } from "@lamina/cli/schema";
for await (const line of readline.createInterface({ input: child.stdout }))
  { const m = JSON.parse(line) as CheckResult | Summary; if (m.type === "result" && m.status !== "ok") fail(m); }
```

```console
$ lamina mcp --root ./docs --read-only     # stdio; tools in deterministic order: lamina_info, lamina_extract, lamina_render …
```

The current crates are clap 4.6.7, clap_complete, clap_mangen, annotate-snippets 0.12.17 (rustc's renderer, preferred to miette 7.6.0, last released April 2025, for pointing at byte offsets), anstream 1.0, schemars 1.2.3, rmcp 3.8.0 (which implements 2026-07-28) and cargo-dist 0.32.0 ([crates.io](https://crates.io/api/v1/crates/clap), [rmcp README](https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/README.md)). Because the core is bytes-in/bytes-out, a `wasm32-wasip2` build of the CLI could run sandboxed under wasmtime with preopened directories. That idea is a recommendation the notes did not verify.

## Implications for lamina

### Proposed work items

Each item names what it is, why it is needed, and which existing round or todo it depends on. Item IDs are placeholders for the board.

| # | What | Why | Depends on |
|---|---|---|---|
| L1 | **Façade binding rules** in B5c: `Send + Sync` `Arc` handles, `&self` only, owned `PageRef`, no generics, string-payload error enum, `CancelToken`, resumable `Job::step` | UniFFI forbids `&mut self` and needs `Send + Sync`; fixing the façade later breaks the TS API | B5c (amend its acceptance criteria) |
| L2 | **Error and diagnostic contract**: `{code, kind, message, hint, location, docsUrl}` generated from `fixtures/codes.json` into Rust, TS and JSON Schema; `lamina explain <code>` text | One error shape for TS, Swift, Kotlin, CLI and MCP; models self-correct from hints | B2a codes (done), B6a |
| L3 | **Split wasm modules**: base / text / extract / render / sign as separate `.wasm` with per-module budgets and twiggy diffs in CI | Rendering and signing cannot fit 350 KB; tree-shaking cannot remove wasm code | B6a (extends its split), C2c |
| L4 | **Per-condition loaders and `init({ module })` escape hatch** (workerd static import, Node `readFile`, browser `compileStreaming`, opt-in `/inline`) | No single loader works across runtimes in 2026 | B6b, B6c |
| L5 | **`@lamina/pdf/worker`**: the C3a typed RPC moved into the library package, with a version handshake, `AbortSignal`, transfer lists and a Promisified mapped type; `@lamina/viewer` consumes it | Avoids two worker protocols; PDF.js version-mismatch lesson | B6b; replaces part of C3a |
| L6 | **Disposal and memory hardening**: `Symbol.dispose` on all handles, `DISPOSED` code, wasm max-memory link flag, `reset()`, abort-and-reinstantiate wrapper | Linear memory never shrinks; Safari `using` support unclear | B6b |
| L7 | **Range `ByteSource` with `NeedData`** at the edges (HTTP range, JS callback, mmap) | Large and remote files without an async core | B2d |
| L8 | **Two-phase signing API** (`prepare_signature` / `finish`, P1363-to-DER conversion) as the E10d `Signer` shape | WebCrypto, the Secure Enclave and HSMs are async or remote; the core stays sync | E10d, E10i |
| L9 | **`lamina-extract` crate**: reading order (structure tree, then XY-Cut++), headings, lists, lattice tables, header/footer removal, de-hyphenation, visibility flags, Markdown/JSON/chunk serialisers, `lamina.extract/1` schema | Rules reach ~0.90 reading order at 15 ms/page; no permissive Rust equivalent; RAG and agent demand | B9b, B9c, B8a (struct tree reading) |
| L10 | **Model plug-in traits** (`LayoutModel`, `OcrEngine`) with an optional Heron ONNX edge crate | Borderless tables and scans need models; keep weights out of core | L9; E13d (local OCR) |
| L11 | **`lamina-cli`** (layer `app`): command layer, clap front end, qpdf ranges plus pdftk handles, exit codes 0–5, `--json`/NDJSON, `--jobs`, `schema`, `explain`, completions, man pages | First permissive CLI covering structure, text and render | B5c, B9c; grows with B3, C2b, E10d |
| L12 | **`lamina mcp`** behind a feature: rmcp stdio adapter over L11 commands, `--root`, `--read-only`, honest annotations, deterministic tool order | Agents in MCP loops; same schemas as the CLI | L11 |
| L13 | **CLI distribution**: cargo-dist releases, Homebrew, npm meta package with per-platform `optionalDependencies` | `npx` and `brew` reach for backend and agent users | L11; P1 (publishing) |
| L14 | **Agent docs**: `llms.txt` plus `.md` twins, docs shipped inside the npm package and the crate, AGENTS.md snippet in examples, machine-readable renames changelog | New APIs are low-frequency for every model | B7a, B7b |
| L15 | **Coordinates and units types**: `Pt`/`Mm` branded types in TS, `RectPt` with an origin in Rust, top-left in high-level APIs, raw `UserSpace` for low level | Removes the `height - y` footgun; aligns extraction with vendors | B5a, B5b, B9b |
| L16 | **`lamina-ffi` spike**: UniFFI binding of the B5c façade, Swift package via cargo-swift, AAR via cargo-ndk, size report per ABI | Validates L1 early and measures native size before committing to mobile | B5c; UniFFI pin decision |
| L17 | **Mobile SDK plan**: `LaminaView` (SwiftUI) and `LaminaViewer` (Compose) with tiles, `trim()`, stylus-to-`/Ink`, accessibility from the struct tree, SPM and Maven Central | Compete with free platform viewers on parity, fidelity and editing | L16, C2b (render), C3b (tiling ideas), E4d (ink appearances), E8 (forms) |
| L18 | **hayro wasm size measurement** added to C1a | Needed to decide the render module per surface | C1a (ongoing) |

### Proposed ADRs

Numbers 0020–0023 are already reserved in todos (signatures, JavaScript policy, content editing, `lamina-content`). The next free number appears to be **0024**; confirm it on the board before assigning.

| ADR | Decision to record | Relationship to accepted ADRs |
|---|---|---|
| 0024 Error and diagnostic contract | Error JSON shape, `hint` and `docsUrl` rules, no document text in messages, `explain` pages | Extends ADR 0003's codes |
| 0025 Coordinates, units and page indexing in public APIs | Top-left points for high-level and extraction; raw user space for low level; 0-based index plus 1-based number | New; constrains the B5 builder and B9 extraction |
| 0026 Jobs, cancellation and progress across bindings | `CancelToken`, `Job::step(budget)`, progress throttling, no threads in core | Complements ADR 0002; informs proposed ADR 0009 |
| 0027 Mobile bindings and packaging | UniFFI version pin (0.31 for ubrn, or 0.32 for zero-copy), façade rules, idiomatic Swift/Kotlin layer, SPM-only, AAR layout, native panic profile | Fills in ADR 0002 §5; does not change its decision |
| 0028 Extraction output and model plug-ins | `lamina.extract/1` schema, rule-based core, plug-in traits, licence rules for weights | Follows ADR 0001 and proposed ADR 0023 |
| 0029 CLI and MCP contract | Grammar, exit codes, JSON/NDJSON versioning, schema command, MCP security defaults | New; reuses ADR 0024 errors |

### Open questions where research presses on accepted decisions

The first concerns **UniFFI timing** (ADR 0002 §5). The decision stands, but the research suggests bringing forward the façade *rules* and a small `lamina-ffi` spike (L16). Waiting risks a TypeScript API that UniFFI cannot express. The version pin also needs a choice between ubrn compatibility and 0.32's zero-copy bytes.

The second concerns the **350 KB budget** (library plan §4). krilla-fit's 377 KB measurement and every comparable renderer show that the budget can only cover the base module. ADR 0016 (B6a) should restate it per module rather than for "`@lamina/pdf` including shaping".

The third concerns **render backend per surface** (proposed ADR 0004, not yet accepted). PDFium as the default suits the desktop. For edge runtimes (2.15 MB gz) and for mobile binary size, hayro may be the better default if C1a shows acceptable fidelity and size.

The fourth concerns **ubrn `wasm2` as a single-definition alternative** to wasm-bindgen. It could cover web and React Native from one UniFFI definition, but it is third-party, one UniFFI minor version behind, and has no known production web user. Keep ADR 0002's wasm-bindgen choice and re-evaluate alongside the Component Model in 2027.

### Items to verify before planning

The androidx.pdf release state (beta01 versus alpha19) heads the list. Others are: Safari's `Symbol.dispose` support; Cloudflare's rule against compiling wasm from bytes; Marker's and Docling's current licences; whether the wasm-bindgen unwind-at-boundary bug is fixed in 0.2.129; Kotlin cancellation semantics in UniFFI 0.32; and the licences of Ghostscript, pdftk-java and cpdf, which are not dependencies but appear in comparison docs.

## Conclusion

The research shifts the question from "which bindings" to "which contract". ADR 0002 already settled the bindings. What remains open is whether lamina's names, units, errors, schemas and job model are consistent enough that one mental model works in a React component, a SwiftUI view, a shell pipeline and an agent's tool call. That consistency is cheap to get before B5c ships and expensive afterwards, which is why the most urgent items here are rules for code that does not exist yet (L1, L2, L15, ADRs 0024–0026), not new crates.

The second shift is in where lamina competes. On Android, the free first-party viewer now annotates, fills forms and runs OCR. The CLI space has a permissive structural tool but no permissive text or render tool. AI extraction is led by models with tangled licences, while rules alone already get reading order mostly right. In each market the gap is the same: a permissive, tolerant engine that edits without breaking signatures, extracts with provenance, and behaves identically everywhere. lamina's accepted ADRs, from tolerance (0003) to the op log (0006) and the save planner (0007), already aim at that gap. The front ends only need to expose those properties without losing them on the way.
