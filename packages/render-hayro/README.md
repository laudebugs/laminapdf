# @laminapdf/render-hayro

lamina's second web renderer is hayro, a pure-Rust engine compiled to its own WebAssembly module behind [`@laminapdf/render`](../render/README.md)'s `RenderBackend`. It checks the default renderer and is offered in the viewer's debug menu. It is private, and `@laminapdf/sdk` never depends on it.

```ts
import { initHayro } from "@laminapdf/render-hayro";

const hayro = await initHayro(); // or a URL, Response, bytes or WebAssembly.Module
```

- **The module:** it ships as `@laminapdf/wasm/render-hayro`, with hayro 0.8.0's built-in standard 14 fonts and CMaps.
- **Input:** hayro needs the whole file. `open` reads the `RenderSource` in one pass in 64 KiB chunks, and every missing range goes into one `data-pending`. Then it moves the bytes into the module, which borrows them without a second copy. The input limit is `maxInputBytes` (1 GiB by default).
- **Geometry:** tile geometry and validation are the Rust planner's (`TilePlan`), the same as the native hayro and PDFium adapters.
- **Known differences from PDFium:**
  - no print intent;
  - no switch for widgets alone;
  - a small share of pages with font or annotation differences.

## For contributors

It is the second renderer of [ADR 0004](../../docs/decisions/0004-render-backend.md), the oracle in CI; its known differences are ADR 0004 §6, measured by [C1a](../../docs/todos/done/C1a-hayro-parity-spike.md), and the nightly oracle diff ([X3b](../../docs/todos/done/X3b-render-diff-nightly.md)) uses the default flags.

`crates/lamina-render-wasm` (an edge crate, with a flat wasm-bindgen surface) is built by `cargo xtask wasm` as `lamina_render_hayro.{js,d.ts,wasm}` into `packages/wasm/dist/`, next to lamina's own modules. It goes through the same `wasm-bindgen` and `wasm-opt` pipeline, with its own size budget (`render_hayro` in `packages/wasm/size-budgets.json`).

### Tests

- **Node:** `cargo xtask wasm` builds the module, then runs `pnpm --filter @laminapdf/render-hayro run test:wasm`: the RenderBackend contract in Node.
- **Browsers:** `cargo xtask pdf-runtimes <browser>` runs the contract in Chromium, Firefox and WebKit, with [C2b](../../docs/todos/done/C2b-lamina-render-crate.md)'s hayro goldens at SSIM ≥ 0.99 (the same code compiled to wasm32).
