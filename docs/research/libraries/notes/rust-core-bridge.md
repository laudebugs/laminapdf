# Rust core bridge: one memory-safe PDF core → WASM/TS and Swift/Kotlin (UniFFI), small binaries

Scope note (state as of 2026-10-29). In this session `mozilla.github.io` (UniFFI book, pdf.js docs) and `contributing.bitwarden.com` could not be reached (DNS blocked by the sandbox). UniFFI facts therefore come from the UniFFI repo on GitHub (CHANGELOG, `docs/manual/src/*` — the same source the book is built from), crates.io and the npm registry. Statements labelled **[prior knowledge]** are from training data (cut-off mid-2026) and were not re-verified in this session; treat them as leads, not citations. Binary sizes marked **[measured]** were measured by downloading the published npm tarball and running `gzip -9` on the `.wasm` file in this session.

## 1. UniFFI in 2026: version, features, packaging, real-world users

### Takeaway
UniFFI is at **0.32.2 (crates.io, 2026-09-23)**; 0.32.0 (2026-06-30) added `#[uniffi::export(foreign)]` traits, async constructors, and **zero-copy byte arguments into Rust** (not yet for async fns). Proc-macros are the main path; UDL still works. Async maps to Swift `async` / Kotlin `suspend` with the foreign side as the executor, but **there is no built-in cancellation**. Kotlin Multiplatform (Gobley), React Native/WASM/Node (uniffi-bindgen-react-native, "ubrn"), C#, Go and Dart are third-party and **lag the UniFFI version** (ubrn pins `=0.31`).

### Cited Findings
- Latest crate versions: `uniffi` 0.32.2, updated 2026-09-23 — [crates.io uniffi](https://crates.io/crates/uniffi). The CHANGELOG lists v0.32.1 (2026-09-08: Kotlin checksum fix on aarch64) and v0.32.0 (2026-06-30) — [UniFFI CHANGELOG](https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md).
- v0.32.0: traits can be exported with `#[uniffi::export(foreign)]`, or `rust, foreign` to allow both directions; constructors can be async; `async_runtime` works on trait exports; async primary constructors fail to generate in Kotlin and Python; **zero-copy transfer of byte arguments to Rust, not yet supported in async functions**; UDL `[ByRef] bytes` now maps to `&[u8]` (breaking); records can rename fields; `HashSet` support; recursive enums — [UniFFI CHANGELOG](https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md).
- v0.31.0 (2026-01-14): renaming enum variants, methods on records and enums, renaming via TOML for every built-in binding, a new `uniffi::generate` function, `--lib-file` removed. v0.31.1 (2026-04-10) fixed a Swift iOS crash under AddressSanitizer and a **memory leak in Swift async code**. v0.31.2 fixed Kotlin JNA `u8`/`u16` returns — [UniFFI CHANGELOG](https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md).
- v0.30.0 (2025-10-08, older): Kotlin moved to **JNA direct mapping**; records and enums can export standard traits (incl. `Ord`); UDL trait interfaces need `#[uniffi::trait_interface]` — [UniFFI CHANGELOG](https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md).
- Built-in bindings are Kotlin, Swift, Python and Ruby. Third-party: `uniffi-bindgen-react-native` (WASM in web pages, plus React Native on iOS/Android), **Gobley** for Kotlin Multiplatform (JVM and Native targets), `uniffi-bindgen-cs` and `uniffi-bindgen-go` (NordSecurity), Dart, Java, Node, Haskell. Mozilla uses UniFFI "extensively" in Firefox mobile and desktop — [UniFFI README](https://github.com/mozilla/uniffi-rs/blob/main/README.md). `gobley-uniffi-bindgen` 0.3.7 was last published 2025-10-08 — [crates.io gobley-uniffi-bindgen](https://crates.io/crates/gobley-uniffi-bindgen).
- Async: UniFFI turns Rust futures into foreign native futures (Swift/Python `async`, Kotlin `suspend fun`). "There's no requirement for a Rust event loop": the foreign side drives polling. Async trait methods use `async-trait`, with `#[uniffi::export]` placed outside `#[async_trait]`. **"There's no builtin way to cancel a future"**: libraries should provide their own `cancel()`/flag, and UniFFI can't raise a platform `CancellationError` — [UniFFI manual: futures.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md).
- Objects: every object is heap-allocated in an `Arc`; foreign destructors drop one reference only. Methods take `&self`; **`&mut self` is disallowed**. State needs `Send + Sync` interior mutability (e.g. `AtomicU64`, not `RefCell`). The implementation must be `Send + Sync` (compile error otherwise), and exported traits need explicit `Send + Sync` bounds. Nested returned objects must be `Arc<T>` — [UniFFI manual: interfaces.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/types/interfaces.md).
- `cargo-swift` 0.11.1 (2026-05-20) is current — [crates.io cargo-swift](https://crates.io/crates/cargo-swift).
- uniffi-bindgen-react-native: npm `latest` is **0.31.0-6 (2026-09-25)**; earlier tags 0.31.0-2 (2026-03-25), -3 (2026-05-28), -5 (2026-08-21) — [npm registry: uniffi-bindgen-react-native](https://registry.npmjs.org/uniffi-bindgen-react-native). It supports uniffi-rs "`=0.31`", and bindings compiled against 0.30.x don't work with 0.31.x. **It doesn't yet support UniFFI 0.32** — [ubrn releases](https://github.com/jhugman/uniffi-bindgen-react-native/releases).
- ubrn WASM: the `web` flavour is wasm-bindgen based and needs the `wasm-bindgen` CLI on PATH at the version in `Cargo.lock` (0.31.0-6). The new `wasm2` flavour (0.31.0-5) reads UniFFI metadata straight from the compiled `.wasm`: "no shim crate to generate and no per-library glue". It needs a `cdylib`, the `uniffi-runtime-wasm` crate and the `uniffi_core` feature, with an `@ubjs/wasm` player. Node runtime `@ubjs/node` arrived in 0.31.0-3; four N-API leaks were fixed in -5. Async trampolines leaked/rebuilt per `await` until -5. The WASM `futures` fixture is excluded because it uses `std::thread::spawn`, "which single-threaded wasm32 cannot do". `forceAsync` "moves no work off the main thread" — [ubrn releases](https://github.com/jhugman/uniffi-bindgen-react-native/releases). Mozilla introduced ubrn as Rust-powered Turbo Modules in Dec 2024 (older) — [Mozilla Hacks](https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/).
- Matrix: in-tree `bindings/matrix-sdk-ffi` (UniFFI for `matrix-sdk`, consumed by Swift `matrix-rust-components-swift`) and `matrix-sdk-crypto-ffi` (UniFFI: Kotlin, Swift, Python, Ruby), plus a `matrix-sdk-ffi-macros` crate. WASM (`matrix-sdk-crypto-wasm`) and Node bindings live in **separate repos** — [matrix-rust-sdk/bindings](https://github.com/matrix-org/matrix-rust-sdk/tree/main/bindings).
- Bitwarden: `bitwarden_uniffi` (mobile, UniFFI `Client` wrapper) and `bitwarden_wasm_internal` (web, wasm-bindgen `BitwardenClient`) are **separate leaf crates**, both over `bitwarden_pm` ("meant for the final application crates") — [bitwarden_uniffi src](https://sdk-api-docs.bitwarden.com/src/bitwarden_uniffi/lib.rs.html), [bitwarden_wasm_internal client.rs](https://sdk-api-docs.bitwarden.com/src/bitwarden_wasm_internal/client.rs.html), [bitwarden_pm](https://sdk-api-docs.bitwarden.com/bitwarden_pm/index.html). `bitwarden_core` exposes `uniffi` and `wasm` Cargo features — [docs.rs bitwarden-core](https://docs.rs/bitwarden-core). They have a `bitwarden_ffi_macro` crate — [bitwarden_ffi_macro](https://sdk-api-docs.bitwarden.com/bitwarden_ffi_macro/all.html), and a shared `bitwarden_error` crate — [bitwarden_error src](https://sdk-api-docs.bitwarden.com/src/bitwarden_error/lib.rs.html). A PR notes UniFFI and wasm-bindgen can't handle generics, so they added a non-generic wrapper struct — [sdk-internal PR #488](https://github.com/bitwarden/sdk-internal/pull/488).

### Inferences
- **Pin UniFFI to 0.31.x** if lamina wants ubrn (React Native / `wasm2`) soon, or accept that Swift/Kotlin on 0.32 and ubrn on 0.31 can't share one `lamina-ffi` build. 0.32's zero-copy `&[u8]` arguments matter for PDF input (no copy of a 50 MB file into a `Vec<u8>`), so this is a real trade-off. Re-check ubrn before Round C.
- UniFFI's `&self` + `Send + Sync` rule shapes the façade even for wasm: designing `lamina` handles as `Arc<Inner>` with `Mutex`/`RwLock` (or `parking_lot`) inner state works for both. On wasm32 without threads a `Mutex` costs little; `RefCell` would not compile for UniFFI.
- Cancellation must be **in lamina's API** (a token object passed in, or a `cancel()` method on a job handle), because UniFFI doesn't map Swift `Task.cancel()`/Kotlin `Job.cancel()` into Rust.
- **[prior knowledge]** UniFFI error enums: `#[derive(uniffi::Error)]` on a `thiserror` enum becomes a Swift `Error` enum / Kotlin exception subclass; `#[uniffi(flat_error)]` sends only the `Display` string. Callback interfaces (`#[uniffi::export(callback_interface)]` or `with_foreign`) let Swift/Kotlin implement Rust traits; Mozilla application-services uses them widely. Swift objects free on `deinit`; Kotlin objects implement `AutoCloseable` with `.destroy()`/`.close()` plus a `Cleaner` fallback. Not re-verified this session.
- **[prior knowledge]** Packaging: iOS uses an XCFramework (`xcodebuild -create-xcframework` over the `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios` static libs plus the generated modulemap). `cargo-swift` automates this into a Swift Package. Android uses `cargo-ndk` to build `.so` per ABI into `jniLibs`, then an AAR; Kotlin bindings depend on JNA (≈1 MB of `.so`/jar per ABI on top of yours). Gobley wires this into Gradle for KMP.

### Gaps
- The UniFFI book page on bytes/zero-copy, the async `async_runtime = "tokio"` docs, and the Kotlin Multiplatform book page couldn't be fetched (mozilla.github.io blocked). Allow that host to verify.
- No source found on UniFFI's own binary-size overhead (scaffolding per exported function), or on 1Password or Signal using UniFFI. **[prior knowledge]** Signal's libsignal uses its own `bridge` macros (one Rust definition generating JNI, Swift-C and Node-Neon glue), not UniFFI, which is a relevant single-definition precedent. Not verified here.
- Lessons from application-services blog posts weren't reached.

## 2. One façade for wasm-bindgen and UniFFI: patterns and pitfalls

### Takeaway
No surveyed project puts one attribute set on one type and gets both bindings for free. The working pattern (Bitwarden, Matrix) is a **shared core with feature-gated derives on data types** (`cfg_attr(feature="wasm", derive(Tsify))`, `cfg_attr(feature="uniffi", derive(uniffi::Record))`) and **thin, separate leaf crates per binding** for handle types and methods. The other route is ubrn's `wasm2`, which compiles the UniFFI definition to wasm and generates TS from it, so a single definition covers both. That costs dependence on a third-party generator one minor version behind.

### Cited Findings
- Bitwarden: `uniffi` and `wasm` features on `bitwarden_core`; separate `bitwarden_uniffi` and `bitwarden_wasm_internal` crates over `bitwarden_pm` — [docs.rs bitwarden-core](https://docs.rs/bitwarden-core), [bitwarden_uniffi](https://sdk-api-docs.bitwarden.com/bitwarden_uniffi/index.html), [PasswordManagerClient (wasm)](https://sdk-api-docs.bitwarden.com/bitwarden_wasm_internal/struct.PasswordManagerClient.html).
- Generics aren't expressible in either binding, so Bitwarden added non-generic wrappers — [sdk-internal PR #488](https://github.com/bitwarden/sdk-internal/pull/488).
- UniFFI forbids `&mut self` and requires `Send + Sync` — [interfaces.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/types/interfaces.md). wasm-bindgen classes are single-threaded JS objects (no such requirement), so the stricter UniFFI rule sets the façade.
- ubrn `wasm2`: one UniFFI definition → `.wasm` + TS bindings, no shim crate — [ubrn releases](https://github.com/jhugman/uniffi-bindgen-react-native/releases).
- Current versions: `wasm-bindgen` 0.2.129 (2026-09-25); `tsify` 0.5.8 (2026-08-23) is maintained again, while the `tsify-next` fork was last released 0.5.6 (2025-04-20) — [crates.io wasm-bindgen](https://crates.io/crates/wasm-bindgen), [crates.io tsify](https://crates.io/crates/tsify), [crates.io tsify-next](https://crates.io/crates/tsify-next).

### Façade sketch (recommendation; derived from the cited constraints)
```rust
// crates/lamina (core layer: no wasm-bindgen / uniffi deps; plain Rust, Send + Sync handles)
pub struct Document { inner: Arc<RwLock<lamina_cos::Document>> }   // cheap Clone
#[derive(Clone, Debug)]
#[cfg_attr(feature = "tsify",  derive(serde::Serialize, tsify::Tsify), tsify(into_wasm_abi))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PageInfo { pub index: u32, pub width_pt: f32, pub height_pt: f32, pub rotation: u16 }

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
pub enum LaminaError { #[error("malformed PDF: {0}")] Malformed(String),
                       #[error("limit exceeded: {0}")] LimitExceeded(String),
                       #[error("cancelled")] Cancelled }

impl Document {
    pub fn open(bytes: Vec<u8>, opts: OpenOptions) -> Result<Self, LaminaError> { /* … */ }
    pub fn page_count(&self) -> u32 { /* … */ }
    pub fn page_info(&self, i: u32) -> Result<PageInfo, LaminaError> { /* … */ }
    pub fn save(&self, cancel: &CancelToken) -> Result<Vec<u8>, LaminaError> { /* … */ }
}
pub struct CancelToken(Arc<AtomicBool>);   // polled in long loops (save, render, text extraction)

// crates/lamina-wasm (edge): newtype wrappers, JS naming, Uint8Array in/out
#[wasm_bindgen(js_name = Document)]
pub struct WasmDocument(lamina::Document);
#[wasm_bindgen(js_class = Document)]
impl WasmDocument {
    #[wasm_bindgen(constructor)]
    pub fn open(bytes: &[u8], opts: JsOpenOptions) -> Result<WasmDocument, JsError> {
        lamina::Document::open(bytes.to_vec(), opts.into()).map(Self).map_err(to_js) }
    #[wasm_bindgen(js_name = pageCount)] pub fn page_count(&self) -> u32 { self.0.page_count() }
    #[wasm_bindgen(js_name = pageInfo)]
    pub fn page_info(&self, i: u32) -> Result<lamina::PageInfo, JsError> { self.0.page_info(i).map_err(to_js) }
    pub fn save(&self, c: &WasmCancel) -> Result<Vec<u8>, JsError> { self.0.save(&c.0).map_err(to_js) } // → Uint8Array
}

// crates/lamina-ffi (edge): UniFFI objects are Arc'd; &self only
#[derive(uniffi::Object)] pub struct FfiDocument(lamina::Document);
#[uniffi::export]
impl FfiDocument {
    #[uniffi::constructor]
    pub fn open(bytes: Vec<u8>, opts: lamina::OpenOptions) -> Result<Arc<Self>, lamina::LaminaError> {
        lamina::Document::open(bytes, opts).map(|d| Arc::new(Self(d))) }
    pub fn page_count(&self) -> u32 { self.0.page_count() }
    pub fn page_info(&self, i: u32) -> Result<lamina::PageInfo, lamina::LaminaError> { self.0.page_info(i) }
    pub fn save(&self, c: Arc<FfiCancel>) -> Result<Vec<u8>, lamina::LaminaError> { self.0.save(&c.0) } // → Data / ByteArray
}
uniffi::setup_scaffolding!();
```
The wrappers are mechanical, so a small `macro_rules!` or proc-macro in xtask can generate both. Bitwarden and Matrix both keep a `*-ffi-macros` crate for this.

### Inferences (pitfalls)
- **Lifetimes:** neither binding can return a borrow (`Page<'doc>`). Return owned handles that hold an `Arc` to the document plus an index (`PageRef { doc: Arc<…>, idx }`), and resolve lazily.
- **Generics:** monomorphise at the façade (`Limits` as a concrete struct, `ByteSource` hidden behind constructor variants such as `open_bytes`, `open_range_reader(callback)`). Consistent with [PR #488](https://github.com/bitwarden/sdk-internal/pull/488).
- **Iterators/streaming:** expose cursor handles (`next_batch(max) -> Vec<T>`) instead of Rust iterators. On wasm, a JS async iterator can wrap the cursor in TS. UniFFI has no iterator or stream type in the sources reviewed.
- **Cancellation:** a shared `CancelToken` object polled by the core (see §5). UniFFI doesn't propagate foreign task cancellation ([futures.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md)).
- **Bytes:** wasm `&[u8]` arguments are copied from JS memory into wasm linear memory regardless. UniFFI 0.32 can lend `&[u8]` without a copy (sync fns only). Return `Vec<u8>` (one copy out on both).
- **Errors:** keep one `thiserror` enum with string payloads (no nested non-FFI types), so it maps to both `uniffi::Error` and a JS `Error` with a `code` property matching `fixtures/codes.json`.

### Gaps
- Bitwarden's contributor docs (language-bindings, crate-structure) weren't fetchable (host blocked), so the exact `cfg_attr` patterns are inferred from crate features and type docs.
- No production user of ubrn `wasm2` for web found; ubrn notes say "distribution packaging … deferred".

## 3. Binary size: techniques and comparable numbers

### Takeaway
Comparable Rust WASM libraries sit far above lamina's ≤350 KB gz budget once they render or shape: typst-ts-renderer ≈ **355 KB gz**, photon ≈ **669 KB gz**, resvg-wasm ≈ **949 KB gz**, matrix-sdk-crypto-wasm ≈ **2.1 MB gz**, PDFium (emscripten, @embedpdf) ≈ **2.15 MB gz**, typst web compiler ≈ **10.8 MB gz** (all **[measured]**). The 350 KB budget holds for parse/edit/save only. Render, text shaping and signing must be separate optional modules (Cargo features plus separately loaded `.wasm`).

### Cited Findings ([measured], gzip -9 on the shipped `.wasm`; raw sizes in brackets)
- `@myriaddreamin/typst-ts-renderer` 0.7.0: 355,187 B gz [972,425 B raw] — [npm tarball](https://registry.npmjs.org/@myriaddreamin/typst-ts-renderer/-/typst-ts-renderer-0.7.0.tgz)
- `@silvia-odwyer/photon` 0.3.3: 669,227 B gz [1,880,773 B] — [npm tarball](https://registry.npmjs.org/@silvia-odwyer/photon/-/photon-0.3.3.tgz)
- `@resvg/resvg-wasm` 2.6.2: 948,750 B gz [2,478,606 B] — [npm tarball](https://registry.npmjs.org/@resvg/resvg-wasm/-/resvg-wasm-2.6.2.tgz)
- `@matrix-org/matrix-sdk-crypto-wasm` 18.9.0: 2,095,562 B gz [7,805,486 B] — [npm tarball](https://registry.npmjs.org/@matrix-org/matrix-sdk-crypto-wasm/-/matrix-sdk-crypto-wasm-18.9.0.tgz)
- `@embedpdf/pdfium` 2.15.1 (PDFium compiled to WASM): 2,148,845 B gz [4,646,932 B] — [npm tarball](https://registry.npmjs.org/@embedpdf/pdfium/-/pdfium-2.15.1.tgz)
- `@myriaddreamin/typst-ts-web-compiler` 0.7.0: 10,767,675 B gz [28,325,178 B] (fonts/compiler embedded) — [npm tarball](https://registry.npmjs.org/@myriaddreamin/typst-ts-web-compiler/-/typst-ts-web-compiler-0.7.0.tgz)
- `pdfjs-dist` 6.4.299 unpacked package is 34.9 MB (all builds, maps, viewer; not comparable per-module) — [npm registry pdfjs-dist](https://registry.npmjs.org/pdfjs-dist/latest)
- hayro has no published npm/wasm package (npm `hayro` → Not Found); its demo is at laurenzv.github.io/hayro — [hayro repo](https://github.com/LaurenzV/hayro), [npm registry](https://registry.npmjs.org/hayro).
- Tooling: `twiggy` 0.8.0 (2025-06-15); `wasm-split-cli` 0.7.10 stable / 0.8.0-alpha.3 (2026-10-16) for splitting a module into lazily loaded parts; `no-panic` 0.1.37 — [crates.io twiggy](https://crates.io/crates/twiggy), [crates.io wasm-split-cli](https://crates.io/crates/wasm-split-cli), [crates.io no-panic](https://crates.io/crates/no-panic).

### Inferences / techniques (recommendations; [prior knowledge] where noted)
- Profile (lamina already has `wasm-release`): `opt-level = "z"` (try `"s"`, often similar size and faster), `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`, then `wasm-opt -Oz --enable-bulk-memory` (plus `--converge` if time allows). **[prior knowledge]** This typically gives 15–30% over an un-opted release build.
- **std::fmt bloat** **[prior knowledge]**: every `format!`/`{:?}`/`panic!` message pulls in `core::fmt` (~20–40 KB). Avoid `Debug` derives in hot error paths of wasm builds, use static `&'static str` + code error variants, and `-Zlocation-detail=none` / `-Zfmt-debug=none` (nightly) for size builds. Panic messages vanish with `panic_immediate_abort` (`-Zbuild-std-features`, nightly).
- **Feature gates:** `render`, `text-shaping` (HarfRust + ICU4X data, the biggest risk: ICU4X segmenter/normaliser data can be hundreds of KB, so use `icu_provider` baked data with a minimal locale/key set), `sign` (RustCrypto + rsa bignum), `jpx`/`jbig2` decoders. hayro-interpret already depends on `skrifa`, `moxcms` and `phf` — [crates.io hayro-interpret 0.8.0 deps](https://crates.io/api/v1/crates/hayro-interpret/0.8.0/dependencies) — so render pulls in font and colour management.
- **Split modules:** ship `lamina_core.wasm` (parse/model/ops/save, target ≤350 KB gz), `lamina_render.wasm`, `lamina_text.wasm`, `lamina_sign.wasm` loaded on demand. Separate wasm-bindgen modules don't share memory, so pass bytes/handles across by serialisation. The other route is `wasm-split` (single build, lazily loaded chunks, sharing one memory), which is newer and less proven.
- Enforce the budget in CI (`cargo xtask wasm` + gzip size check + `twiggy top`/`twiggy dominators` diff on PRs).
- Native: **[prior knowledge]** iOS static `.a` files are large before linking (LLVM bitcode/metadata), but the app-size cost after dead-strip is what matters; measure with an Xcode app-thinning report. Android `.so` per ABI with `opt-level="z"`, LTO, `strip` is typically 1.5–3× the gz wasm size. Ship only `arm64-v8a` (+ `x86_64` for emulators).

### Gaps
- No reliable published wasm size for hayro, lopdf-in-wasm or pdf.js's own worker found; measuring hayro is a cheap spike (`cargo build --target wasm32-unknown-unknown -p hayro --profile wasm-release`).
- No authoritative iOS/Android binary-size numbers for UniFFI libraries found in this session.

## 4. Zero-copy, streaming, partial loading

### Takeaway
pdf.js shows the model to copy on web: range requests + streaming + optional auto-fetch, which pays off fully only for linearized files. For lamina, the `ByteSource` trait (ADR 0003/0005) should get a **fallible, possibly async, range-read edge implementation** (HTTP range / JS callback / mmap) feeding a sync core via "data not yet available" errors, instead of making the core async.

### Cited Findings
- pdf.js options: `disableRange` (range requests if the server supports `206 Partial Content`), `disableStream` (chunked streaming), `disableAutoFetch` (stop pre-fetching data not needed for the current page). Auto-fetch can only be disabled while streaming is on: "disabling streaming is also necessary for disabling pre-fetching to work correctly" — [pdfjs-dist api.d.ts](https://app.unpkg.com/pdfjs-dist@4.1.392/files/types/src/display/api.d.ts), [Nutrient: pdf.js advanced loading](https://www.nutrient.io/blog/pdfjs-advanced-loading-streaming-workers/). Fast first-page display from partial data needs linearized files — [Nutrient: linearized downloads](https://www.nutrient.io/guides/web/performance/linearized-downloads.md).
- hayro-syntax is "read-only by design" for speed and memory efficiency; the README says nothing about lazy or partial loading — [hayro repo](https://github.com/LaurenzV/hayro). hayro-interpret depends on `yoke` (borrowed-from-owned data), which suggests zero-copy parsing over an owned buffer — [hayro-interpret deps](https://crates.io/api/v1/crates/hayro-interpret/0.8.0/dependencies).

### Inferences
- **[prior knowledge]** PDFium's `FPDF_AVAIL` (`fpdf_dataavail.h`: `FPDFAvail_Create`, `FPDFAvail_IsDocAvail`, `FPDFAvail_IsPageAvail`, with `FX_FILEAVAIL`/`FX_DOWNLOADHINTS` callbacks) is a "pull: tell me which ranges you need" API. The core returns "not available" plus hints, and the embedder fetches and retries. This pattern fits a sync, bytes-in core: `ByteSource::read_at(off, len) -> Result<&[u8], NeedData{ranges}>`, and lazy object loading through the xref means only the bytes needed for an object are touched.
- Native: an `mmap` `ByteSource` lives in an edge crate (unsafe + fs). The core sees only `&[u8]`/`Arc<[u8]>`. Web: `Uint8Array` copied once into wasm memory (unavoidable with wasm-bindgen unless JS writes straight into a wasm-allocated buffer, as in `alloc(len) → view → fill`, which saves the double copy for large files). For `ReadableStream`, append chunks into a growable `ByteSource` and let open/parse return `NeedData` until the xref is reachable (trailer at EOF means non-linearized files need the tail first, via a range request for the last ~1–2 KB).
- Save planner (incremental append) works naturally with ranged sources: output = original bytes (passed through by reference) + appended section, so the façade could return only the appended `Vec<u8>` and let the host concatenate (saves copying the original back across FFI).

### Gaps
- PDFium availability-API docs and hayro's loading internals weren't verified in this session.

## 5. Concurrency, cancellation, progress

### Takeaway
Keep the core single-threaded and `Send + Sync`-safe. On web, use **one wasm instance per Web Worker** (no SharedArrayBuffer/COOP-COEP requirement). On native, parallelism (rayon per page) lives only in edge crates. Cancellation and progress are explicit façade objects, because neither UniFFI nor single-threaded wasm provides them.

### Cited Findings
- UniFFI objects must be `Send + Sync`; interior mutability must be thread-safe — [interfaces.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/types/interfaces.md).
- UniFFI async: foreign executor, no built-in cancellation; recommended pattern is a library-level cancel flag — [futures.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md).
- Single-threaded wasm32 can't `std::thread::spawn` (ubrn excludes its futures fixture for this reason); `forceAsync` adds a Promise surface but "moves no work off the main thread" — [ubrn releases](https://github.com/jhugman/uniffi-bindgen-react-native/releases).
- pdf.js runs parsing in a worker — [Nutrient: pdf.js streaming & workers](https://www.nutrient.io/blog/pdfjs-advanced-loading-streaming-workers/).

### Inferences
- **[prior knowledge]** wasm threads (`wasm-bindgen-rayon`, `+atomics,+bulk-memory` with nightly `-Zbuild-std`) need a SharedArrayBuffer, which needs cross-origin isolation (COOP `same-origin` + COEP `require-corp`/`credentialless`) on the embedding page. That is a burden for a library embedded in third-party apps. Recommend not requiring it: a worker pool (N instances, each its own document copy or page range) gives parallel page rendering without SAB.
- Cancellation design: `CancelToken(Arc<AtomicBool>)` checked every N objects/ops in loops. On wasm in a worker, a JS-side `cancel()` can't run while the worker is busy in a sync call, so either chunk work into resumable steps (`job.step(budget) -> Progress | Done`) or share the flag through a SAB (needs isolation). **Resumable job handles** (`step`) work everywhere and also give progress reporting without callbacks.
- Progress callbacks: UniFFI foreign traits (`#[uniffi::export(with_foreign)]`) or callback interfaces on native; `js_sys::Function` on wasm. Throttle on the Rust side (e.g. every 1%) to limit FFI crossings.

### Gaps
- No 2026 source on browser support or uptake of wasm shared memory without COOP/COEP; not researched further.

## 6. Rendering crate choices

### Takeaway
**hayro 0.8.0 (2026-10-04)** is the pure-Rust candidate: MIT/Apache, CPU raster via **vello_cpu 0.3.0**, SVG output, own JPX/JBIG2/CCITT decoders, and the best Rust correctness claims (1,600+ PDF regression suite). **pdfium-render 0.9.4** binds a C++ PDFium binary loaded at runtime. Its fidelity is the best, but it adds ≈2.1 MB gz wasm (or a native .so/.dylib per platform) and C++ memory-safety exposure.

### Cited Findings
- hayro crates: `hayro-syntax`, `hayro-interpret` (emits commands to an abstract `Device`), `hayro` (bitmaps), `hayro-svg`, `hayro-jpeg2000`, `hayro-jbig2`, `hayro-ccitt`, `hayro-postscript`, `hayro-cmap`. Licence Apache-2.0/MIT. Panic/timeout hardening on untrusted input, though timeouts/OOM can still occur. 1,600+ PDF visual regression suite (many from pdf.js/PDFBox). Knockout groups, CMYK blending and 3D are not supported. Glyph caching is missing. MSRV 1.92 — [hayro repo](https://github.com/LaurenzV/hayro).
- hayro 0.8.0 depends on `vello_cpu`, `fearless_simd`, `kurbo`, `image`, `pic-scale`, `bytemuck`; hayro-interpret on `skrifa`, `moxcms`, `hayro-cmap`, `yoke`, `phf` — [crates.io hayro deps](https://crates.io/api/v1/crates/hayro/0.8.0/dependencies), [hayro-interpret deps](https://crates.io/api/v1/crates/hayro-interpret/0.8.0/dependencies). `vello_cpu` 0.3.0 (2026-10-02) — [crates.io vello_cpu](https://crates.io/crates/vello_cpu).
- pdfium-render 0.9.4 (2026-09-06) depends on `libloading` (dynamic load of the PDFium library), `wasm-bindgen`, `web-sys`, `js-sys`, `chrono`, `console_error_panic_hook` — [crates.io pdfium-render deps](https://crates.io/api/v1/crates/pdfium-render/0.9.4/dependencies). PDFium as WASM (@embedpdf/pdfium) ≈ 2.15 MB gz **[measured]** — [npm tarball](https://registry.npmjs.org/@embedpdf/pdfium/-/pdfium-2.15.1.tgz).

### Inferences
- hayro has its own parser (hayro-syntax), separate from lamina-cos. Using it for rendering means parsing twice or adapting. hayro-interpret's `Device` trait is the natural seam: lamina-content could target the same device abstraction, or lamina-render could feed hayro bytes of a (saved) document. Worth a spike before Round C. `image` and `fearless_simd` dependencies should be audited for wasm size and `unsafe` (render would be an edge or optional core crate).
- GPU (`vello` + `wgpu`): **[prior knowledge]** wgpu adds a large binary (>1 MB gz wasm) and needs WebGPU/WebGL. Good for interactive zoom/tiling in apps, overkill for the core. Keep it as a later edge crate behind a `Device`-like abstraction, with `vello_cpu` as default.
- PDFium as an optional native-only backend (desktop/mobile edge crate) is reasonable for fidelity fallbacks. Licence is BSD-3/Apache, but it brings C++ (and FreeType/OpenJPEG CVEs), which conflicts with the memory-safety goal.

### Gaps
- No independent fidelity comparison of hayro vs PDFium vs pdf.js found; no hayro wasm size.

## 7. Cryptography for PAdES signing (pure Rust, wasm-compatible) and external signers

### Takeaway
The RustCrypto stack (`cms` 0.2.3 / 0.3.0-rc.0, `x509-cert` 0.3.0, `der` 0.8.3, `rsa` 0.9.10 / 0.10.0-rc.19, `p256` 0.14.0, `ecdsa` 0.17.0) is pure Rust and the usual choice for a wasm-compatible CMS/PAdES builder. The design should be **"build signed attributes → hand the digest to an external signer → embed the signature"**, so keys can stay in an HSM, Secure Enclave or WebCrypto.

### Cited Findings
- `cms`: pure Rust CMS (RFC 5652, RFC 3274) with a `builder` module; 0.2.3 stable, 0.3.0-rc.0 updated 2026-10-13. Dependencies: `der`, `spki`, `x509-cert`, `const-oid`, with optional `rsa`, `sha1`/`sha2`/`sha3`, `signature`, `aes`/`cbc` — [docs.rs cms](https://docs.rs/cms), [crates.io cms deps](https://crates.io/api/v1/crates/cms/0.2.3/dependencies), [crates.io cms](https://crates.io/crates/cms).
- Versions: `x509-cert` 0.3.0 (2026-07-09), `rsa` 0.9.10 stable / 0.10.0-rc.19 (2026-10-06), `p256` 0.14.0, `ecdsa` 0.17.0 (2026-07), `der` 0.8.3 (2026-10-25) — [crates.io x509-cert](https://crates.io/crates/x509-cert), [rsa](https://crates.io/crates/rsa), [p256](https://crates.io/crates/p256), [ecdsa](https://crates.io/crates/ecdsa), [der](https://crates.io/crates/der).
- Non-RustCrypto alternative `cryptographic-message-syntax` has a `SignedDataBuilder` — [docs.rs cryptographic-message-syntax](https://docs.rs/cryptographic-message-syntax/0.27.0/cryptographic_message_syntax/).
- pdf_oxide, a Rust PDF library, signs in its CMS module, but its Node/WASM bindings only read and verify signatures, with no CMS signer wired in — [pdf_oxide cms.rs](https://docs.rs/crate/pdf_oxide/0.3.76/source/src/signatures/cms.rs), [pdf_oxide digital signatures](https://pdf.oxide.fyi/docs/creation/digital-signatures).

### Inferences
- RustCrypto version churn is high (stable/rc split across `rsa` 0.9→0.10, `cms` 0.2→0.3, `x509-cert` 0.3). Pin one coherent generation, and isolate it in `lamina-sign` behind a feature, so churn and size (RSA bignum, SHA-2) don't hit the default build.
- **[prior knowledge]** `rsa` had the Marvin timing side-channel advisory (RUSTSEC-2023-0071). For *signing* with in-process keys this matters; external signers sidestep it. Check `cargo deny` advisories.
- **[prior knowledge]** `aws-lc-rs`/`ring` need C/asm and aren't a good fit for `wasm32-unknown-unknown` (ring has partial wasm support), so RustCrypto is the only realistic pure-Rust core choice. `getrandom` needs the `wasm_js` backend feature on web (getrandom 0.3+), and it belongs only in edge or sign crates.
- External-signer façade (recommendation):
  ```rust
  pub trait Signer: Send + Sync {            // UniFFI: #[uniffi::export(with_foreign)]; wasm: JS object / async fn
      fn certificate_chain(&self) -> Vec<Vec<u8>>;           // DER
      fn algorithm(&self) -> SigAlg;                         // RsaPkcs1Sha256 | EcdsaP256Sha256 | …
      fn sign_digest(&self, digest: Vec<u8>) -> Result<Vec<u8>, SignError>;
  }
  // Two-phase API for async/remote signers (WebCrypto is Promise-based, HSMs and remote signing are network calls):
  let pending = doc.prepare_signature(fields, placeholder_len)?;   // reserves /Contents, computes ByteRange digest of signedAttrs
  let to_sign: Vec<u8> = pending.data_to_sign();                    // host signs with Secure Enclave / WebCrypto / HSM
  let pdf: Vec<u8> = pending.finish(signature, chain, tsa_token)?;   // embeds CMS SignedData (PAdES B-B, B-T with RFC 3161 token)
  ```
  A two-phase (prepare/finish) API avoids async callbacks across FFI, works with WebCrypto's async `subtle.sign` and Secure Enclave, and keeps the core sync and bytes-in/bytes-out. Note that WebCrypto ECDSA returns raw `r||s` (IEEE P1363), which must be DER-encoded for CMS.

### Gaps
- No confirmed source that `cms`'s `SignedDataBuilder` builds for `wasm32-unknown-unknown` without extra features, or of an existing pure-Rust PAdES B-LT/B-LTA implementation. Verify with a spike build.

## 8. Panic safety across FFI

### Takeaway
On stable wasm32 the default is `panic=abort`, so a panic kills the instance. As of April 2026 wasm-bindgen can catch panics (`PanicError` in JS, instance still usable), but only with nightly `-Zbuild-std` + `-Cpanic=unwind` + WASM exception handling. For lamina the realistic policy stays **"no panics in core"** (lints + fuzzing), with abort-and-reinstantiate as the fallback on web and `catch_unwind` at the native FFI edge.

### Cited Findings
- Cloudflare (Apr 2026): wasm-bindgen catches panics in exports, surfaced to JS as `PanicError`; async exports reject their promise; the instance stays usable. Needs `RUSTFLAGS='-Cpanic=unwind'` with `-Zbuild-std` (nightly) and WebAssembly exception handling (broad engine support since 2023). Aborts such as OOM still can't unwind; an experimental `--reset-state-function` option was added for reinitialising after abort — [Cloudflare: Making Rust Workers reliable](https://blog.cloudflare.com/making-rust-workers-reliable). The stable default remains panic=abort without exception handling; std support for wasm32 `panic=unwind` exists behind `-Zbuild-std` — [Rust internals: wasm32 panic_unwind via wasm exceptions](https://internals.rust-lang.org/t/wasm32-unknown-unknown-panic-unwind-support-via-native-wasm-exceptions/18665).
- A March 2026 wasm-bindgen issue reported that unwinding builds aborted at the JS/wasm boundary instead of throwing; fix status unconfirmed — [wasm-bindgen task 4796 mirror](https://hub.harborframework.com/tasks/abundant/wasm-bindgen__wasm-bindgen-4796).
- hayro hardens against panics on untrusted input, but timeouts and OOM can still be triggered — [hayro repo](https://github.com/LaurenzV/hayro). `no-panic` 0.1.37 (link-time proof that a function can't panic) is maintained — [crates.io no-panic](https://crates.io/crates/no-panic).

### Inferences
- **[prior knowledge]** UniFFI-generated scaffolding wraps each exported call in `catch_unwind` and turns a panic into an `InternalException`/`UniffiInternalError` on the foreign side. That requires `panic=unwind` in native builds, so don't set `panic=abort` in the iOS/Android profiles if you want that safety net. Use a separate profile from `wasm-release`.
- The wasm module should be treated as disposable: the TS wrapper detects `RuntimeError: unreachable`, drops the instance, and reopens from the original bytes. The one-instance-per-worker design from §5 makes this cheap.
- Core hygiene (already partly in lamina's guidelines): clippy `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `arithmetic_side_effects` (consider), `cargo fuzz` targets per parser/filter in CI nightly lane, `Limits` for allocation (OOM is the one failure unwind can't catch), and `no-panic` on a few hot leaf functions where feasible.

### Gaps
- Whether the wasm-bindgen boundary-abort bug (Mar 2026) is fixed in 0.2.129, and whether `-Cpanic=unwind` for wasm32 has a stabilisation timeline, weren't confirmed.
