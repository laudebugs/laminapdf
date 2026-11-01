# `@laminapdf/wasm`

lamina's WebAssembly modules and their [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) glue. The package is private: use [`@laminapdf/sdk`](../sdk/README.md), which is built on it. This package is its raw layer.

| Export | File | What it has |
| --- | --- | --- |
| `@laminapdf/wasm/base` | `dist/lamina_base.js`, `.d.ts`, `lamina_base.wasm` | Open, read, edit with ops, undo, save (save jobs, warnings); build vector and image pages with links, outline, labels, metadata |
| `@laminapdf/wasm/text` | `dist/lamina_text.js`, `.d.ts`, `lamina_text.wasm` | All of base, plus fonts (`fontBytes`, `standardFont`), `text` and `textBox`, and `WasmPage.text` |
| `@laminapdf/wasm/extract` | `dist/lamina_extract.js`, `.d.ts`, `lamina_extract.wasm` | All of base, plus `WasmPage.text` (text extraction) |

Each glue module exports a default async `init(input)` (a URL, `Response`, bytes or `WebAssembly.Module`) and `initSync({ module })`; after either, every call is synchronous. Errors are thrown as lamina's error object, the same fields as `LaminaError`'s. Paths are flat `Float64Array`s (verb, numbers…: `0 x y` moveTo, `1 x y` lineTo, `2 x1 y1 x2 y2 x y` cubicTo, `3 x1 y1 x y` quadTo, `4` close, `5 x y w h` rect, `6 x y w h` ellipse, `7 cx cy r` circle), colours arrays of 1, 3 or 4 components.

## For contributors

The modules are built from [`crates/laminapdf-wasm`](../../crates/laminapdf-wasm) by `cargo xtask wasm` into `dist/`, which is generated and gitignored; this `package.json` is committed so `pnpm install --frozen-lockfile` works before a build ([ADR 0002](../../docs/decisions/0002-binding-strategy.md) §2). The package design is [ADR 0016](../../docs/decisions/0016-package-design.md) and the error object [ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md) §1.

```sh
cargo xtask wasm   # build both modules into dist/, run the wasm-bindgen-test suite and test/smoke.test.ts (Node)
cargo xtask size   # raw/gzip/brotli per module and min+gzip glue vs size-budgets.json; writes target/size-report.json
```

`size-budgets.json` holds the budgets ADR 0016 §6 decides and the baseline the 5 % growth report compares with (`cargo xtask size --update-baseline` rewrites the baseline).
