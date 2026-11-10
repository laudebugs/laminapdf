---
title: "Install"
description: "Install Lamina PDF from npm or crates.io, and find every published package."
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/install/"
---

# Install

Install Lamina PDF from npm or crates.io, and find every published package.

Lamina PDF is published to npm as [`@laminapdf/sdk`](https://www.npmjs.com/package/@laminapdf/sdk) and to crates.io as [`laminapdf`](https://crates.io/crates/laminapdf). Both are alpha releases, so name the version when you install them. A plain `npm install` or `cargo add` picks a `0.0.0` placeholder that only reserves the name.

## TypeScript: `@laminapdf/sdk`

Install the version these docs describe:

```sh
npm install @laminapdf/sdk@0.2.0-alpha.5
```

`npm install @laminapdf/sdk@alpha` installs the newest alpha, whichever version that is. pnpm, Yarn and Bun take the same name (`pnpm add @laminapdf/sdk@0.2.0-alpha.5`), and Deno takes `npm:@laminapdf/sdk@0.2.0-alpha.5`. The package brings [`@laminapdf/wasm`](https://www.npmjs.com/package/@laminapdf/wasm), its WebAssembly modules, as a dependency.

The package is ESM only. It has one entry per runtime (the `workerd`, `deno`, `browser`, `node` and `default` export conditions), and each entry loads the `.wasm` its own way. The [quick starts](../pages/quick-start/browser.md) show each one.

## Rust: `laminapdf`

Add the crate at the version these docs describe:

```sh
cargo add laminapdf@=0.2.0-alpha.5
```

The command writes `laminapdf = "=0.2.0-alpha.5"` to `Cargo.toml`, which keeps your project on this release until you change it: alphas promise nothing to each other (see the [stability policy](../pages/reference/stability.md)). The default features are `text` (fonts and text, which includes the builder and text extraction) and `saslprep` (passwords for AES-256 files). For reading and editing only, add `--no-default-features`.

## Every published package

Most projects need only the two above. The table links every published package to its registry page:

| Package | Registry | What it holds |
| --- | --- | --- |
| `@laminapdf/sdk` | [npm](https://www.npmjs.com/package/@laminapdf/sdk) | The TypeScript API, for every JavaScript runtime |
| `@laminapdf/wasm` | [npm](https://www.npmjs.com/package/@laminapdf/wasm) | The WebAssembly modules under `@laminapdf/sdk` |
| `@laminapdf/render` | [npm](https://www.npmjs.com/package/@laminapdf/render) | The interface that page renderers implement |
| `@laminapdf/render-pdfium` | [npm](https://www.npmjs.com/package/@laminapdf/render-pdfium) | The default web renderer, built on PDFium |
| `@laminapdf/render-hayro` | [npm](https://www.npmjs.com/package/@laminapdf/render-hayro) | A second web renderer, built on hayro |
| `@laminapdf/viewer` | [npm](https://www.npmjs.com/package/@laminapdf/viewer) | The core of a web PDF viewer, one worker per document |
| `laminapdf` | [crates.io](https://crates.io/crates/laminapdf) | The Rust API: open, read, edit, save and build PDFs |
| `laminapdf-syntax` | [crates.io](https://crates.io/crates/laminapdf-syntax) | Objects, the lexer, the parser and stream filters |
| `laminapdf-cos` | [crates.io](https://crates.io/crates/laminapdf-cos) | File structure: the tolerant reader and the writer |
| `laminapdf-model` | [crates.io](https://crates.io/crates/laminapdf-model) | Typed views of pages, outlines, metadata and labels |
| `laminapdf-ops` | [crates.io](https://crates.io/crates/laminapdf-ops) | Editing ops, transactions, undo and redo |
| `laminapdf-save` | [crates.io](https://crates.io/crates/laminapdf-save) | Incremental and full saves |
| `laminapdf-content` | [crates.io](https://crates.io/crates/laminapdf-content) | The content-stream interpreter |
| `laminapdf-text` | [crates.io](https://crates.io/crates/laminapdf-text) | Fonts, shaping and line breaking |
| `laminapdf-build` | [crates.io](https://crates.io/crates/laminapdf-build) | The builder that generates PDFs |
| `laminapdf-source` | [crates.io](https://crates.io/crates/laminapdf-source) | Byte sources filled range by range, for remote files |
| `laminapdf-render` | [crates.io](https://crates.io/crates/laminapdf-render) | The Rust interface that page renderers implement |
| `laminapdf-render-pdfium` | [crates.io](https://crates.io/crates/laminapdf-render-pdfium) | The native PDFium renderer |

*Each JavaScript package and what it depends on, beside the Rust API.*

In JavaScript, `@laminapdf/sdk` is the TypeScript API, and it depends on `@laminapdf/wasm`, whose WebAssembly the SDK loads as three modules: base, extract and text. `@laminapdf/viewer`, a web PDF viewer, depends on `@laminapdf/sdk`, on `@laminapdf/render`, the interface that renderers implement, and on both renderers. Those renderers are `@laminapdf/render-pdfium`, the default, and `@laminapdf/render-hayro`, which also uses `@laminapdf/wasm`. In Rust, the API is one crate, `laminapdf`, with features instead of modules.

## Try a change before it's released

An unreleased change is only in the repository, so build the packages from a checkout. You need Rust (the toolchain in `rust-toolchain.toml` installs itself), Node 22 or newer and pnpm:

```sh
pnpm install
cargo xtask wasm      # builds packages/wasm/dist/ and packages/sdk/dist/, then tests them
```

Then run `pnpm pack` in `packages/wasm` and in `packages/sdk`, and install both `.tgz` files in one command: `@laminapdf/sdk` depends on `@laminapdf/wasm` at the same version, which the registry may not have yet. In Rust, use a path dependency: `laminapdf = { path = "../laminapdf/crates/laminapdf" }`.

## API docs

- TypeScript: the [`@laminapdf/sdk` reference](https://docs.laminapdf.com/reference/sdk/readme/), generated from the package's TSDoc comments.
- Rust: the [Rust API](../pages/reference/rust.md) page, which links each crate's docs on docs.rs.
