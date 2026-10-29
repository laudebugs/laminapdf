# Mobile PDF SDK expectations (iOS Swift/SwiftUI, Android Kotlin/Compose) for a Rust + UniFFI core — as of October 2026

Research note scope: what mobile developers expect from a drop-in PDF SDK, and what lamina (Rust core exposed via UniFFI) must offer. Researched 2026-10-29. Fetch access: `mozilla.github.io` was blocked from this session (UniFFI docs were read via raw.githubusercontent.com instead); Apple doc pages were not fetched directly (search snippets only). Items not verified in this session are marked **[unverified]** and live in Inferences/Gaps.

## 1. Platform and vendor API surfaces (PDFKit, PdfRenderer, androidx.pdf, Nutrient, Apryse, Foxit, PDFium)

### Takeaway
Android's first-party story changed fundamentally in 2024–2026: `androidx.pdf` reached **1.0.0-beta01 (26 Aug 2026)** with a Compose `PdfViewer`, an `EditablePdfViewerFragment` (ink via `androidx.ink`, highlights, form filling, autofill) and a backport to minSdk 28. That makes a "free first-party viewer + basic annotate + forms" the Android baseline lamina competes against. On iOS, PDFKit's API has been essentially stable since the iOS 16 (WWDC22) overlay-view/PencilKit additions; I found no evidence of significant new PDFKit API at WWDC25/iOS 26, only iOS 26 regressions reported on forums.

### Cited Findings
**Android PdfRenderer (framework)**
- Android 15 DP2 announced "substantial improvements" to PdfRenderer: password-protected files, annotations, form editing, searching, selection with copy; the code was moved to a module updatable via Google Play system updates, and a pre-Android-15 compatible version (`PdfRendererPreV`) was planned back to Android R — [Android Developers Blog, Mar 2024 (older, 2024)](https://android-developers.googleblog.com/2024/03/the-second-developer-preview-of-android-15.html)
- `PdfRenderer.Page.getFormWidgetInfos()` (two overloads, one filtering by `int[] types`), `searchText(String)` (returns bounds of all matches) and `selectContent(...)` (returns `PageSelection`) are marked added in API 35 — [PdfRenderer.Page reference](https://developer.android.com/reference/android/graphics/pdf/PdfRenderer.Page); [API 35 diff](https://developer.android.com/sdk/api_diff/35/changes/android.graphics.pdf.PdfRenderer.Page)
- `PdfRendererPreV` exposes the Android V APIs on Android R–U (API 30–34) — [Android PDF viewer guide](https://developer.android.com/develop/ui/views/layout/pdf/pdf-viewer); [PdfRendererPreV.Page reference](https://developer.android.com/reference/kotlin/android/graphics/pdf/PdfRendererPreV.Page)
- Note: the English PDF viewer guide lists these methods under a "prior to Android V" heading while the reference marks them API 35 — an inconsistency in Google's docs (search-result observation) — [Android PDF viewer guide](https://developer.android.com/develop/ui/views/layout/pdf/pdf-viewer)

**androidx.pdf (Jetpack) — release history** (all from [androidx.pdf release notes](https://developer.android.com/jetpack/androidx/releases/pdf))
- 1.0.0-alpha01 (7 Aug 2024): initial `PdfViewerFragment`, "supported only on Android V (SDK 35)", find-in-file via `isTextSearchActive`; "Edit mode is not yet supported"; annotate FAB fires `android.intent.action.ANNOTATE` (i.e. hands off to another app).
- alpha04 (13 Nov 2024): S–V supported via SDK extension 13; "actively working on incorporating Jetpack Compose".
- alpha07 (Feb 2025): `StylingOptions`. alpha09 (May 2025): rewritten fully in Kotlin with Coroutines + ViewModel.
- alpha10 (16 Jul 2025): "Exposes a Composable for presenting PDF content"; cross-page text selection; `pdf-document-service` with `PdfLoader`, `PdfDocument`, `SandboxedPdfLoader`; pre-warm API to cut cold-start latency.
- alpha11 (Oct 2025): selection menu API on the `PdfViewer` composable; page alignment; `PdfView` becomes a `ViewGroup`.
- alpha13 (11 Feb 2026): **`EditablePdfViewerFragment`** "PDF editing capabilities, including annotations and form-filling"; ink Pen/Highlighter/Eraser/Undo/Redo "leverages the androidx.ink library"; `isFormFillingEnabled` on `PdfView` and the `PdfViewer` composable; `EditablePdfDocument`; `PdfWriteHandle` for saving; image selection; two-page layout via `pagesPerRow`.
- alpha14 (Feb 2026): keyboard/mouse (Ctrl+C, Ctrl+scroll zoom); `onDocumentLoadSuccess(pdfDocument)`; annotation disabled while search is active.
- alpha15 (Mar 2026): SDK Extension 19 required for `ImageSelection`; Java 11 target.
- alpha17 (Apr 2026): `contentPadding` on `PdfViewer`; Compose compileSdk 37 → requires AGP 9.2.0 when using Compose.
- alpha18 (22 Apr 2026): "Backported to support android devices down to minSdk = 28 and sdk-extension < 13", described as reaching "an additional 2 billion active Android devices".
- alpha19 (1 Jul 2026): `AnnotationView`, `EditablePdfDocument` and annotation classes public; new `pdf-core` submodule; "Autofill support for form filling is enabled".
- **1.0.0-beta01 (26 Aug 2026)**: public APIs promoted to Beta except `@ExperimentalPdfApi`, which now covers `PdfViewer`, `PdfViewerState`, `EditablePdfViewerFragment`, `AnnotationsView`, `OcrProvider` — i.e. **the Compose viewer and editing are still experimental**; `OcrProvider` + `MlKitOcrProvider` in `pdf-ocr-play-services`; search streams page-by-page results with debounced input.
- Artifacts listed: `pdf-viewer-fragment`, `pdf-viewer`, `pdf-document-service`, `pdf-ocr-play-services` (and `pdf-core`); dependency `androidx.pdf:pdf-viewer-fragment:1.0.0-beta01`.

**Apple PDFKit**
- WWDC22 "What's new in PDFKit" (iOS 16): `PDFPageOverlayViewProvider` lets you place your own view over each page; `overlayViewForPage` supplies it (PDFKit handles bounds/rotation) and `willEndDisplayingOverlayView` is the place to save overlay data such as PencilKit drawings; also Live Text support and an image-to-PDF-page API — [WWDC22 session 10089](https://developer.apple.com/videos/play/wwdc2022/10089/)
- Developers struggle to make the overlay pattern work; sample code was never published (only a 2017 project referenced); a forum reply says `isInMarkupMode = true` on `PDFView` is needed to change hit testing [tip not verified against Apple docs] — [Apple forums 716766](https://developer.apple.com/forums/thread/716766); [Apple forums 722033](https://developer.apple.com/forums/thread/722033); [Apple forums 734544](https://developer.apple.com/forums/thread/734544)
- iOS 26: forum reports that a gesture recognizer over PDFKit breaks paging and of a single-page display-mode issue on iOS 26 beta (user reports, not Apple-confirmed) — [Apple forums, PDFKit tag](https://developer.apple.com/forums/tags/pdfkit?page=2)
- A recent forum thread reports a crash in PDFKit's tile-rendering code (`PDFPageAnalyzerV2`) on iPadOS 26.x on lower-RAM iPads — [Apple forums 825803](https://origin-devforums.apple.com/forums/thread/825803)

**Nutrient (formerly PSPDFKit)**
- iOS SwiftUI entry point is a `PDFView` struct conforming to `View` (no manual representable); usage `PDFView(document:)`; toolbar via `.toolbar` with `@PDFView.Scope` to share state; imports `PSPDFKit` and `PSPDFKitUI` — [Nutrient SwiftUI guide](https://www.nutrient.io/guides/ios/getting-started/swiftui/); [How to show a PDF in SwiftUI](https://www.nutrient.io/blog/how-to-show-a-pdf-in-swiftui/)
- Distribution: SPM, CocoaPods, or manual — [Nutrient iOS getting started](https://www.nutrient.io/sdk/ios/getting-started/)
- Nutrient changelog lists a fix for a crash on iOS 26/27 where pages with an active text selection were not recycled correctly after scrolling offscreen — evidence that even the commercial leader fights page-recycling bugs on new OS releases — [Nutrient iOS changelog](https://www.nutrient.io/guides/ios/changelog/)

**Snippets (platform baseline; written from the APIs cited above — illustrative, not compiled)**

Show a PDF — PDFKit in SwiftUI:
```swift
import PDFKit, SwiftUI
struct PDFKitView: UIViewRepresentable {
    let document: PDFDocument
    func makeUIView(context: Context) -> PDFView {
        let v = PDFView(); v.autoScales = true; v.displayMode = .singlePageContinuous
        v.document = document; return v
    }
    func updateUIView(_ v: PDFView, context: Context) { if v.document !== document { v.document = document } }
}
```
Search — PDFKit: `document.findString("invoice", withOptions: .caseInsensitive)` returns `[PDFSelection]`; `pdfView.highlightedSelections = results`.
Annotate — PDFKit: `let a = PDFAnnotation(bounds: r, forType: .highlight, withProperties: nil); page.addAnnotation(a)`. Ink: `PDFAnnotation(bounds:forType:.ink…)` + `add(_ path: UIBezierPath)`.
Fill form — PDFKit: widgets are `PDFAnnotation` with `widgetFieldType`; set `annotation.widgetStringValue = "…"`.
PencilKit overlay (iOS 16+): `pdfView.pageOverlayViewProvider = provider; pdfView.isInMarkupMode = true` with `func pdfView(_:overlayViewFor page: PDFPage) -> UIView?` returning a `PKCanvasView`.

Android — androidx.pdf:
```kotlin
// Fragment (stable surface)
val f = PdfViewerFragment()          // or EditablePdfViewerFragment() (@ExperimentalPdfApi)
supportFragmentManager.commit { replace(R.id.container, f) }
f.documentUri = uri                   // show
f.isTextSearchActive = true           // search UI
// Compose (experimental): PdfViewer(state, isFormFillingEnabled = true, contentPadding = …)
```
Raw PdfRenderer (show + search, API 35 / PreV on 30–34):
```kotlin
PdfRenderer(pfd).use { r -> r.openPage(i).use { p ->
    val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
    p.render(bmp, null, matrix, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
    val hits = p.searchText("invoice")         // API 35
    val widgets = p.getFormWidgetInfos()       // API 35
} }
```

### Inferences
- Because `androidx.pdf` now covers view/search/select/highlight/ink/forms/OCR for free on minSdk 28+, lamina's Android value must come from things it doesn't do: cross-platform parity, high-fidelity rendering of malformed PDFs (ADR 0003 tolerance), incremental save (ADR 0007), op log/undo (ADR 0006), signing, redaction, generation, and a non-experimental Compose API.
- PdfRenderer/androidx.pdf are built on PDFium (Google's PDF engine), so "PDFium-based viewers" and Android's first-party stack largely share rendering fidelity **[unverified in this session — commonly known; verify]**.
- Nutrient's SwiftUI shape (`PDFView(document:)` + scoped toolbar state) is the de-facto ergonomic bar; lamina's SwiftUI API should match it.

### Gaps
- Apryse (PDFTron) and Foxit mobile API snippets, and PDFium-based OSS viewers (e.g. AndroidPdfViewer/barteksc forks) — not researched within the tool budget.
- Exact PdfRenderer limits (one open page at a time, not thread-safe) were not confirmed from the fetched page (the fetch truncated); widely known but should be verified at [PdfRenderer reference](https://developer.android.com/reference/android/graphics/pdf/PdfRenderer).
- No WWDC25/iOS 26 PDFKit "what's new" source found; check the PDFKit docs' "What's new" diffs.
- Nutrient Android Compose API name/shape not found (search returned nothing).

## 2. Tile-based virtualized rendering

### Takeaway
The common pattern is: virtualized page list → each visible page drawn first as a low-res whole-page bitmap, then hi-res tiles for the visible region at the current zoom, rendered on background threads and cached. On Compose, Telephoto's subsampling `ZoomableImage` is the reference implementation of the tile approach.

### Cited Findings
- saket/telephoto: `ZoomableImage` is a "drop-in replacement for Image() composables featuring support for pan & zoom gestures and automatic sub-sampling of large images"; `Modifier.zoomable()` for other composables; Apache 2.0 — [klibs.io telephoto](https://klibs.io/project/saket/telephoto)
- panpf/zoomimage: Compose Multiplatform + Views; subsampling via Sketch/Coil integrations; base `ZoomImage` requires manual `subsampling.setImageResource()` — [zoomimage subsampling wiki](https://github.com/panpf/zoomimage/blob/main/docs/wiki/subsampling.md)
- hehua2008/ComposeSubsamplingImage: experimental CMP subsampling library, possibly first to support iOS — [Kotlin Slack](https://slack-chats.kotlinlang.org/t/18819156/hi-all-i-developed-composesubsamplingimage-https-github-com-)
- PDFKit itself renders via tiles: a reported iPadOS 26 crash is in PDFKit's tile-rendering code — [Apple forums 825803](https://origin-devforums.apple.com/forums/thread/825803)
- `androidx.graphics.lowlatency` is explicitly "not intended… when large areas are modified like panning or zooming" — i.e. tiling/zoom must use the normal render path — [Android Developers: stylus low latency](https://medium.com/androiddevelopers/stylus-low-latency-d4a140a9c982)

### Inferences
- lamina should expose a tile render primitive from Rust: `render_tile(page, scale, rect_in_page_space, into: &mut [u8] /* RGBA, caller-owned */)`, so the Swift/Kotlin side can feed CATiledLayer-style (iOS) or Telephoto-style subsampling (Compose) without copying. Telephoto's `SubSamplingImageSource` abstraction could be the Android adapter point **[API name unverified]**.
- Pipeline: (1) page-size table available immediately at open (so `LazyColumn`/`UICollectionView` can lay out 1,000+ pages without rendering); (2) thumbnail-scale render as placeholder; (3) tiles at power-of-two zoom buckets; (4) cancel off-screen tile jobs (requires explicit cancel tokens; see §5).

### Gaps
- CATiledLayer docs (levelsOfDetail/levelsOfDetailBias, tileSize) and subsampling-scale-image-view internals not fetched; cite [CATiledLayer](https://developer.apple.com/documentation/quartzcore/catiledlayer) after verifying.
- Bitmap pool specifics (Glide/Coil BitmapPool, `inBitmap`) not researched.

## 3. Memory

### Takeaway
iOS gives no published per-device limits; the only reliable signal is `os_proc_available_memory()` at runtime, and limits changed under developers (iPadOS 26.4 reportedly halved them on 8 GB iPads). An SDK must size its tile cache from the live budget and shed on pressure, not hard-code numbers.

### Cited Findings
- `os_proc_available_memory` returns the bytes remaining before the process hits its limit, at the time of the call; unavailable on macOS — [Apple docs: os_proc_available_memory](https://developer.apple.com/documentation/os/os_proc_available_memory); [xnu proc.h](https://cs.iossec.tech/xnu/latest/source/libsyscall/os/proc.h)
- `com.apple.developer.kernel.increased-memory-limit` is a Boolean entitlement; the higher limit is available "only on some device models", and the app should still work if extra memory isn't granted — [Apple entitlement docs](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.kernel.increased-memory-limit)
- A developer measured ~6144 MB ceiling with the entitlement on both a 12 GB iPhone 17 Pro Max and an 8 GB iPhone 16 Pro Max (unresolved, FB23183521) — [Apple forums 834805](https://developer.apple.com/forums/thread/834805)
- Report: iPadOS 26.4+ reduced per-app memory on an 8 GB iPad from 6 GB to 3 GB with no documentation — [Apple forums 826325](https://developer.apple.com/forums/thread/826325)
- App extensions have their own memory limit that overrides the entitlement — relevant for a Share/QuickLook/File Provider extension embedding the SDK — [Apple forums 852862](https://developer.apple.com/forums/thread/852862); [Apple forums 779756](https://developer.apple.com/forums/thread/779756)
- PDFKit memory complaints: ~30 MB growth per iteration looping a 300+ page document, not reclaimed before crash; PDFKit requesting allocations as large as 30 MB to render a 100×100 thumbnail — [Apple forums 93589](https://developer.apple.com/forums/thread/93589?page=3) (older, 2017–18)
- Apple's forum thread "Strategies for minimizing OOM crashes?" — [Apple forums 722126](https://developer.apple.com/forums/thread/722126)

### Inferences
- Recommended lamina design: Rust side holds no decoded page bitmaps by default; the platform layer owns pixel buffers (UIImage/CGImage or Android `Bitmap`) and the cache; Rust exposes `trim(level)` that drops decoded images, font caches and parsed content streams. Swift wires `UIApplication.didReceiveMemoryWarningNotification` and Kotlin wires `ComponentCallbacks2.onTrimMemory` to it.
- Budget heuristic (opinion): tile cache = min(fixed cap e.g. 64–128 MB, ~25% of `os_proc_available_memory()` at open), re-evaluated on warnings; Android: fraction of `ActivityManager.getMemoryClass()`.
- For 1,000+ pages: lazy page-tree resolution and an object cache bounded by `Limits` (ADR 0003) so memory is O(visible pages), not O(document).
- Rust allocator: a counting global allocator or per-document arena with a hard cap that maps to `Error::LimitExceeded` lets the SDK fail gracefully instead of being jetsammed.

### Gaps
- `onTrimMemory` levels (Android 14+ deprecations of some levels), `largeHeap`, hardware bitmaps (`Bitmap.Config.HARDWARE`, not writable — so tiles must be rendered into ARGB_8888 then optionally copied) were not researched in this session; verify at [ComponentCallbacks2](https://developer.android.com/reference/android/content/ComponentCallbacks2) and [Bitmap.Config](https://developer.android.com/reference/android/graphics/Bitmap.Config).
- No official Apple table of jetsam limits exists (consistent with all sources).

## 4. Stylus input and ink

### Takeaway
Android now has a complete first-party stylus stack: `androidx.ink` 1.0 stable (Dec 2025), `input-motionprediction` 1.0 stable (Nov 2025), and `graphics-lowlatency` front-buffered rendering — and androidx.pdf already uses ink for PDF annotation. On iOS, PencilKit over `PDFPageOverlayViewProvider` is the sanctioned route. lamina should not build its own ink engine; it should accept stroke data from these and serialise it to `/Ink` annotations (with appearance streams).

### Cited Findings
- `androidx.ink` 1.0.0 is stable; 1.1.0-alpha08 latest (9 Sep 2026); 1.1 alphas add a mesh-editing eraser that splits strokes and public brush customization — [androidx.ink releases](https://developer.android.com/jetpack/androidx/releases/ink)
- 1.0 stable date reported as 17 Dec 2025 (secondary source) — [Juejin](https://juejin.cn/post/7595478717985062921)
- A finished `Stroke` is immutable: `ImmutableStrokeInputBatch` (input points), `Brush` (style), `PartitionedMesh` (geometry); `InProgressStroke` for live rendering — [Ink API modules](https://developer.android.com/develop/ui/views/touch-and-input/stylus-input/ink-api-modules)
- androidx.pdf `EditablePdfViewerFragment` "leverages the androidx.ink library" for Pen/Highlighter/Eraser/Undo/Redo — [androidx.pdf releases](https://developer.android.com/jetpack/androidx/releases/pdf)
- `MotionEventPredictor` (androidx.input:input-motionprediction) predicts future motion events; 1.0.0 stable 19 Nov 2025; since beta06 it uses the system prediction API when available — [androidx.input releases](https://developer.android.com/jetpack/androidx/releases/input)
- `graphics-lowlatency` (`GLFrontBufferedRenderer`) uses front-buffer rendering for handwriting; Android 10+ (API 29) and ChromeOS Android 11+; works with your own data type (x, y, colour, brush, pressure); not for panning/zooming — [Android Developers: stylus low latency](https://medium.com/androiddevelopers/stylus-low-latency-d4a140a9c982)
- iOS: `PDFPageOverlayViewProvider` overlay + save PencilKit drawing in `willEndDisplayingOverlayView` — [WWDC22 10089](https://developer.apple.com/videos/play/wwdc2022/10089/)

### Inferences
- Stroke → PDF: convert each stroke's input points (page-space, after inverse view transform) into `/InkList` arrays and generate an `/AP /N` appearance stream from the brush mesh/outline so rendering matches on viewers that ignore `/InkList` smoothing (ISO 32000-2 §12.5.6.13 Ink annotations — **[section number unverified]**). Keep the original PencilKit `PKDrawing` data / ink `Stroke` serialisation in a private key for lossless re-editing.
- Pressure/tilt: `/InkList` has no pressure; variable width must live in the appearance stream.

### Gaps
- Apple Pencil Pro squeeze/hover APIs (`UIPencilInteraction`, `UIHoverGestureRecognizer`), predicted touches (`UIEvent.predictedTouches(for:)`), and PencilKit iOS 17/18 additions not researched in this session.
- ink Stroke serialisation format (`ink-storage`) not checked.

## 5. Native concurrency and UniFFI async

### Takeaway
UniFFI maps Rust `async fn` to Swift `async` and Kotlin `suspend fun`, with the foreign side driving the future — but **there is no built-in cancellation**, and Swift 6 support is only "partial" (generated classes are `@unchecked Sendable`). lamina must design explicit cancellation tokens and an idiomatic hand-written Swift/Kotlin layer on top of generated bindings.

### Cited Findings
- UniFFI "can convert a Rust Future/async fn to and from foreign native futures"; Swift/Python use async/await, Kotlin `suspend fun`; proc-macro `#[uniffi::export]` on `async fn`, UDL `[Async]`; "There's no requirement for a Rust event loop" (foreign executor drives it) — [UniFFI futures.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md) (also published at [mozilla.github.io/uniffi-rs/latest/futures.html](https://mozilla.github.io/uniffi-rs/latest/futures.html))
- Cancellation: "There's no builtin way to cancel a future", no way to raise Swift's `CancellationError`; recommended: library-specific `cancel()` that sets a flag, mapped to an error variant or empty result — [UniFFI futures.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md)
- A UniFFI commit notes Python and Kotlin get cancellation "for free" because they throw cancellation exceptions (i.e. the foreign await is abandoned; the Rust future is dropped/released) — [semanticdiff of commit 20d4d8cc](https://app.semanticdiff.com/gh/mozilla/uniffi-rs/commit/20d4d8ccab416bb7a8b87af299ffc38391050779)
- Async callback interfaces (foreign traits with async methods) are supported; example trait bound `Send + Sync` — [UniFFI futures.md](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md)
- UniFFI Swift docs: "partial support for Swift 6" — [UniFFI Swift overview](https://mozilla.github.io/uniffi-rs/latest/swift/overview.html)
- Issue #2046: generated Swift failed to compile in Swift 6 mode (shared mutable state); PR #2318 (merged Jan 2025) marks all generated classes `@unchecked Sendable`; discussion calls that a pitfall and proposes per-type control — [uniffi-rs #2046](https://github.com/mozilla/uniffi-rs/issues/2046); [uniffi-rs PR #2318](https://github.com/mozilla/uniffi-rs/pull/2318)
- Foreign-trait protocols generated by UniFFI require `Sendable`, imposing obligations on implementers — [UniFFI 0.29 Swift overview](https://mozilla.github.io/uniffi-rs/0.29/swift/overview.html)
- Swift forum: generated `static var` globals changed to `static let` for Swift 6 — [Swift forums 76219](https://forums.swift.org/t/question-on-sendability-swift-6-data-race-safety-and-ffi-interfaces/76219)
- Workaround pattern in the wild: pair each async endpoint with a separate cancel function — [docs.rs smix-ffi](https://docs.rs/smix-ffi)

### Inferences
- Pattern for lamina: Rust exports `RenderJob` objects with `cancel()`; Swift wrapper uses `withTaskCancellationHandler { try await job.run() } onCancel: { job.cancel() }`; Kotlin wrapper uses `suspendCancellableCoroutine`/`invokeOnCompletion` or relies on the free coroutine cancellation plus a Rust-side `Drop` that flips the flag.
- Expose the document as a Swift `actor`-friendly, truly `Sendable` handle (Rust `Arc<Document>` + internal locking) so `@unchecked Sendable` is actually true; never hand out non-thread-safe Rust objects.
- Kotlin: progress/search results as `Flow<SearchHit>` built over a Rust callback interface; render on `Dispatchers.Default` (CPU), IO on `Dispatchers.IO`. Swift: `AsyncStream<SearchHit>`. This mirrors androidx.pdf beta01 streaming page-by-page search results.

### Gaps
- Exact Kotlin cancellation semantics in UniFFI 0.30–0.32 (does dropping the RustFuture handle actually drop the Rust future?) — internals doc on mozilla.github.io was unreachable.

## 6. Packaging and distribution

### Takeaway
SPM with a binary XCFramework target is now the only forward-looking iOS channel: CocoaPods trunk goes **read-only on 2 December 2026** (test run 1–7 Nov 2026). Android: AAR on Maven Central with per-ABI `.so`. React Native via uniffi-bindgen-react-native (Turbo Modules over JSI) is viable; Flutter via flutter_rust_bridge.

### Cited Findings
- CocoaPods: trunk read-only test run Nov 1–7, 2026; real switch **Dec 2, 2026**; existing builds won't break, but no further updates through trunk — [CocoaPods blog: Specs Repo](https://blog.cocoapods.org/CocoaPods-Specs-Repo/)
- Commentary: after the freeze a critical fix for a popular pod may have no canonical way to ship — [nesbitt.io, Jun 2026](https://nesbitt.io/2026/06/23/sunsetting-a-package-manager.html)
- Nutrient still lists SPM, CocoaPods and manual install for iOS — [Nutrient iOS getting started](https://www.nutrient.io/sdk/ios/getting-started/)
- uniffi-bindgen-react-native (Mozilla + Filament, Dec 2024): generates Turbo Modules in Rust; TypeScript + JSI C++ for Hermes; "early release" at launch — [Mozilla Hacks](https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/)
- 0.31.0-x releases: `ubrn build jsi2`, second WASM target `wasm2`, uniffi pin relaxed to `=0.31` — [newreleases 0.31.0-6](https://newreleases.io/project/github/jhugman/uniffi-bindgen-react-native/release/0.31.0-6); [0.31.0-4](https://newreleases.io/project/github/jhugman/uniffi-bindgen-react-native/release/0.31.0-4)
- androidx.pdf Compose path requires AGP 9.2.0 / compileSdk 37 — a signal of how fast Android toolchain floors move — [androidx.pdf releases](https://developer.android.com/jetpack/androidx/releases/pdf)

### Inferences
- uniffi-bindgen-react-native's web/WASM flavour means one Rust core could feed RN mobile and RN-web; relevant given ADR 0002 (wasm-bindgen for web).
- Ship: `Lamina.xcframework` (ios-arm64, ios-arm64-simulator, maybe macCatalyst) zipped as an SPM `.binaryTarget(url:checksum:)` + a source Swift target with the idiomatic wrapper; Android `lamina-core` (JNI .so for arm64-v8a, armeabi-v7a, x86_64) + `lamina-compose` AARs; rely on Play App Bundle ABI splits.

### Gaps
- App-size impact numbers for a Rust PDF core (and for PDFium-based competitors), Kotlin Multiplatform packaging of UniFFI bindings (e.g. Gobley / uniffi-kotlin-multiplatform), flutter_rust_bridge status — not researched.

## 7. Accessibility, dark mode, RTL

### Takeaway
Not researched in depth within the tool budget; requirements below are inferences to verify.

### Cited Findings
- androidx.pdf supports keyboard/mouse (Ctrl+C copy, Ctrl+scroll zoom) and selection menus — accessibility-adjacent parity the platform viewer already offers — [androidx.pdf releases](https://developer.android.com/jetpack/androidx/releases/pdf)
- androidx.pdf exposes OCR (`OcrProvider`, `MlKitOcrProvider`), relevant for making scanned PDFs readable/selectable — [androidx.pdf releases](https://developer.android.com/jetpack/androidx/releases/pdf)

### Inferences
- VoiceOver/TalkBack: rendered tiles are images; the SDK must publish an accessibility tree per page from the text layer (tagged-PDF structure tree when present, else reading-order heuristics) as `UIAccessibilityElement`s / Compose `semantics {}` nodes. Dynamic Type can't reflow fixed-layout PDF; offer a "reflow/reader mode" text view.
- Dark mode: chrome follows system; page content should optionally be inverted/sepia via a render-time colour filter rather than altering the PDF.
- RTL: mirror page order/scroll direction based on `/ViewerPreferences /Direction /R2L` and UI layout direction.

### Gaps
- No sources gathered on PDFKit VoiceOver behaviour or TalkBack support in androidx.pdf.

## 8. What developers complain about, and why they pay

### Takeaway
Complaints about first-party stacks: memory spikes/crashes on large docs, undocumented OS regressions, missing sample code, limited annotation/form fidelity, and (Android, until 2026) no editing at all. Commercial SDKs sell a finished UI, cross-platform parity and support — but they also fight OS regressions.

### Cited Findings
- PDFKit memory growth and oversized thumbnail allocations — [Apple forums 93589](https://developer.apple.com/forums/thread/93589?page=3) (older)
- PDFKit tile-rendering crash on low-RAM iPads under iPadOS 26 — [Apple forums 825803](https://origin-devforums.apple.com/forums/thread/825803)
- PencilKit-on-PDFKit overlay is hard to get working; no Apple sample — [Apple forums 716766](https://developer.apple.com/forums/thread/716766); [Apple forums 734544](https://developer.apple.com/forums/thread/734544)
- PDFKit ink annotation zoom issue — [Apple forums 99789](https://developer.apple.com/forums/thread/99789) (older)
- iOS 26 paging regressions with gesture recognizers over PDFKit — [Apple forums, PDFKit tag](https://developer.apple.com/forums/tags/pdfkit?page=2)
- androidx.pdf took two years of alphas (Aug 2024 → Aug 2026 beta) and its Compose viewer and editing are still `@ExperimentalPdfApi` — [androidx.pdf releases](https://developer.android.com/jetpack/androidx/releases/pdf)
- Nutrient ships a native SwiftUI `PDFView` with toolbar scoping — the ergonomics PDFKit lacks — [Nutrient SwiftUI guide](https://www.nutrient.io/guides/ios/getting-started/swiftui/)

### Inferences
- lamina's pitch to mobile devs: "PDFKit/androidx.pdf ergonomics, Nutrient-class features, identical behaviour on both platforms, open licence (MIT OR Apache-2.0)" — the licence point matters because Apryse/Nutrient/Foxit are commercial with per-app licensing **[pricing not researched]**.

### Gaps
- Nutrient/Apryse/Foxit pricing and licensing terms; developer survey data on why teams pay.

## Ideal drop-in API sketch (recommendation, not sourced)

```swift
// SwiftUI
import Lamina
struct ReaderScreen: View {
    @State private var doc = try! LaminaDocument(url: fileURL)   // async open: try await LaminaDocument.open(url:)
    @State private var viewer = LaminaViewerState()               // @Observable: page, zoom, selection, tool
    var body: some View {
        LaminaView(document: doc, state: viewer)
            .laminaDisplayMode(.continuous)
            .laminaTool($viewer.tool)                                // .none / .ink(.pen) / .highlight / .form
            .laminaSearch(text: $query)                              // AsyncStream<SearchHit> under the hood
            .onLaminaSave { data in try data.write(to: fileURL) }   // incremental save (ADR 0007)
            .toolbar { LaminaAnnotationToolbar(state: viewer) }
    }
}
// headless
let hits = doc.search("invoice")               // AsyncThrowingStream<SearchHit, Error>, cancels with the Task
try await doc.form.setValue("Jane", forField: "name")
let png = try await doc.page(0).render(scale: 2)  // cancellable via Task
```

```kotlin
// Compose
@Composable fun ReaderScreen(uri: Uri) {
    val doc by rememberLaminaDocument(uri)                  // State<LoadState<LaminaDocument>>
    val state = rememberLaminaViewerState()                  // page, zoom, selection, tool; saveable
    LaminaViewer(
        state = state, document = doc,
        tool = LaminaTool.Ink(Brush.Pen),                    // uses androidx.ink + motion prediction
        formFillingEnabled = true,
        contentPadding = PaddingValues(16.dp),
    )
}
// headless
lifecycleScope.launch {
    doc.search("invoice").collect { hit -> /* Flow<SearchHit> */ }
    doc.form.setValue("name", "Jane")
    val bmp: Bitmap = doc.page(0).render(scale = 2f)          // suspend, cancels Rust job on coroutine cancel
}
```
