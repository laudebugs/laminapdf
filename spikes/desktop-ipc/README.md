# Spike: `desktop-ipc` (Round D1, todo D1a)

- **Date:** 2026-11-09
- **Status:** done. The recommendation below is an input to ADR 0008, which todo [D2a](../../docs/todos/ready/D2a-adr-0008-and-desktop-scaffold.md) writes.
- **Plan:** [desktop app §6 D1](../../docs/plans/desktop-app.md#d1-ipc-and-print-spikes), [architecture §4](../../docs/plans/architecture.md#4-spikes-to-close-open-questions) (`desktop-ipc`)
- **Code:** throwaway quality, outside both workspaces, never imported by product code ([code organisation §1](../../docs/guidelines/code-organization.md#1-top-level-layout)). Tauri 2.12 (crate `tauri` 2.12.5, `tauri-build` 2.7.4, `@tauri-apps/api` and `@tauri-apps/cli` 2.12.4), Rust 1.99.1.

## Question

ADR 0008 needs two answers before the desktop reader can be built. First, **which transport moves rendered page tiles from Rust into the webview fastest and most smoothly**: a `tauri::ipc::Channel` carrying raw RGBA, a command returning `tauri::ipc::Response` (a raw body, no JSON), or a custom URI scheme that the webview fetches, carrying raw RGBA or a compressed image? Second, **does each webview become cross-origin isolated** when Tauri sends COOP and COEP through `app.security.headers`? Isolation decides whether the desktop app can use the threaded WASM build and `SharedArrayBuffer` in its UI.

## Answer

**Compressed tiles over the custom scheme win on Windows and Linux and tie on the Mac, and lossless WebP is the compressed format that never fell behind.** In the medians of 5 runs at 512 px, DPR 2, `lamina://…?fmt=webp` moved 2.7× more tiles per second than the best raw transport on Windows, 2.5–2.7× more on Linux with the DMABUF renderer off, and 1.2× more on the hosted macOS 15 runner. On an M1 Pro it moved 1.02× as many, a tie under the spike's 15 % rule. Raw RGBA is limited by bytes: about 160 MB/s on the Windows runner and 200 MB/s on Linux, against 600 MB/s on the M1 Pro. A rendered PDF page is mostly white, so its 1 MB tile compresses to 8–37 KB and the bytes stop mattering. PNG matched WebP within 8 % on Windows and macOS 15, but ran at half its speed on Linux and below raw on the M1 Pro. Among the raw transports at 512 px, DPR 2, `ipc::Response` and the raw scheme finished within 6 % of each other on every machine; at 256 px, DPR 2 the raw scheme led by 13–19 % on Windows and the Macs, and by 0–3 % on Linux. The channel was the slowest raw transport at 512 px, DPR 2 on every machine, and trailed the raw scheme by up to 35 % on Windows (256 px, DPR 1).

**Isolation differs per OS.** WebView2 honours the headers fully (`crossOriginIsolated` and `SharedArrayBuffer` both work). WKWebView on macOS 27 reports `crossOriginIsolated = true` but still has no `SharedArrayBuffer`, and on macOS 15 it ignores the headers. WebKitGTK 2.52 ignores them. Only Windows could run the threaded WASM build, so the desktop app should stay single-threaded everywhere.

**Linux needs `WEBKIT_DISABLE_DMABUF_RENDERER=1` without a GPU driver:** under Wayland with the DMABUF renderer on, painting froze for good within two minutes, in both attempts.

## Recommendation (inputs to ADR 0008)

| Decision | Recommendation | The number behind it |
| --- | --- | --- |
| Primary transport | A custom scheme, `lamina://localhost/tile/…` (`http://lamina.localhost/tile/…` on Windows), fetched with `fetch()` and decoded by `createImageBitmap(blob)`, serving **lossless WebP** | Medians of 5 runs, 512 px, DPR 2: 701 tiles/s against 260 for the best raw transport on Windows, 820–897 against 331–332 on Linux, 397 against 319 on macOS 15, and 990 against 970 on the M1 Pro (a tie) |
| Fallback transport | The same scheme without `fmt`, serving raw RGBA: same URL, same Rust handler, no encoder | At 512 px, DPR 2, within 2 % of the fastest raw transport on every machine |
| Not used | `ipc::Channel` for tiles | The slowest raw transport in the medians at 512 px, DPR 2 on every machine (211 tiles/s against 260 for the scheme on Windows), with the worst p95 ready latency on Windows (107 ms); it also needs request batching that the others don't |
| Tile size | **512 device pixels** | At DPR 2, WebP moved 1.7–2.4× more pixels per second at 512 px than at 256 px on every machine (Windows 184 against 77 Mpx/s) |
| Compression | **Yes, lossless WebP**, encoded off the main thread in Rust | 0.8–1.7 ms per 512 px tile to encode, against 2.6–5.2 ms to render it; 8–12 KB per PDF tile instead of 1 MB |
| COOP/COEP | Send no isolation headers; build the desktop app single-threaded on every OS | Isolation works only on Windows; WebKit never exposes `SharedArrayBuffer` (macOS 15, macOS 27, WebKitGTK) |
| Linux renderer | Set `WEBKIT_DISABLE_DMABUF_RENDERER=1` when the GPU driver can't serve EGL, and measure on real GPUs in D2e | Wayland, DMABUF on, no GPU: froze after the prime, the checks and 5 sweep runs, and again in the smoke plan. X11, DMABUF on: 205 raw tiles/s and 80 dropped frames per run, against 324 and 8 with it off |
| Licences | Ask the owner for an MPL-2.0 exception for Tauri's dependencies before `apps/desktop` joins the workspace | `cargo deny check licenses` with the root `deny.toml` rejects five MPL-2.0 crates that Tauri 2.12 itself pulls in (see [Licences](#licences)) |

**The 15 % rule.** The spike's rule was to prefer the simpler transport when two finish within 15 % of each other. On the M1 Pro, WebP, the raw scheme, `ipc::Response` and the channel all did, so on that Mac alone the rule picks raw RGBA over the scheme. One transport has to serve every OS, though, and WebP was 2.5–2.7× faster on Windows and Linux at no measurable cost on the Mac. Raw RGBA stays as the fallback, one query parameter away.

ADR 0008 should record six things: the scheme name and its Windows form, WebP as the default with raw RGBA behind a switch, 512 px tiles, the single-threaded build with no isolation headers, the Linux DMABUF rule, and the MPL-2.0 exception. It should also record what this spike didn't measure (see [Limits](#limits-of-this-spike)): real GPUs on Windows and Linux, NVIDIA, and photo-heavy documents on fast Macs, where raw RGBA can beat WebP.

## Method

`src-tauri/` is a Tauri 2 app. Its Rust side cuts tiles from two sources. The **synthetic** document has 40 A4 pages of a gradient with noise in 4 × 4 blocks, which compresses, but not trivially. The **PDF** documents are three fixtures rendered on white by `laminapdf-render-pdfium`, the reader's own adapter, with PDFium `chromium/7881`. The text-heavy one is `fixtures/golden/examples/flowing_report.pdf`, the image-heavy one `fixtures/corpus/imaging-demo.pdf`, and the vector-heavy one `fixtures/golden/examples/images_and_vectors.pdf`. The 40 scrolled pages cycle through the three documents' pages, so a page repeats every few slots; the webview's 256 MB bitmap cache serves a repeated tile without the transport, which lightens the scroll's load (see [Limits](#limits-of-this-spike)). An LRU cache of 768 MB holds rendered tiles, so the benchmark measures the transport, not PDFium. A prime pass fills the cache and reports render time apart.

The webview side (`src/`) paints like `@laminapdf/viewer`: it gets each tile as an `ImageBitmap` and draws it on one Canvas2D canvas with `drawImage`, with no WebGL. Four transports, with three formats for the compressed one, make six variants:

| Name | Rust | Webview |
| --- | --- | --- |
| `channel` | `stream_tiles(jobs, on_tile: Channel<InvokeResponseBody>)` sends `InvokeResponseBody::Raw(header ++ rgba)` per tile; one channel per batch of requests made in the same task | `ArrayBuffer` → `ImageData` → `createImageBitmap` |
| `response` | `tile(job)` returns `tauri::ipc::Response::new(header ++ rgba)` | `invoke` → `ArrayBuffer` → as above |
| `scheme-raw` | `register_asynchronous_uri_scheme_protocol("lamina", …)` answers on a blocking thread with `header ++ rgba` | `fetch(url).arrayBuffer()` → as above |
| `scheme-png` | The same handler encodes with `png` 0.18.1 (`Compression::Fastest`, `Filter::Up`) | `fetch(url).blob()` → `createImageBitmap(blob)` |
| `scheme-webp` | The same handler encodes lossless with `image-webp` 0.2.4 | as `scheme-png` |
| `scheme-qoi` | The same handler encodes with `qoi` 0.4.1 | `fetch(url).arrayBuffer()` → a QOI decoder in script (`src/qoi.ts`, on the main thread: no webview decodes QOI) → `createImageBitmap` |

Every raw tile starts with 16 bytes: the request ID, width, height and an FNV-1a hash of the first row. The webview checks all four on every raw tile, and checks compressed tiles' sizes against the `X-Tile` header. Before the measured runs of each combination, every transport fetches 96 tiles with full checks, compressed tiles included (their first row is read back from a canvas). A mismatch stops the whole run with exit code 3. No transport corrupted a tile.

Each measured run has four phases, timed by `requestAnimationFrame`:

1. **Idle (0.5 s):** the median frame gap gives the display's frame interval.
2. **Scroll (8 s):** 40 pages scrolled top to bottom at a constant speed, at 100 % zoom (96 CSS px per inch). Each frame paints the tiles it has, fills the missing ones white, and requests missing tiles while fewer than *n* requests are in flight.
3. **Zoom (2 × 1.5 s):** 200 %, then 400 %, around the viewport centre.
4. **Burst:** the first 96 tiles that run requested, fetched again as fast as *n* requests in flight allow, decoded and dropped. Burst tiles/s and MB/s are the throughput figures. The scroll is paced, so its own rate measures demand, not capacity.

The tables report six figures:

- **Throughput:** burst tiles/s and MB/s (bytes as transported: compressed tiles count their compressed size).
- **Ready latency:** p50 and p95 from request to `ImageBitmap`. The CSVs also record request-to-paint latency, but it rounds to frames and counts repeated pages (a tile requested for one page and first painted on its repeat seconds later), so the summary leaves it out.
- **Dropped frames:** for each frame gap, `round(gap / interval) − 1`, summed over the scroll and zoom.
- **Fill time:** from the zoom step to the first frame with no white tile (`400 % fill ms` in `results/summary.md`).
- **Memory and CPU:** peak RSS of the app and its webview processes, and mean CPU, sampled every 100 ms by `sysinfo`.
- **Encode time:** Rust's mean encode time per compressed tile.

The plan per machine starts with an **in-flight sweep**: each transport runs once at 512 px, DPR 2 with *n* = 4, 8 and 16, and keeps the *n* with the highest burst throughput. Then each of five combinations runs 5 times, interleaved across transports so drift hits them alike: PDF tiles at 256 and 512 px with DPR 1 and 2, and synthetic tiles at 512 px with DPR 2. DPR is simulated: the canvas's backing store and the render scale use the plan's DPR, whatever the display's. A run that gets no animation frame for 3 s (a hidden window, a sleeping display, a stuck compositor) is void. It is retried twice, then the plan stops with exit code 4 and writes a failure row. 

The isolation check is a separate launch of each build: `src/coi.ts` reads `crossOriginIsolated`, tries `new SharedArrayBuffer(1)`, calls a command, and loads a scheme tile by `fetch` and by `<img>` with `Cross-Origin-Resource-Policy` set to `same-origin`, `cross-origin` and none.

## Environment

| Label | Machine | Webview | Display |
| --- | --- | --- | --- |
| `macos-m1pro` | Apple M1 Pro, 8 cores, 16 GiB; macOS 27.0.1 | WKWebView (macOS 27.0.1) | built-in Retina, 3024 × 1964; 60 frames per second |
| `macos-15-hosted` | GitHub `macos-15`: Apple M1 (virtual), 3 cores, 7 GiB; macOS 15.8.2 | WKWebView (macOS 15.8.2) | virtual |
| `windows-2025` | GitHub `windows-2025`: AMD EPYC 9V74, 4 cores, 16 GiB; Windows Server 2025 (26100) | WebView2 156.0.4314.58 | Hyper-V video, 1024 × 768, no GPU |
| `linux-x11-dmabuf-off` / `-on` | GitHub `ubuntu-24.04`: Intel Xeon 8573C or AMD EPYC, 4 cores, 15 GiB | WebKitGTK 2.52.6 (`webkit2gtk-4.1`) | Xvfb 1920 × 1080 × 24, Mesa llvmpipe |
| `linux-wayland-dmabuf-off` / `-on` | as above | as above | headless Weston, pixman renderer |

Each machine's details are in `results/<label>-machine.txt`. The hosted runners have no GPU: their numbers are a floor for those operating systems, not a forecast for users' machines.

## Results

Medians of 5 runs, PDF tiles. Each cell reads **burst tiles/s · burst MB/s · ready p50/p95 ms · dropped frames per run**; *n* is the in-flight limit the sweep chose. `results/summary.md` adds the request-to-paint latency, blank frames, zoom fill, memory, CPU, encode time, the synthetic runs and the sweep; the raw rows are in `results/*.csv`.

### macOS

**macos-m1pro**

| transport | 256 px, DPR 1 | 256 px, DPR 2 | 512 px, DPR 1 | 512 px, DPR 2 |
| --- | --- | --- | --- | --- |
| channel | 1391 · 218 · 4.0/9.0 · 1 | 1185 · 245 · 5.0/10.0 · 0 | 920 · 491 · 6.0/15.0 · 0 | 923 · 579 · 6.0/15.0 · 1 |
| response | 1548 · 242 · 4.0/10.0 · 0 | 1352 · 279 · 6.0/10.0 · 0 | 942 · 502 · 4.0/20.0 · 0 | 932 · 584 · 6.0/17.0 · 2 |
| scheme-raw | 1548 · 242 · 4.0/6.0 · 0 | 1524 · 315 · 4.0/7.0 · 1 | 988 · 527 · 4.0/10.0 · 1 | 970 · 608 · 5.0/11.0 · 0 |
| scheme-png | 1231 · 14 · 4.0/14.0 · 0 | 1157 · 4 · 5.0/13.0 · 0 | 609 · 22 · 5.0/16.0 · 1 | 793 · 22 · 6.0/29.0 · 4 |
| scheme-webp | 1600 · 6 · 4.0/9.0 · 0 | 1811 · 1 · 4.0/8.0 · 0 | 810 · 9 · 5.0/20.0 · 2 | 990 · 8 · 5.0/14.0 · 0 |
| scheme-qoi | 1655 · 14 · 3.0/7.0 · 0 | 1600 · 6 · 3.0/7.0 · 0 | 717 · 19 · 4.0/9.0 · 1 | 744 · 16 · 5.0/11.0 · 0 |

**macos-15-hosted**

| transport | 256 px, DPR 1 | 256 px, DPR 2 | 512 px, DPR 1 | 512 px, DPR 2 |
| --- | --- | --- | --- | --- |
| channel | 338 · 53 · 13.0/29.0 · 4 | 307 · 62 · 15.0/34.0 · 4 | 289 · 154 · 15.0/38.0 · 3 | 267 · 167 · 15.0/46.0 · 6 |
| response | 362 · 57 · 13.0/44.0 · 4 | 354 · 72 · 17.0/57.0 · 5 | 288 · 154 · 14.0/40.0 · 3 | 319 · 200 · 13.0/64.0 · 4 |
| scheme-raw | 397 · 62 · 12.0/32.0 · 4 | 421 · 86 · 16.0/31.0 · 8 | 319 · 170 · 13.0/44.0 · 4 | 316 · 198 · 14.0/34.0 · 5 |
| scheme-png | 568 · 6 · 8.0/39.0 · 1 | 706 · 2 · 12.0/41.0 · 3 | 422 · 15 · 10.0/31.0 · 1 | 429 · 12 · 9.0/55.0 · 5 |
| scheme-webp | 667 · 2 · 7.0/31.0 · 1 | 780 · 0 · 10.0/31.0 · 1 | 503 · 6 · 10.0/32.0 · 1 | 397 · 3 · 9.0/60.0 · 4 |
| scheme-qoi | 400 · 3 · 13.0/31.0 · 2 | 417 · 2 · 16.0/30.0 · 8 | 274 · 7 · 15.0/48.0 · 4 | 224 · 5 · 18.0/55.0 · 13 |

### Windows

**windows-2025**

| transport | 256 px, DPR 1 | 256 px, DPR 2 | 512 px, DPR 1 | 512 px, DPR 2 |
| --- | --- | --- | --- | --- |
| channel | 443 · 69 · 8.9/34.3 · 0 | 387 · 80 · 18.9/41.8 · 0 | 240 · 125 · 14.6/45.1 · 0 | 211 · 132 · 23.2/107.2 · 0 |
| response | 645 · 101 · 6.5/14.0 · 0 | 521 · 108 · 10.0/21.2 · 0 | 290 · 151 · 8.3/35.6 · 0 | 248 · 156 · 19.2/51.6 · 0 |
| scheme-raw | 683 · 107 · 6.4/25.7 · 0 | 590 · 122 · 11.4/36.5 · 0 | 308 · 161 · 7.9/34.4 · 0 | 260 · 163 · 16.4/89.3 · 0 |
| scheme-png | 1119 · 13 · 5.3/17.2 · 0 | 1132 · 4 · 10.9/26.0 · 0 | 754 · 28 · 5.5/13.5 · 0 | 724 · 21 · 11.1/29.4 · 0 |
| scheme-webp | 1211 · 4 · 5.1/14.4 · 0 | 1176 · 1 · 10.3/24.6 · 0 | 827 · 10 · 5.1/11.7 · 0 | 701 · 6 · 11.2/28.3 · 0 |
| scheme-qoi | 1112 · 9 · 5.0/9.3 · 0 | 1045 · 4 · 10.7/14.3 · 0 | 572 · 16 · 5.3/15.9 · 0 | 488 · 10 · 12.4/28.2 · 0 |

### Linux

X11 and Wayland with the DMABUF renderer off behave the same within 15 %. The DMABUF renderer is slower on X11 and froze on Wayland (see [Linux renderer](#linux-renderer)).

**linux-x11-dmabuf-off**

| transport | 256 px, DPR 1 | 256 px, DPR 2 | 512 px, DPR 1 | 512 px, DPR 2 |
| --- | --- | --- | --- | --- |
| channel | 941 · 147 · 9.0/11.0 · 0 | 774 · 160 · 13.0/16.0 · 0 | 346 · 185 · 11.0/30.0 · 3 | 305 · 191 · 21.0/37.0 · 10 |
| response | 1231 · 193 · 5.0/21.0 · 1 | 923 · 191 · 9.0/22.0 · 4 | 363 · 194 · 7.0/44.0 · 2 | 331 · 208 · 12.0/71.0 · 11 |
| scheme-raw | 1200 · 188 · 5.0/10.0 · 0 | 950 · 196 · 9.0/12.0 · 1 | 382 · 204 · 6.0/37.0 · 2 | 324 · 203 · 12.0/43.0 · 8 |
| scheme-png | 1185 · 13 · 7.0/16.0 · 0 | 1067 · 4 · 11.0/28.0 · 3 | 458 · 17 · 7.0/25.0 · 2 | 425 · 12 · 14.0/59.0 · 8 |
| scheme-webp | 1882 · 7 · 6.0/10.0 · 0 | 1920 · 1 · 9.0/14.0 · 0 | 910 · 11 · 6.0/13.0 · 0 | 820 · 7 · 10.0/25.0 · 3 |
| scheme-qoi | 1920 · 16 · 5.0/6.0 · 0 | 1714 · 6 · 8.0/10.0 · 0 | 794 · 21 · 5.0/10.0 · 0 | 691 · 15 · 10.0/13.0 · 0 |

**linux-wayland-dmabuf-off**

| transport | 256 px, DPR 1 | 256 px, DPR 2 | 512 px, DPR 1 | 512 px, DPR 2 |
| --- | --- | --- | --- | --- |
| channel | 970 · 152 · 9.0/18.0 · 0 | 857 · 177 · 15.0/24.0 · 0 | 370 · 197 · 11.0/48.0 · 3 | 312 · 195 · 21.0/55.0 · 14 |
| response | 1231 · 193 · 6.0/19.0 · 0 | 960 · 198 · 9.0/21.0 · 2 | 386 · 206 · 7.0/40.0 · 3 | 332 · 208 · 12.0/92.0 · 10 |
| scheme-raw | 1231 · 193 · 5.0/20.0 · 1 | 960 · 198 · 9.0/20.0 · 2 | 386 · 206 · 6.0/36.0 · 2 | 330 · 207 · 12.0/88.0 · 10 |
| scheme-png | 1116 · 13 · 7.0/11.0 · 0 | 1032 · 4 · 11.0/14.0 · 0 | 455 · 16 · 7.0/23.0 · 2 | 417 · 12 · 14.0/36.0 · 7 |
| scheme-webp | 1846 · 7 · 6.0/11.0 · 0 | 1959 · 1 · 9.0/14.0 · 0 | 920 · 11 · 6.0/13.0 · 0 | 897 · 7 · 10.0/23.0 · 3 |
| scheme-qoi | 1846 · 16 · 5.0/6.0 · 0 | 1778 · 7 · 8.0/9.0 · 0 | 802 · 21 · 5.0/9.0 · 0 | 686 · 15 · 9.0/14.0 · 0 |

**linux-x11-dmabuf-on**

| transport | 256 px, DPR 1 | 256 px, DPR 2 | 512 px, DPR 1 | 512 px, DPR 2 |
| --- | --- | --- | --- | --- |
| channel | 619 · 97 · 15.0/60.0 · 8 | 485 · 100 · 50.0/75.0 · 125 | 221 · 118 · 17.0/90.0 · 15 | 186 · 116 · 44.0/152.0 · 93 |
| response | 696 · 109 · 11.0/32.0 · 7 | 546 · 113 · 26.0/52.0 · 127 | 232 · 124 · 14.0/82.0 · 12 | 205 · 129 · 30.0/85.0 · 93 |
| scheme-raw | 733 · 115 · 9.0/35.0 · 4 | 561 · 116 · 26.0/49.0 · 124 | 242 · 129 · 12.0/82.0 · 9 | 205 · 128 · 28.0/85.0 · 80 |
| scheme-png | 906 · 10 · 8.0/24.0 · 3 | 787 · 3 · 21.0/46.0 · 176 | 366 · 13 · 11.0/34.0 · 5 | 333 · 10 · 25.0/90.0 · 62 |
| scheme-webp | 1215 · 4 · 9.0/22.0 · 1 | 1280 · 1 · 17.0/32.0 · 125 | 664 · 8 · 11.0/26.0 · 4 | 636 · 5 · 20.0/38.0 · 48 |
| scheme-qoi | 1129 · 10 · 9.0/28.0 · 4 | 980 · 4 · 20.0/32.0 · 98 | 468 · 12 · 10.0/39.0 · 6 | 421 · 9 · 21.0/69.0 · 67 |

### Synthetic tiles (512 px, DPR 2)

The gradient with noise is the worst case for compression: WebP still shrinks a 1 MB tile to about 13 KB. WebP stayed the fastest transport on Windows (654 tiles/s against 216 raw), Linux (733 against 291) and the hosted macOS 15 runner (379 against 276). On the M1 Pro, raw RGBA was faster (1,021 tiles/s against 828), because 710 MB/s of bytes cost less there than encoding and decoding. PNG and QOI gained less than WebP everywhere, and on the M1 Pro both ran at half raw's speed. A photo-heavy document on a fast Mac is where raw RGBA could beat WebP; the fallback keeps that case one parameter away.

### Cross-origin isolation

| Webview | Headers | `crossOriginIsolated` | `SharedArrayBuffer` | Commands | Scheme tiles under COEP |
| --- | --- | --- | --- | --- | --- |
| WebView2 156 (Windows) | off | false | unavailable | work | load |
| WebView2 156 (Windows) | on | **true** | **works** | work | load (`fetch` and `<img>`, any CORP) |
| WKWebView (macOS 27.0.1) | off | false | unavailable | work | load |
| WKWebView (macOS 27.0.1) | on | **true** | **unavailable** | work | load (`fetch` and `<img>`, any CORP) |
| WKWebView (macOS 15.8.2) | off | false | unavailable | work | load |
| WKWebView (macOS 15.8.2) | on | false | unavailable | work | load |
| WebKitGTK 2.52.6 (X11 and Wayland) | off | false | unavailable | work | load |
| WebKitGTK 2.52.6 (X11 and Wayland) | on | false | unavailable | work | load |

The headers came from `src-tauri/tauri.coi.conf.json` (COOP `same-origin`, COEP `require-corp`, CORP `same-origin`). The scheme's origin differs from the page's: `lamina://localhost` against `tauri://localhost`, or `http://lamina.localhost` against `http://tauri.localhost` on Windows. So `fetch` runs in CORS mode, and the handler answers `Access-Control-Allow-Origin: *`. Tiles loaded under COEP whatever CORP said, even `same-origin`, in every webview that applied the headers.

### Linux renderer

**Without a GPU, WebKitGTK's DMABUF renderer slowed painting on X11 and froze it on Wayland.** Mesa found no device (`ZINK: vkCreateInstance failed`, `egl: failed to create dri2 screen`) and fell back to llvmpipe.

- **X11, DMABUF on:** every plan finished, but 512 px tiles at DPR 2 dropped 48–93 frames per run, against 3–11 with it off. Raw throughput fell from 324 to 205 tiles/s.
- **Wayland, DMABUF on:** the window stopped receiving animation frames for good, once after the prime, verify and 5 sweep runs (about 1.5 minutes), and once in the shorter smoke plan. Both retries stalled too. Before it froze, each run dropped about 400 frames.

The `-on` labels set `WEBKIT_DISABLE_DMABUF_RENDERER=0`, because WebKitGTK disables the renderer for any other value, the empty string included.

### Licences

`cargo deny check licenses`, run inside `src-tauri/` with the root `deny.toml` copied in, passes every crate the spike chose: `pdfium-render` (MIT OR Apache-2.0, through `laminapdf-render-pdfium`), `png`, `image-webp`, `qoi` and `sysinfo` (MIT or MIT OR Apache-2.0). PDFium's binaries are BSD-3-Clause and Apache-2.0. It rejects five crates that Tauri 2.12.5 pulls in, all MPL-2.0:

- **`selectors` 0.38.0, `cssparser` 0.37.0, `cssparser-macros` 0.7.1 and `dtoa-short` 0.3.5,** through `dom_query` 0.28.0 in `tauri-utils`, which parses the app's HTML at build time to inject the CSP;
- **`option-ext` 0.2.0,** through `dirs` 7.0.0 in `tauri` and `wry`.

Any Tauri 2.12 app has them. MPL-2.0 is file-level copyleft, as with UniFFI, for which the owner allowed it per crate on 2026-10-30 (`deny.toml`). The desktop app needs the same decision before it joins the root workspace.

## Observations per OS

- **macOS (M1 Pro):** every transport is fast. The three raw ones moved 920–970 tiles/s at 512 px, DPR 2, and WebP 990, with at most 4 dropped frames per run. WebP gave the lowest webview memory (186 MB peak against 252–275 MB for raw), because compressed bodies are smaller to hold. Two runs stalled once and passed on their retry. An earlier attempt at the plan stopped producing results when the display went to sleep, because WebKit sends no animation frames then; the stall check and `caffeinate` came from it.
- **macOS 15 (hosted, virtual):** a third of the M1 Pro's raw throughput (316 tiles/s); WebP gained 1.2× over raw and PNG 1.3×. QOI, decoded in script, was the slowest transport there and dropped the most frames.
- **Windows:** the raw transports were limited to 160 MB/s, and their p95 ready latency reached 52–107 ms at 512 px, DPR 2. PNG and WebP gave 2.7–2.8× the tiles and halved the zoom fill time (47–63 ms against 141–172 ms at 400 %). WebView2 dropped no frames on any transport. Its frame timing on the runner's Hyper-V display may hide jank that a real display would show. The CPU column reads 0 on Windows: `sysinfo` reported no usage for these processes, so CPU wasn't measured there.
- **Linux:** the raw transports reached about 200 MB/s. WebP gave 2.5–2.7× the tiles, and WebP and QOI dropped the fewest frames (0–3 per run at 512 px, DPR 2).

Five things differed from the docs or needed care:

- **Scheme URL:** `convertFileSrc("", "lamina")` returns the right base on each OS (`lamina://localhost/` or `http://lamina.localhost/`), so the webview code needs no OS check.
- **Fetching the scheme:** `fetch` of a custom scheme works in WebKitGTK as in the others.
- **Channel payloads:** raw `InvokeResponseBody::Raw` payloads arrive in the channel's `onmessage` as an `ArrayBuffer`.
- **Process counting:** `sysinfo` lists every Linux thread as a process with its parent's memory. The first Linux run summed the webview's memory to 20 GB until threads were skipped.
- **Hidden GPU names:** WebKit reports its WebGL renderer as "Apple GPU" on Linux too, so the GPU comes from `glxinfo` instead.

## Limits of this spike

- **One machine per row, no GPU on the hosted ones.** Windows and Linux ran on 4-core cloud VMs with software rendering. Real users' GPUs will raise the raw transports' ceilings, so the gap to WebP on Windows and Linux is an upper bound. No NVIDIA machine was available: measuring one is left to D2e's CI matrix.
- **A small document set.** Three short fixtures make 40 pages by repetition. Scanned or photo-heavy documents compress less, as the synthetic runs show.
- **Cached renders.** The transports were timed on tiles rendered earlier. A real reader renders on demand, which adds 2.6–5.2 ms per 512 px tile here and makes WebP's 0.8–1.7 ms encode a smaller share.
- **The in-flight limit is part of each result.** The sweep chose *n* by burst throughput, and a low *n* (4) slows zoom fill: the M1 Pro's `scheme-raw` and `scheme-qoi` took 100 ms to fill at 400 % against 50 ms for the others. The single runs at equal *n* = 8 (`results/*-sweep.csv`) agree that WebP leads on every machine, but they reorder the raw transports (the channel comes first on the M1 Pro) and put raw ahead of PNG on macOS 15. One run per cell is too noisy to rank transports that finish within 15 % of each other, so every comparison above uses the 5-run medians.
- **Repeated pages.** The scroll's 40 pages repeat the fixtures' pages, and the webview's bitmap cache serves repeats, so the scroll asked each transport for fewer tiles than 40 distinct pages would. Dropped and blank frames were measured under that lighter load. Burst throughput and ready latency count only tiles the transport carried, so they are unaffected.
- **The channel's design.** `stream_tiles` sends one batch's tiles one after another from one blocking thread, while the other transports serve each request on its own thread. A channel per tile, or parallel sends, might close part of its gap.
- **Some renders in measured runs.** 29 of 900 measured runs rendered between 1 and 31 tiles that the 768 MB cache had dropped, under 4 % of their tiles. Their PDFium time (2.6–5.2 ms a tile) counts as transport time there.
- **Memory on macOS.** WKWebView's processes are launched by the system, not the app, so the probe counts every `com.apple.WebKit` process started after the app. A webview another app opens during a run would be counted too.
- **Simulated DPR.** Both DPRs ran on each display; the M1 Pro's Retina display was the only one at a real DPR 2.
- **Dropped frames on Windows.** WebView2 reported a steady frame interval on the runner's virtual display throughout, so its zeros say little about real displays.

## Reproduce

Needs Rust (the root `rust-toolchain.toml`), Node 22+ and pnpm. On Linux, also `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`, and Xvfb or Weston for a headless display.

```sh
cargo xtask fetch-pdfium                       # prints LAMINA_PDF_PDFIUM_DIR=…
export LAMINA_PDF_PDFIUM_DIR=…
cd spikes/desktop-ipc
scripts/run-all.sh <label> [full|quick]        # macOS, Linux; Windows: scripts\run-all.ps1 <label> [full|quick]
python3 scripts/summarise.py results/ > results/summary.md
```

`run-all` builds the app twice (`pnpm tauri build --no-bundle`, then again with `--config src-tauri/tauri.coi.conf.json`), runs the isolation check with each build, and runs the plan with the plain build. The full plan takes about 40 minutes; `quick` takes 2. On macOS, keep the window on screen: the script keeps the display awake and the window stays on top, but a locked screen or another Space stops WebKit's animation frames. On Linux, run it under the display to measure: `xvfb-run -a -s "-screen 0 1920x1080x24"` for X11, or a running Weston with `GDK_BACKEND=wayland`. Set `WEBKIT_DISABLE_DMABUF_RENDERER` to `0` or `1`. `cargo build --release` inside `src-tauri/` builds the Rust side alone; it needs `dist/` from `pnpm build` first.

The hosted runs came from a temporary workflow on the spike's branch (run [42649542710](https://github.com/laudebugs/laminapdf/actions/runs/42649542710)), deleted before merge. Spikes are not in CI.
