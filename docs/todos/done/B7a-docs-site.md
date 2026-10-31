# B7a — Docs site (`apps/docs`): Starlight, quick starts, guides and TypeDoc, with per-PR previews

| Field | Value |
| --- | --- |
| ID | `B7a` |
| Status | `done` |
| Plan | [pdf-library §7 B7](../../plans/pdf-library.md#b7-docs-examples-and-first-release), [§6](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2b](./B2b-standard-security-decryption.md), [B6b](./B6b-lamina-pdf-package.md), [B6c](./B6c-runtime-matrix.md) |
| Blocks | [B7b](../ongoing/B7b-versioning-and-0-1-0-alpha.md), [B7c](./B7c-docs-custom-domain.md), [L5a](../ready/L5a-agent-docs.md), [U5a](../ready/U5a-pricing-and-promises-page.md), [U6g](./U6g-docs-site-paper.md) |
| ADRs | Follows [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (apps in the pnpm workspace), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (npm licences), 0016 (API names); Cloudflare deploy conventions until ADR 0011 (web C2) |
| Branch | `docs/b7a-docs-site` |
| PR title | `feat(docs): Starlight docs site with quick starts, guides and TypeDoc, deployed per PR` |
| Size | L (≈ 600–1,000 lines of config, MDX and workflow) |
| PR | [#117](https://github.com/laudebugs/lamina/pull/117) |

## Goal

Publish usable documentation for `lamina` and `@lamina/pdf` before the first tag: a Starlight site in `apps/docs` with a quick start per runtime, guides for every M1 feature, TypeDoc API reference for the TS package, links to rustdoc, and runnable examples — deployed to Workers static assets with a preview URL on every PR that touches it.

## Context

- [pdf-library §6](../../plans/pdf-library.md#6-docs-examples-and-publishing): Starlight (MIT), `docs.laminapdf.com`, TypeDoc, docs.rs (later; locally `cargo doc` for now since nothing is published).
- [Cloudflare notes](../../research/architecture/notes/cloudflare.md): Workers static assets, preview URLs.
- Examples from [B5c](../done/B5c-lamina-facade-and-examples.md) (Rust) and [B6b](../done/B6b-lamina-pdf-package.md)/[B6c](../done/B6c-runtime-matrix.md) (TS, Worker).

## Scope

**In scope**

- `apps/docs` (Astro + Starlight), in the pnpm workspace, Biome + typecheck in `cargo xtask check ts`.
- Pages: Overview; Install (local tarball and path/git deps while unpublished — wording updated by [P2](../ready/P2-first-registry-prerelease.md)); Quick start for browser/bundler, Node, Deno, Bun, Cloudflare Workers, Rust; Guides: fonts and text (shaping, scripts, fsType), images and vectors, links/outlines/labels/metadata, open/edit/save (`transact`, undo, save modes, `SaveConstraint`), encrypted files (password), errors and warnings (code table generated from `fixtures/codes.json`), limits; API: TypeDoc output for `@lamina/pdf` (via `starlight-typedoc` or a generated folder), link to rustdoc built by CI as an artefact; Examples gallery with source and output PDF.
- Code snippets imported from the example files (single source of truth), not copied.
- Deploy: `wrangler` Workers static assets config `apps/docs/wrangler.toml`; GitHub workflow deploying a preview per PR (comment with URL) and production from `main` (domain TBD; workers.dev until the owner picks, architecture §8 Q3).
- Link checking inside the site build (Starlight's) plus repo-wide [X1](../done/X1-docs-link-checker.md) if merged.

**Out of scope** (and which todo covers it)

- Changesets, tag, install verification: [B7b](../ongoing/B7b-versioning-and-0-1-0-alpha.md).
- PDF/UA and PDF/A guides: [B8b](../ready/B8b-conformance-profiles-and-verapdf.md).
- docs.rs publishing: [P2](../ready/P2-first-registry-prerelease.md).

## Tasks

- [x] Scaffold `apps/docs` (pinned Astro/Starlight versions), add to pnpm workspace; npm licence check
- [x] Content pages and generated code table (`cargo xtask gen codes --md`)
- [x] TypeDoc integration; rustdoc artefact link
- [x] `wrangler.toml`, deploy workflow with preview comments (secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID` — owner provides)
- [x] Fast-lane TS checks for `apps/docs`. Nothing to add to the full lane ([X4](../done/X4-full-lane-gate.md)): every path under `apps/`, its Markdown content included, is already code there

## Acceptance criteria

- [x] `pnpm --filter docs build` succeeds with zero broken internal links
- [ ] Every PR touching `apps/docs/`, `packages/pdf/` or examples gets a preview URL comment (this PR's own preview included). **Not yet:** the workflow (`.github/workflows/docs.yml`) is in place, but the repository has no `CLOUDFLARE_API_TOKEN`/`CLOUDFLARE_ACCOUNT_ID` secrets, so it skips with a notice; the owner's steps are in [`apps/docs/README.md`](../../../apps/docs/README.md#what-the-owner-needs-to-do). Tick this once a PR shows its preview comment
- [x] Every code snippet on the site is imported from a tested example file (the Worker, Deno and Bun quick starts from [B6c](../done/B6c-runtime-matrix.md)'s `packages/pdf/examples/worker/` and its Deno/Bun runners)
- [x] The error/warning code table matches `fixtures/codes.json` (generated)

## Post-merge checks

- [ ] The first push to `main` after merge deploys production (workers.dev until the owner picks a domain); link the run and URL in the done todo. Waits for the owner's secrets (above): until then the run skips with a notice

## Tests

Site build in CI; snippet sources are already tested by [B5c](../done/B5c-lamina-facade-and-examples.md)/[B6b](../done/B6b-lamina-pdf-package.md).

## Docs to update

- [x] [pdf-library §10 B7](../../plans/pdf-library.md#10-to-dos): tick `apps/docs` and per-PR preview deploys
- [x] `CLAUDE.md` (layout: `apps/docs`; commands), `README.md` (link to docs)

## Risks and open questions

- Domain and Cloudflare account ownership: needs the owner (architecture §8 Q3); use workers.dev meanwhile.
- Previews for fork PRs must not get secrets; skip deploy for forks.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- `apps/docs` (`@lamina/docs`): Astro 7.3.8, Starlight 0.42.7, starlight-typedoc 0.23.1 with TypeDoc 0.28.20 and typedoc-plugin-markdown 4.14.1 (on TypeScript 6.0.3 from the `typedoc` catalog: TypeDoc needs the compiler API TypeScript 7 lacks), starlight-links-validator 0.27.0, wrangler from the catalog. Astro 7.3.x rather than 7.4 because 7.4.4 requires Node ≥ 22.23.4 and `engineStrict` would refuse an older Node 22 on a runner.
- Pages: overview, install, six quick starts, seven guides, the code reference (from `fixtures/codes.json`, every group by its key), the Rust API page (the fast lane's `rustdoc` artefact), the examples gallery (TS and Rust source, the golden PDFs), the TypeDoc reference.
- Snippets: `<Snippet file="…" />` reads the file at build time from a fixed glob (the TS and Rust examples, the Worker, the Vite, Node, Deno and Bun runners); shell commands are prose, not snippets. The encrypted-files, limits and errors guides needed an example none of the existing ones covered: `packages/pdf/examples/open-untrusted.ts`, tested in `test/examples.test.ts`.
- Headers: ADR 0011 §4's, with `script-src` hashes of the build's inline scripts instead of a plain `'self'`, `style-src 'unsafe-inline'` (Expressive Code's style attributes) and no CSP on the example PDFs; checked in Chromium against `wrangler dev` (no violations, search works).
- `cargo xtask check ts` builds the site; the deploy workflow skips cleanly without the secrets.
- After review: the code table links each code to its page on the site (`/reference/codes/<code>/`, the `docs/errors/` pages); no GitHub links unless built with `LAMINA_DOCS_GITHUB=1` (the repository is private); the preview is built without secrets and deployed by a job that runs no PR code; the `rustdoc` artefact is rendered fresh, on pushes to `main` only.
