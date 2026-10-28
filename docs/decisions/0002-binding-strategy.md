# 0002 — Binding strategy

- Status: Accepted
- Date: 2026-10-28

## Context

[ADR 0001](0001-core-language-and-licence.md) puts the engine in a bytes-in/bytes-out Rust core. Each shell needs a way to call it:

| Consumer | Runtime | Needs |
| --- | --- | --- |
| `@lamina/pdf`, web app worker | Browsers, Cloudflare Workers (workerd), Node, Deno, Bun | WASM with TypeScript types |
| Desktop app | Tauri 2 (Rust process) | Native linking, no binding layer |
| Mobile (later, [architecture §8](../plans/architecture.md#8-open-questions-for-the-team) Q4) | Swift, Kotlin | Generated native bindings |
| Node users wanting native speed | Node | Optional |

The spike proved the mechanics with no glue library: a hand-written four-function C ABI (`alloc`, `dealloc`, `hello_pdf`, `page_count`) loaded with plain `WebAssembly.instantiate` returned byte-identical output ([research ch. 07 §4](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned); [`wasm.rs`](../../spikes/pdf-basics/rust/src/wasm.rs)). It also showed the cost: `unsafe` pointer handling, manual memory management across the boundary, and no types. The spike recommended wasm-bindgen for the web and UniFFI for mobile, but did not test either generator ([ch. 07 §5](../research/pdf/07-spike-rust-and-typescript.md#5-implications-for-lamina-recommendation-for-discussion)).

The architecture research checked the 2026 state of each option ([report: repository section](../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus); [repo notes §3](../research/architecture/notes/repo-and-packaging.md)):

- The `rustwasm` GitHub org was archived in 2025; **wasm-bindgen** moved to a new org and is actively released. Many projects now call `wasm-bindgen-cli` and `wasm-opt` directly instead of `wasm-pack`.
- Upstream **tsify** is maintained again (0.5.6) and emits `.d.ts` types for serde data structs. Bitwarden's SDK guidance is tsify for data, `#[wasm_bindgen]` classes for live handles.
- **napi-rs v3** WASM output targets `wasm32-wasip1-threads`, which needs COOP/COEP in browsers, so it is a Node-native option, not the browser path.
- **WASI 0.3** was ratified in June 2026 but jco support is still landing, so the Component Model is not ready for a browser-first library.
- **UniFFI** generates Swift, Kotlin and Python. 0.32 changed Kotlin byte buffers (`ForeignBytes`), so its version must be pinned and upgraded deliberately.

## Decision

1. **Web, Workers and Node: wasm-bindgen + tsify** in an edge crate (`crates/lamina-wasm`, a `cdylib`). It targets `wasm32-unknown-unknown`, with no WASI and no threads in the baseline build. Live objects (`Document`, `Page`, later `Session`) are `#[wasm_bindgen]` classes. Plain data (options, warnings, metadata, results) derives `Tsify` with serde so TypeScript gets exact types. Byte buffers cross the boundary as `Uint8Array`, never as serde-serialised arrays. Core errors map 1:1 to a JS `Error` with a stable `code` ([code organization §5](../guidelines/code-organization.md#5-error-handling-by-layer)). The TypeScript package (`@lamina/pdf`) is an idiomatic API over the generated glue, synchronous after an async `init()` that accepts a URL or a precompiled `WebAssembly.Module` (which Workers require). Export conditions and size budgets are ADR 0016.

2. **Build pipeline, driven by `cargo xtask wasm`** ([ADR 0015](0015-monorepo-layout-and-tooling.md)):

   ```
   cargo build -p lamina-wasm --target wasm32-unknown-unknown --release
     → wasm-bindgen (CLI, pinned to exactly the wasm-bindgen crate version in Cargo.lock)
     → wasm-opt -Oz (binaryen, pinned)
     → packages/wasm (build output, gitignored)
   ```

   The xtask checks the CLI version against `Cargo.lock` and fails on mismatch. CI runs the same command and enforces the size budget on its output.

3. **Handle-based façade.** The public surface that bindings wrap is designed once, in the `lamina` façade crate: owned types, opaque handles, byte buffers in and out, no borrowed lifetimes or generics across the boundary. It maps one-to-one onto wasm-bindgen classes, Tauri commands and UniFFI objects ([report: reference architecture](../research/architecture/app-and-package-architecture.md#the-reference-architecture-is-one-engine-with-three-shells)). Binding crates contain only conversion and error mapping, never PDF logic.

4. **Desktop links the crates natively.** `apps/desktop/src-tauri` depends on the library crates by path; it does not use WASM. Tauri commands are app code, not a binding crate (ADR 0008).

5. **Mobile: UniFFI, later.** When mobile is in scope, a `crates/lamina-ffi` edge crate exposes the same façade to Swift and Kotlin through UniFFI (pinned version). Not built now.

6. **Deferred:** **napi-rs** (native Node addon) until a measured Node use case needs more than WASM gives; **Component Model / jco** until browser and workerd support is usable, re-evaluated in 2027. Python (PyO3/maturin or UniFFI's Python output), recommended early by [ch. 05 §8.6](../research/pdf/05-creating-pdfs-from-code.md#86-recommendation), gets its own ADR when scheduled; it will wrap the same façade.

7. **The spike's C-ABI shim is superseded.** Its hand-written `alloc`/`dealloc` exports and `unsafe` pointer code are not carried into product code. Any future C ABI (for languages UniFFI does not cover) would be generated or a separate, reviewed edge crate.

## Consequences

- TypeScript users get generated, accurate types and automatic memory management for handles (wasm-bindgen's `FinalizationRegistry` and `Symbol.dispose` support), with an explicit `free()`/`using` for deterministic release of large documents.
- `unsafe` stays inside generated code; lamina's own edge crates need little or none ([`rust.md` Unsafe](../guidelines/rust.md#unsafe)).
- The same WASM file runs in browsers, Workers, Node, Deno and Bun. There is no per-runtime build, at the cost of not using threads or native Node speed in the baseline.
- wasm-bindgen glue adds some size over the spike's raw C ABI; the size budget in CI makes that visible. `wasm-opt` recovers part of it.
- Upgrading wasm-bindgen means upgrading the crate and CLI together; Renovate groups them ([ADR 0019](0019-supply-chain-and-licence-hygiene.md)).
- Designing the façade around handles and owned buffers costs some ergonomics for Rust callers, who may also use the core crates directly.

## Alternatives considered

- **Keep a hand-written C ABI** (the spike's approach). Smallest output, but untyped, `unsafe` on both sides, and every function needs manual marshalling. Fine for a proof, not for ~180 operations.
- **wasm-pack.** Still released under the new org, but it hides the pipeline and its bundled tool versions; calling `wasm-bindgen-cli` and `wasm-opt` from xtask gives exact pinning and one task entry point.
- **serde-wasm-bindgen everywhere** (all values serialised). Simple, but copies large buffers and loses identity for live objects; tsify plus classes avoids both.
- **napi-rs as the primary JS binding.** Best for Node-only native modules; its WASM target needs threads and COOP/COEP, which breaks embeds, OAuth popups and Workers. Deferred.
- **Component Model (WIT + jco).** The likely long-term standard, but tooling and runtime support were not ready in 2026.
- **Diplomat or cbindgen for all languages.** One generator for many targets, but weaker TypeScript and WASM ergonomics than wasm-bindgen + tsify, and UniFFI already covers Swift and Kotlin.
