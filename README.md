# Lamina PDF

[![npm](https://img.shields.io/npm/v/@laminapdf/sdk/alpha)](https://www.npmjs.com/package/@laminapdf/sdk) [![crates.io](https://img.shields.io/crates/v/laminapdf)](https://crates.io/crates/laminapdf)

Lamina PDF creates, reads, edits and saves PDFs from code, in Rust and in TypeScript on every JavaScript runtime.

**Documentation:** [docs.laminapdf.com](https://docs.laminapdf.com) has quick starts for each runtime, guides, examples and the API reference.

```ts
import { Builder, Font, init } from "@laminapdf/sdk";

await init({ text: true });
const builder = new Builder();
const page = builder.page("A4");
page.surface().text("Hello, world!", { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
page.finish();
const bytes = builder.finish().save(); // a Uint8Array holding the PDF
```

> **Status:** alpha, published on npm as `@laminapdf/sdk` (the `alpha` tag) and on crates.io as `laminapdf`. Anything can change between alphas, so pin the exact version you use; the [stability policy](https://docs.laminapdf.com/reference/stability/) shows how and says what each version promises.

## What's here

Lamina PDF's core is Rust. The `laminapdf` crate is its API, over the core crates `laminapdf-syntax`, `laminapdf-cos`, `laminapdf-model`, `laminapdf-ops`, `laminapdf-save`, `laminapdf-content`, `laminapdf-text` and `laminapdf-build`. Its WebAssembly build is `@laminapdf/wasm`, and `@laminapdf/sdk` is the TypeScript API over it. The web viewer is `@laminapdf/viewer`, with its renderers in `@laminapdf/render`, `-pdfium` and `-hayro`.

The docs site is built from [`apps/docs/`](apps/docs/README.md): run `pnpm install`, then `pnpm --filter @laminapdf/docs dev`. Decisions are recorded as [ADRs](docs/decisions/README.md); the plans and the todo board are in [`docs/`](docs/README.md).

## Repository layout

| Path | What lives there |
| --- | --- |
| [`apps/docs/`](apps/docs/README.md) | The documentation site (Astro + Starlight). |
| [`docs/`](docs/) | All project documentation — plans, guidelines, references, research notes and decision records. Start at [`docs/README.md`](docs/README.md). |
| [`CLAUDE.md`](CLAUDE.md) | Working agreements for Claude Code (and a good summary for humans too). |

## Licence

Licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([`LICENSE-MIT`](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Contributing

All changes after the initial commit go through pull requests against `main`. See [`docs/guidelines/contributing.md`](docs/guidelines/contributing.md).
