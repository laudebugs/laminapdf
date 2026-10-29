# B7a — Docs site (`apps/docs`): Starlight, quick starts, guides and TypeDoc, with per-PR previews

| Field | Value |
| --- | --- |
| ID | `B7a` |
| Status | `ready` |
| Plan | [pdf-library §7 B7](../../plans/pdf-library.md#b7-docs-examples-and-first-release), [§6](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2b](./B2b-standard-security-decryption.md), [B6b](./B6b-lamina-pdf-package.md), [B6c](./B6c-runtime-matrix.md) |
| Blocks | [B7b](./B7b-versioning-and-0-1-0-alpha.md) |
| ADRs | Follows [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (apps in the pnpm workspace), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (npm licences), 0016 (API names); Cloudflare deploy conventions until ADR 0011 (web C2) |
| Branch | `docs/b7a-docs-site` |
| PR title | `feat(docs): Starlight docs site with quick starts, guides and TypeDoc, deployed per PR` |
| Size | L (≈ 600–1,000 lines of config, MDX and workflow) |
| PR | — |

## Goal

Publish usable documentation for `lamina` and `@lamina/pdf` before the first tag: a Starlight site in `apps/docs` with a quick start per runtime, guides for every M1 feature, TypeDoc API reference for the TS package, links to rustdoc, and runnable examples — deployed to Workers static assets with a preview URL on every PR that touches it.

## Context

- [pdf-library §6](../../plans/pdf-library.md#6-docs-examples-and-publishing): Starlight (MIT), `docs.<domain>`, TypeDoc, docs.rs (later; locally `cargo doc` for now since nothing is published).
- [Cloudflare notes](../../research/architecture/notes/cloudflare.md): Workers static assets, preview URLs.
- Examples from [B5c](./B5c-lamina-facade-and-examples.md) (Rust) and [B6b](./B6b-lamina-pdf-package.md)/[B6c](./B6c-runtime-matrix.md) (TS, Worker).

## Scope

**In scope**

- `apps/docs` (Astro + Starlight), in the pnpm workspace, Biome + typecheck in `cargo xtask check ts`.
- Pages: Overview; Install (local tarball and path/git deps while unpublished — wording updated by [P2](./P2-first-registry-prerelease.md)); Quick start for browser/bundler, Node, Deno, Bun, Cloudflare Workers, Rust; Guides: fonts and text (shaping, scripts, fsType), images and vectors, links/outlines/labels/metadata, open/edit/save (`transact`, undo, save modes, `SaveConstraint`), encrypted files (password), errors and warnings (code table generated from `fixtures/codes.json`), limits; API: TypeDoc output for `@lamina/pdf` (via `starlight-typedoc` or a generated folder), link to rustdoc built by CI as an artefact; Examples gallery with source and output PDF.
- Code snippets imported from the example files (single source of truth), not copied.
- Deploy: `wrangler` Workers static assets config `apps/docs/wrangler.toml`; GitHub workflow deploying a preview per PR (comment with URL) and production from `main` (domain TBD; workers.dev until the owner picks, architecture §8 Q3).
- Link checking inside the site build (Starlight's) plus repo-wide [X1](../done/X1-docs-link-checker.md) if merged.

**Out of scope** (and which todo covers it)

- Changesets, tag, install verification: [B7b](./B7b-versioning-and-0-1-0-alpha.md).
- PDF/UA and PDF/A guides: [B8b](./B8b-conformance-profiles-and-verapdf.md).
- docs.rs publishing: [P2](./P2-first-registry-prerelease.md).

## Tasks

- [ ] Scaffold `apps/docs` (pinned Astro/Starlight versions), add to pnpm workspace; npm licence check
- [ ] Content pages and generated code table (`cargo xtask gen codes --md`)
- [ ] TypeDoc integration; rustdoc artefact link
- [ ] `wrangler.toml`, deploy workflow with preview comments (secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID` — owner provides)
- [ ] Add `apps/docs` to the full-lane path filter ([X4](./X4-full-lane-gate.md)) and fast-lane TS checks

## Acceptance criteria

- [ ] `pnpm --filter docs build` succeeds with zero broken internal links
- [ ] Every PR touching `apps/docs/`, `packages/pdf/` or examples gets a preview URL comment (this PR's own preview included)
- [ ] Every code snippet on the site is imported from a tested example file (the Worker, Deno and Bun quick starts from [B6c](./B6c-runtime-matrix.md)'s `packages/pdf/examples/worker/` and its Deno/Bun runners)
- [ ] The error/warning code table matches `fixtures/codes.json` (generated)

## Post-merge checks

- [ ] The first push to `main` after merge deploys production (workers.dev until the owner picks a domain); link the run and URL in the done todo

## Tests

Site build in CI; snippet sources are already tested by [B5c](./B5c-lamina-facade-and-examples.md)/[B6b](./B6b-lamina-pdf-package.md).

## Docs to update

- [ ] [pdf-library §10 B7](../../plans/pdf-library.md#10-to-dos): tick `apps/docs` and per-PR preview deploys
- [ ] `CLAUDE.md` (layout: `apps/docs`; commands), `README.md` (link to docs)

## Risks and open questions

- Domain and Cloudflare account ownership: needs the owner (architecture §8 Q3); use workers.dev meanwhile.
- Previews for fork PRs must not get secrets; skip deploy for forks.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
