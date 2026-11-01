# P0c — Call the product Lamina PDF and rename its public names to match

| Field | Value |
| --- | --- |
| ID | `P0c` |
| Status | `done` |
| Plan | [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [§8 Q3](../../plans/architecture.md#8-open-questions-for-the-team) |
| Round / milestone | Publishing (deferred) |
| Depends on | [P0b](./P0b-crate-names-laminapdf.md) |
| Blocks | [P1](../ready/P1-reserve-names-and-trusted-publishing.md) |
| ADRs | Amends [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (names), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (the error schema ID) |
| Branch | `refactor/p0c-lamina-pdf-names` |
| PR title | `refactor: call the product Lamina PDF and rename its public names` |
| Size | XL in lines, but a mechanical rename |
| PR | [#179](https://github.com/laudebugs/laminapdf/pull/179) |

## Goal

Give the product one name, Lamina PDF, and make every public name follow it before anything is published, while renaming is still free. The crates became `laminapdf-*` in [P0b](../done/P0b-crate-names-laminapdf.md); this todo renames what P0b kept on purpose.

## Context

- The owner's decision of 2026-11-01: the product is **Lamina PDF**, on the docs site too; the website is laminapdf.com and the repository `laudebugs/laminapdf`.
- [P0b](../done/P0b-crate-names-laminapdf.md) lists what kept the old name; [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) defines the error object and its schema ID.

## Scope

**In scope**

- The product name in what readers see: the docs site, the READMEs, `docs/errors`, the guidelines and `CLAUDE.md`: lamina → Lamina PDF.
- `@laminapdf/wasm`'s modules: `lamina_base`, `lamina_text`, `lamina_extract`, `lamina_render_hayro` → `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract`, `lamina_pdf_render_hayro`.
- TypeScript names: `LaminaError` → `LaminaPdfError`, `LaminaWarning` → `LaminaPdfWarning`, `isLaminaError` → `isLaminaPdfError`.
- Rust types and the planned Swift and Kotlin names, which the owner added during review: `LaminaId` → `LaminaPdfId`, `LaminaView` → `LaminaPdfView`, ….
- Cloudflare resources: the docs Worker `lamina-docs` → `lamina-pdf-docs`, and the planned Workers and R2 buckets (`lamina-web`, `lamina-web-staging`, `lamina-releases`, `lamina-staging`, `lamina-nightly`, `lamina-symbols`) → `lamina-pdf-*`. Also added during review.
- Schema IDs: `lamina.error/1` → `laminapdf.error/1`, and every other `lamina.<name>/<n>`.
- Environment variables: `LAMINA_*` → `LAMINA_PDF_*`.
- `[package.metadata.lamina]` → `[package.metadata.laminapdf]`.
- The planned command-line program `lamina` → `laminapdf`, with its MCP tools (`lamina_info` → `laminapdf_info`, …).
- `Cargo.toml`: `repository` → `https://github.com/laudebugs/laminapdf`, and `homepage` → `https://laminapdf.com`.

**Out of scope**

- The product name inside plans, research notes, ADR bodies and todos, which are the project's working notes; done todos, frozen spikes, `CHANGELOG.md` and text written into PDFs and fixtures, as in P0b.
- Other infrastructure names: the Linux runner's container image, name and label (`lamina-ci-runner-linux`, `lamina-self-hosted-linux-1`).
- Removing the ratchet's rename exemption: P0b's post-merge check, once no open branch predates this rename.

## Tasks

- [x] Rename the public names listed above (a token-exact script), keeping PDF and fixture text
- [x] Regenerate what is generated (codes, schemas, explain pages) and format
- [x] Teach the ratchet's rename exemption the new names
- [x] The owner's additions during review: Rust, Swift and Kotlin type names, and the Cloudflare names (with the Worker renamed in place at merge)
- [x] A changeset for the npm packages (renamed exports and files)
- [x] Record the decision in ADR 0015 and ADR 0026

## Acceptance criteria

- [x] `git grep` finds none of the old names outside the out-of-scope list
- [x] Every PDF golden and checksummed input is unchanged byte for byte
- [x] The docs site calls the product Lamina PDF, and its build passes
- [x] CI green

## Post-merge checks

- [ ] Rename `LAMINA_*` variables set outside the repository (the Linux runner's environment, local shells) to `LAMINA_PDF_*`
- [ ] Cloudflare: the first production deploy creates the Worker `lamina-pdf-docs`; move the custom domain `docs.laminapdf.com` to it if the deploy can't, then delete the old `lamina-docs` Worker (owner approval)

## Tests

The existing suites, unchanged apart from the names; the error-contract tests check the new schema ID in both languages.

## Docs to update

- [x] ADR 0015 and ADR 0026 (amendments), [architecture §8 Q3](../../plans/architecture.md#8-open-questions-for-the-team), `CLAUDE.md`, READMEs and the docs site

## Risks and open questions

- Open branches conflict with the rename; Rex and Dona hold new work until it lands.
- A `LAMINA_*` variable set outside the repository stops working until it is renamed.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, added this file in `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
