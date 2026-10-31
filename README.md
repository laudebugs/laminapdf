# lamina

A platform for anything and everything PDF — creating, reading, rendering, editing, filling, signing and transforming PDF documents from code.

> **Status:** foundations. Research is done and the first architecture decisions are accepted ([ADRs](docs/decisions/README.md)); Round A is under way: the monorepo skeleton (Cargo + pnpm workspaces, `cargo xtask`, CI) is in place, and the first core crates exist: `lamina-syntax` and `lamina-cos` (reading and writing PDF file structure, ported from the research spike), `lamina-model` (typed views and stable IDs), `lamina-ops` (typed edits with transactions and undo) and `lamina-content` (a read-only content-stream interpreter). No packages or apps yet.

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
