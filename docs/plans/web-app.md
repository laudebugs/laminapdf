# Plan: web PDF reader on Cloudflare (`apps/web`)

- **Status:** Draft
- **Date:** 2026-10-28
- **Builds on:** [architecture plan](architecture.md) Round C (ADRs 0004, 0009, 0010, 0011; spikes `pdfium-wasm`, `hayro-parity`) and the [architecture research report](../research/architecture/app-and-package-architecture.md) (web client and Cloudflare sections)
- **Depends on:** [pdf-library.md](pdf-library.md) Rounds B2–B3 (`Document::open` with decryption, typed model), B6 (`lamina-wasm`; gates C1) and B9 (text extraction; gates C5)
- **Research:** [06 §9 phase 1](../research/pdf/06-feature-inventory.md#9-suggested-phasing), [04 §1 navigation](../research/pdf/04-interactive-security.md#1-navigation), [04 §7 encryption](../research/pdf/04-interactive-security.md#7-encryption-76), [03 §8 extraction](../research/pdf/03-text-and-fonts.md)
- **Roadmap:** [roadmap.md](roadmap.md)

## 1. Goal

A fast, accessible, privacy-first **PDF reader in the browser**, served from Cloudflare Workers static assets. It opens local and remote files, renders them with PDFium through the `RenderBackend` trait, and never uploads a document unless the user explicitly opts in to a server feature. The reader is the first shell of the eventual [editor](full-editor.md) and ships the packages (`@lamina/viewer`, `@lamina/ui`, `@lamina/render-pdfium`) that the [desktop app](desktop-app.md) reuses.

## 2. Scope and non-goals

**In scope (Rounds C1–C7)**

- Open: file picker, drag and drop, paste, `#url=` remote files (a fragment, so the document URL is never sent to our server), OS "Open with" (PWA file handling where supported), password-protected files (decrypted on device: PDFium for pixels, lamina's standard security handler from [library B2](pdf-library.md#b2-document-model) for text, outline and properties).
- Render: PDFium-WASM in a Web Worker; virtualised continuous scroll; low-res pass then sharp tiles; zoom (fit width, fit page, actual size, pinch, ctrl+wheel) with anchor preserved; rotation of the view.
- Navigate: thumbnails sidebar, outline/bookmarks, internal and external links (external links confirm before leaving), page labels, go-to-page, back/forward history.
- Text: selection and copy via a DOM text layer, find in document with highlighting and match count.
- Output: print, download (original bytes, unchanged).
- Document properties dialog (metadata, fonts, PDF version, encryption, signatures present).
- Keyboard and screen-reader support, high-contrast and dark mode, reduced motion.
- Offline/PWA, privacy-respecting operation, per-PR preview deploys.

**Non-goals for the reader** (they belong to [full-editor.md](full-editor.md))

- Any modification and save; annotations display only (rendered by PDFium), form fields display only.
- Accounts, cloud storage, sync, server-side processing. lamina will offer these as a hosted service (decided 2026-10-28, [architecture §8](architecture.md#8-open-questions-for-the-team) Q2), but they arrive in later rounds ([full-editor §6](full-editor.md#6-design-summary)); the reader v0.1 has no login and uploads nothing.
- Running PDF JavaScript (never executed in the reader).
- Mobile-specific UI beyond a responsive layout (mobile apps are an open question in [architecture §8](architecture.md#8-open-questions-for-the-team)).

## 3. Users and use cases

| User | Use case |
| --- | --- |
| Anyone with a PDF | Read a file without installing anything and without uploading it |
| Privacy-conscious users, legal/medical/finance | Open sensitive documents with a guarantee that bytes stay on the device |
| Developers and sites | Link to `https://<domain>/#url=…` to show a PDF; later embed `@lamina/viewer` |
| Users of assistive technology | Read tagged PDFs with a screen reader; navigate by keyboard |
| Users with huge files | Scroll a 1,000-page manual or a 500 MB scan without the tab crashing |

## 4. Requirements

### Functional

1. **Open** local files (input, drag-drop, `showOpenFilePicker` where available) and remote files by URL with HTTP range requests through a `ByteSource`. The remote URL comes from the `#url=` fragment, which browsers never send in requests, so neither the Worker nor its logs see which document was opened; the file is fetched directly from its host. A remote file without CORS shows an explanation instead of silently proxying. Encrypted files prompt for a password, which stays in the worker's memory.
2. **Render** every page through `RenderBackend` (PDFium default; hayro selectable in a debug menu for diffing). Annotation appearances render; widgets render read-only.
3. **Virtualise** pages with a prefix-sum layout; only visible pages ± a margin have canvases; canvases are pooled.
4. **Text layer** from lamina's text extraction ([library B9](pdf-library.md#b9-text-extraction), not the renderer) for selection, copy, find and screen readers; struct-tree linked via `aria-owns` when the file is tagged.
5. **Search** incrementally with results streaming in, case and whole-word options, and next/previous with keyboard.
6. **Navigate** via thumbnails, outline, links, page labels and history.
7. **Print** via a print-optimised render path (rasterise pages at print DPI into a hidden iframe; native PDF printing where the browser supports it); **download** returns the original bytes.
8. **Keyboard**: every action reachable without a mouse; shortcuts documented in a help dialog and consistent with desktop.
9. **Offline**: app shell and WASM cached by a service worker; works offline after first visit; installable as a PWA.
10. **Error handling**: damaged files open with a "repaired" notice (warnings from ADR 0003); unsupported features are listed, never silently dropped.

### Non-functional (targets are recommendations, measured from C3 onward)

| Metric | Target |
| --- | --- |
| App shell JS (excl. WASM), min+gz | ≤ 200 KB |
| PDFium WASM | Lazy-loaded in parallel with file selection; cached immutable |
| First page visible, 10-page 1 MB PDF, warm cache, mid-range laptop | ≤ 700 ms after file selection |
| First page visible, cold load on Fast 4G profile | ≤ 3 s |
| Scrolling a 1,000-page text PDF | No main-thread task > 50 ms; visible pages sharp within 300 ms of stopping |
| Memory, 1,000-page PDF scrolled end to end | ≤ 600 MB total (JS heap + WASM + bitmaps), stable (no growth on second pass) |
| Search, 1,000-page text PDF | First result ≤ 500 ms; full pass ≤ 10 s, non-blocking |
| Accessibility | WCAG 2.2 AA for app chrome; zero serious/critical axe violations |
| Browsers | Last two versions of Chrome, Edge, Firefox, Safari (desktop and iPadOS) |

### Privacy and security

- **No upload by default.** The default build has no `/api` routes, and remote-document URLs travel in the fragment (`#url=`), never in a request ([report, Cloudflare section](../research/architecture/app-and-package-architecture.md)). Any later server feature is opt-in per document and says what leaves the device (ADR 0011).
- **No cookies, no third-party scripts, no fingerprinting.** Telemetry recommendation: none in v0.1. If added later, only cookieless aggregate page views (for example Cloudflare Web Analytics) and opt-in error reports that never include document content, names or URLs. The decision goes in ADR 0011.
- **Strict CSP** (`script-src 'self' 'wasm-unsafe-eval'`, no inline scripts), `Referrer-Policy: no-referrer`, `Permissions-Policy` minimal; COOP/COEP set via `_headers` for the threaded WASM variant only (ADR 0009 decides).
- Parsing and rendering happen in a worker with `Limits`; hostile files can crash the worker but not the page, and the worker restarts.

## 5. Design summary

Architecture details live in [architecture.md](architecture.md) and the research report. The reader adds:

- **Packages**: `@lamina/viewer` (framework-free: worker RPC client, virtualiser, canvas pool, text layer, find controller, link layer), `@lamina/ui` (React 19 chrome: toolbar, sidebars, dialogs, `Platform` adapter interface), `@lamina/render-pdfium` (PDFium WASM adapter) and `@lamina/render-hayro` (pure-Rust renderer as its own `.wasm`), both optional so `@lamina/pdf` stays small.
- **Worker**: one document worker running `lamina-wasm` plus PDFium-WASM, a typed RPC with request IDs, priorities, transfer lists and `AbortSignal` cancellation (ADR 0009).
- **Platform adapter** (ADR 0010): the web implementation provides file open/save, print, clipboard and capability flags (`canWriteInPlace: false` on most browsers, `multiWindow: false`).
- **Hosting** (ADR 0011): one Worker with static assets, `not_found_handling: "single-page-application"`, immutable hashed assets, `_headers` for security headers, Wrangler pinned in CI. Production at `<domain>` (deployed only from `web-v*` release tags), staging from `main`, previews per PR via Workers preview URLs.
- **Storage**: OPFS as the random-access cache for large files and the "recent files" list (IndexedDB holds metadata only; files stay on the device and can be cleared in settings).

## 6. Rounds

Architecture Round C is delivered as rounds C1–C7, each split into todos (one PR each). C1 needs B6, because its PDFium spike loads `lamina-wasm` and measures reload after an incremental save; the `hayro-parity` half has no lamina dependency and can start earlier. C2–C4 follow C1; C5 also needs B9 (text extraction).

Each round is split into one or more todos (one pull request each) on the [todo board](../todos/README.md); they move `ready/` → `ongoing/` → `done/`. When a todo moves, the PR that moves it updates every link to it, including these. Their "Depends on" fields fix the order, including the cross-plan gates on library B6 and B9.

### C1. Rendering spikes

**Deliverable:** `spikes/pdfium-wasm/` (PDFium-WASM next to `lamina-wasm` in one worker: size, memory, render time, reload after incremental save) and `spikes/hayro-parity/` (pass rate vs PDFium on the pdf.js and PDFBox corpora), each with a written result.
**Accepted when:** both READMEs report numbers on the same machine and corpus, and recommend the defaults that ADR 0004 records.
**Todos:** [C1a](../todos/ready/C1a-hayro-parity-spike.md), [C1b](../todos/ready/C1b-pdfium-wasm-spike.md)

### C2. Render backend and ADRs

**Deliverable:** ADRs 0004, 0009, 0010, 0011; `crates/lamina-render` with the `RenderBackend` trait and PDFium and hayro adapters; `@lamina/render-pdfium`.
**Accepted when:** golden render tests pass for both adapters on `fixtures/`; the hayro-vs-PDFium diff runs in the nightly lane.
**Todos:** [C2a](../todos/ready/C2a-render-adrs.md), [C2b](../todos/ready/C2b-lamina-render-crate.md), [C2c](../todos/ready/C2c-render-packages.md)

### C3. Viewer core

**Deliverable:** `@lamina/viewer`: worker RPC, page virtualiser, canvas pool, progressive rendering, zoom with anchoring; a minimal demo page; performance harness.
**Accepted when:** a 1,000-page fixture scrolls end to end in Playwright (Chromium, Firefox, WebKit) within the memory and long-task budgets; cancelled renders never paint.
**Todos:** [C3a](../todos/ready/C3a-viewer-worker-rpc.md), [C3b](../todos/ready/C3b-viewer-virtualiser-and-zoom.md), [C3c](../todos/ready/C3c-viewer-perf-harness.md)

### C4. App shell on Cloudflare

**Deliverable:** `@lamina/ui` chrome (toolbar, page indicator, zoom, open dialogs, error and password prompts), `apps/web` Worker with static assets, `_headers`, CSP, per-PR preview deploys, staging deploy from `main`, production deploy from `web-v*` release tags.
**Accepted when:** each PR gets a preview URL comment; local open, drag-drop and `#url=` (CORS-enabled) work; encrypted fixtures open after the password prompt; securityheaders-style checks pass; Playwright asserts that no request body carries document bytes and that no request to our origin (URL, request line or headers, including `Referer`) contains the remote document URL.
**Todos:** [C4a](../todos/ready/C4a-ui-chrome-and-platform.md), [C4b](../todos/ready/C4b-web-app-shell.md), [C4c](../todos/ready/C4c-remote-files-encryption-privacy.md), [C4d](../todos/ready/C4d-deploy-previews-and-production.md)

### C5. Text, selection and search

**Deliverable:** text layer from the extraction API of [library B9](pdf-library.md#b9-text-extraction) (`lamina-content` interpreter + `lamina-text`), selection and copy, find bar with streaming results and highlights.
**Accepted when:** copy from fixtures in five scripts equals expected text, including encrypted fixtures; search results match pdf.js counts on the corpus; selection works with screen readers on tagged fixtures.
**Todos:** [C5a](../todos/ready/C5a-text-layer-and-selection.md), [C5b](../todos/ready/C5b-find-in-document.md)

### C6. Navigation, print and properties

**Deliverable:** thumbnails, outline, links (with external-link confirmation), page labels, history, print, download, document properties dialog.
**Accepted when:** link and outline targets match `expected.json`; printing a 50-page document works in Chrome, Firefox and Safari; download is byte-identical to the input.
**Todos:** [C6a](../todos/ready/C6a-thumbnails-outline-page-labels.md), [C6b](../todos/ready/C6b-links-and-history.md), [C6c](../todos/ready/C6c-print-download-properties.md)

### C7. Accessibility, offline and launch

**Deliverable:** keyboard map and help dialog, focus management, ARIA landmarks, high contrast and dark mode; service worker and web app manifest (with `file_handlers`); performance budgets enforced in CI; ADR 0011 privacy page published; public release `web v0.1`.
**Accepted when:** axe shows zero serious violations; a manual NVDA, VoiceOver and keyboard-only pass is recorded; the app works offline after first load; Lighthouse PWA installability passes; performance targets in §4 are met on the reference machine.
**Todos:** [C7a](../todos/ready/C7a-accessibility-and-theming.md), [C7b](../todos/ready/C7b-offline-pwa.md), [C7c](../todos/ready/C7c-performance-gates-and-launch.md)

## 7. Risks

| Risk | Mitigation |
| --- | --- |
| PDFium-WASM size (≈2.7 MB gz) hurts first load | Lazy load during file selection; immutable caching; hayro as long-term smaller default |
| Two parsers (lamina for text, PDFium for pixels) disagree on geometry | Nightly differential tests; text layer uses lamina only; report mismatches with fixture repro |
| 4 GB WASM ceiling, Safari without Memory64 | Stream from `ByteSource`/OPFS, cap tile caches, recommend desktop for giant files |
| Remote files blocked by CORS | Clear message; optional proxy only as an opt-in server feature later (open question) |
| COOP/COEP breaking embeds or popups | Single-threaded baseline by default; threaded variant only when cross-origin isolated |
| Accessibility of untagged PDFs | Heuristic reading order from text extraction; announce "untagged document" |

## 8. Open questions

1. Domain and product name for the hosted reader?
2. Do we offer an opt-in CORS proxy for remote URLs, given it sees the bytes?
3. Any telemetry at all in v0.1, or none until there is a privacy review?
4. Should `@lamina/viewer` be published for embedding in v0.1, or after the API settles?
5. Is iPad/tablet touch UX in scope for v0.1 beyond "works"?

## 9. To-dos

Each round's todos (one per PR, with dependencies) are linked from [§6](#6-rounds) and on the [todo board](../todos/README.md); this checklist is the summary.

Ticked by the PR that completes them.

### C1. Rendering spikes
- [ ] Build `spikes/pdfium-wasm`: load PDFium-WASM and `lamina-wasm` in one worker
- [ ] Measure bundle size, peak memory and render time per page on 3 reference files
- [ ] Measure reload cost after an incremental save from `lamina-save`
- [ ] Build `spikes/hayro-parity`: render the pdf.js and PDFBox corpora with hayro and PDFium; compute pass rate at a perceptual threshold
- [ ] Write both results with a recommendation for ADR 0004

### C2. Render backend and ADRs
- [ ] Write ADR 0004 (`RenderBackend`, PDFium default, hayro oracle, MuPDF prohibited)
- [ ] Write ADR 0009 (React chrome + framework-free viewer, typed worker RPC)
- [ ] Write ADR 0010 (Platform adapter and capability flags)
- [ ] Write ADR 0011 (Cloudflare topology, privacy and retention, telemetry policy, and the hosted service: login, accounts, cloud storage and sync as opt-in features; the login stack is added after the `auth` spike)
- [ ] Create `crates/lamina-render` with the trait and PDFium and hayro adapters behind features
- [ ] Create `packages/render-pdfium` (pin and hash the PDFium build) and `packages/render-hayro`
- [ ] Add golden render tests and the nightly hayro-vs-PDFium diff

### C3. Viewer core
- [ ] Create `packages/viewer` with the typed RPC (IDs, priorities, transfer lists, cancellation)
- [ ] Implement the prefix-sum virtualiser and canvas pool
- [ ] Implement low-res then tiled sharp rendering with `ImageBitmap` transfer
- [ ] Implement zoom modes and anchor-preserving zoom
- [ ] Add a perf harness: long tasks, memory, time-to-first-page, run in CI on a 1,000-page fixture

### C4. App shell on Cloudflare
- [ ] Create `packages/ui` (React 19, React Compiler) with the `Platform` interface and web implementation
- [ ] Create `apps/web` (Vite 8) with `wrangler.jsonc`, SPA fallback, `_headers`, CSP
- [ ] Implement open via input, drag-drop, paste, `showOpenFilePicker` and `#url=` (fragment) with range requests
- [ ] Add password prompt (passes the password to PDFium and `Document::open`) and error/repair notices
- [ ] Add CI deploy: preview URL per PR (comment on the PR), staging on `main`, production from `web-v*` release tags (behind C7c's release gates)
- [ ] Playwright test asserting no request body contains document bytes and no request to our origin contains the remote document URL

### C5. Text, selection and search
- [ ] Expose glyph runs and quads from the B9 extraction API through the worker RPC
- [ ] Build the DOM text layer aligned to rendered glyphs at every zoom
- [ ] Link the struct tree via `aria-owns` for tagged files
- [ ] Implement the find controller with streaming results, highlights and options
- [ ] Copy/extraction tests in five scripts; search-count parity tests vs pdf.js

### C6. Navigation, print and properties
- [ ] Thumbnails sidebar (rendered at low priority, virtualised)
- [ ] Outline sidebar with expand/collapse and current-section tracking
- [ ] Link layer: internal destinations, URIs with confirmation, named destinations
- [ ] Page labels in the page indicator and go-to-page
- [ ] Back/forward navigation history
- [ ] Print path at print DPI; download of original bytes
- [ ] Document properties dialog (metadata, fonts, version, encryption, signatures present)

### C7. Accessibility, offline and launch
- [ ] Keyboard shortcuts map shared with desktop; help dialog
- [ ] Focus management, landmarks, live-region announcements for search and page changes
- [ ] High contrast, dark mode, reduced motion
- [ ] Service worker caching shell + WASM; web app manifest with `file_handlers`
- [ ] Performance budgets as CI gates
- [ ] axe in Playwright; record a manual NVDA, VoiceOver and keyboard-only pass
- [ ] Publish the privacy page; tag `web v0.1` and deploy to production
