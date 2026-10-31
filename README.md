# lamina

A platform for anything and everything PDF — creating, reading, rendering, editing, filling, signing and transforming PDF documents from code.

> **Status:** early, nothing published yet. The core crates read, edit, save and generate PDFs (`lamina-syntax`, `lamina-cos`, `lamina-model`, `lamina-ops`, `lamina-save`, `lamina-content`, `lamina-text`, `lamina-build`), behind the `lamina` façade. Its WebAssembly build (`@lamina/wasm`) and the TypeScript API over it (`@lamina/pdf`) are private packages, and the docs site is the first app (`apps/docs`). Decisions are recorded as [ADRs](docs/decisions/README.md); the plans and the todo board are in [`docs/`](docs/README.md).

## Documentation

The docs site (quick starts for the browser, Node, Deno, Bun, Cloudflare Workers and Rust, guides, the examples gallery and the `@lamina/pdf` API reference) is built from [`apps/docs/`](apps/docs/README.md): `pnpm install`, then `pnpm --filter @lamina/docs dev`. It deploys to Cloudflare Workers (workers.dev until a domain is chosen) once the owner adds the deploy secrets; see [its README](apps/docs/README.md#deploying).

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
