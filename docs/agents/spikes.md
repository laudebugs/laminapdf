# Research spikes

How to run each spike in `spikes/`. The results are in [`docs/README.md`](../README.md#spikes) and each spike's README.

Research spikes (`spikes/`) are throwaway-quality APIs used to inform decisions, not the product. They are outside both workspaces; CI still runs the pdf-basics spike's checks, in the fast lane's `msrv-and-spike` job.

`spikes/krilla-fit/` — krilla vs a builder over `laminapdf-cos` (Round B1); standalone, not in CI: `scripts/fetch-font.sh` once, then `cargo run --release` prints the measurements (see its README).

`spikes/lamina-ffi/` — the `laminapdf` façade bound with UniFFI `=0.32.2` (Round L6a, ADR 0029); standalone, not in CI: `cargo test` (Rust), `scripts/build-kotlin.sh` (Kotlin JVM tests; JDK 17+ and Gradle 8), `scripts/run-on-mac.sh` on Apple silicon with Xcode (XCFramework, Swift tests, Android NDK libraries, 16 KB check, AAR, sizes into `target/mac-report.md`; see its README).

`spikes/desktop-ipc/` — a Tauri 2.12 app that moves PDFium tiles into the webview four ways and checks cross-origin isolation (D1a, ADR 0008); standalone, not in CI: `cargo xtask fetch-pdfium`, then `scripts/run-all.sh <label> [full|quick]` (Windows: `scripts\run-all.ps1`) and `python3 scripts/summarise.py results/` (see its README).

`spikes/desktop-print/` — a Tauri 2.12 app that prints three generated fixtures through the webview, through PDFium rasters and the OS print API, and through each OS's vector path, then scores the PDFs against the fixtures (D1b); standalone, not in CI: `cargo xtask fetch-pdfium`, then `scripts/run-all.sh <label>` (Windows: `scripts\run-all.ps1`; Linux under `xvfb-run` with CUPS and `cups-pdf`) and `python3 scripts/summarise.py results/` (see its README).

`spikes/docs-twoslash/` — Twoslash hover types (`expressive-code-twoslash` 0.6.1) on one docs page, measured in Chromium, Firefox and WebKit against the docs' CSP and Q4i's (Q4p); throwaway, not in CI: a patch for `apps/docs` and `measure.mjs` (see its README, which recommends keeping Twoslash out).

`spikes/auth/` — Better Auth 1.7.14 and Auth.js 0.41.3 on a Worker with local D1, `@clerk/backend` on a Worker, and a Rust loopback + PKCE desktop client (H1, ADR 0011); standalone, not in CI: each folder is its own pnpm project (`pnpm install --ignore-workspace`, `pnpm dev` on `wrangler dev --local`, then `node check.mjs` with Playwright's Chromium), `desktop/` a Cargo project of its own (see its README).

`spikes/flow-layout/` — our own paginator over `Surface`, Typst 0.15.1 embedded, and Typst's frames drawn through `Surface` (T4a, ADR 0032); standalone, not in CI. Typst needs `skrifa` 0.42.1 where the workspace pins `=0.42.0`, so `scripts/run.sh <command>` moves the pin for one command and restores the root `Cargo.toml`: `scripts/run.sh cargo run --release` (the PDFs in `target/flow-layout/`), `scripts/run.sh scripts/wasm.sh` (WebAssembly sizes), then `node scripts/node-time.mjs target/wasm/own.wasm` (see its README).

`spikes/content-edit/` — an editable list of content operators with source spans, grouped into objects, and two encoders measured against each other (E7a, ADR 0022); standalone, not in CI. After `cargo xtask corpus fetch`, `cargo build --release` in the spike, then pass it PDFs (`find ../../target/corpus/pdfjs -name '*.pdf' -print0 | xargs -0 target/release/lamina-content-edit-spike`; `SPIKE_DEBUG=1` prints each mismatch).

`spikes/pdfium-wasm/` — `@embedpdf/pdfium` beside `@laminapdf/sdk` in one web worker (Round C1b); standalone, its own pnpm project, not in CI; frozen before the `@laminapdf` rename, so it still imports `@lamina/pdf` from `vendor/lamina-*-0.0.0.tgz` and needs that rename (and its lockfile regenerated) before a re-run: `cargo xtask wasm`, then `sh scripts/pack-lamina.sh && pnpm install && node scripts/make-fixtures.mjs && pnpm exec playwright test && node scripts/summarise.mjs` (Chromium, Firefox, WebKit; see its README).

`spikes/hayro-parity/` — hayro vs PDFium render parity (Round C1a); standalone, not in CI: `scripts/fetch-pdfium.sh && scripts/fetch-corpora.sh` once (downloads into gitignored folders), then `cargo run --release` writes `out/dpi96/results.csv` and `summary.md` (see its README).

`spikes/pdf-basics/` — the same minimal PDF core in Rust and TypeScript (see its `SPEC.md`):

```sh
# Rust (needs rustup; targets wasm32-unknown-unknown, aarch64-apple-ios, aarch64-linux-android)
cd spikes/pdf-basics/rust
cargo test
cargo clippy --all-targets --all-features --locked -- -D warnings
scripts/check-targets.sh                               # everything: cross-target checks, wasm build, fmt, clippy, tests
cargo build --release --target wasm32-unknown-unknown  # wasm loaded by the TS tests

# TypeScript (Node ≥ 20, pnpm)
cd spikes/pdf-basics/ts
pnpm install
pnpm typecheck && pnpm lint && pnpm test               # WASM interop tests run once the Rust wasm is built
pnpm size                                              # min+gz bundle size
```
Rust code follows the `rust-best-practices` skill (no `unwrap`/`expect` in library code, clippy clean with `-D warnings`). Library code in both languages is bytes-in/bytes-out: no file system, network or Node-only APIs, so it runs on web, mobile and desktop.
