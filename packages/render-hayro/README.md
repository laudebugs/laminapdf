# @laminapdf/render-hayro

ADR 0004's second web renderer, the CI oracle and the debug-menu choice: `lamina-render`'s pure-Rust hayro adapter, compiled to its own WebAssembly module, behind [`@laminapdf/render`](../render/README.md)'s `RenderBackend`. Private; `@laminapdf/pdf` never depends on it.

```ts
import { initHayro } from "@laminapdf/render-hayro";

const hayro = await initHayro(); // or a URL, Response, bytes or WebAssembly.Module
```

- **The module:** `crates/lamina-render-wasm` (an edge crate, with a flat wasm-bindgen surface) is built by `cargo xtask wasm` as `lamina_render_hayro.{js,d.ts,wasm}` into `packages/wasm/dist/`, next to lamina's own modules (`@laminapdf/wasm/render-hayro`). It goes through the same `wasm-bindgen` and `wasm-opt` pipeline, with its own size budget (`render_hayro` in `packages/wasm/size-budgets.json`). It is hayro 0.8.0 with its built-in standard 14 fonts and CMaps.
- **Input:** hayro needs the whole file. `open` reads the `RenderSource` in one pass in 64 KiB chunks, and every missing range goes into one `data-pending`. Then it moves the bytes into the module, which borrows them without a second copy. The input limit is `maxInputBytes` (1 GiB by default).
- **Geometry:** tile geometry and validation are the Rust planner's (`TilePlan`), the same as the native hayro and PDFium adapters.
- **Known differences from PDFium** (ADR 0004 §6; X3b's oracle diff uses the default flags):
  - no print intent;
  - no switch for widgets alone;
  - a font or annotation tail that C1a measured.

## Tests

- **Node:** `cargo xtask wasm` builds the module, then runs `pnpm --filter @laminapdf/render-hayro run test:wasm`: the RenderBackend contract in Node.
- **Browsers:** `cargo xtask pdf-runtimes <browser>` runs the contract in Chromium, Firefox and WebKit, with C2b's hayro goldens at SSIM ≥ 0.99 (the same code compiled to wasm32).
