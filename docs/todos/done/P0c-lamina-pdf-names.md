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
- TypeScript names: `LaminaError` → `LaminaPdfError`, `LaminaWarning` → `LaminaPdfWarning`, `isLaminaError` → `isLaminaPdfError`, and the other `Lamina*` TypeScript names.
- Schema IDs: `lamina.error/1` → `laminapdf.error/1`, and every other `lamina.<name>/<n>`.
- Environment variables: `LAMINA_*` → `LAMINA_PDF_*`.
- `[package.metadata.lamina]` → `[package.metadata.laminapdf]`.
- The planned command-line program `lamina` → `laminapdf`, with its MCP tools (`lamina_info` → `laminapdf_info`, …).
- `Cargo.toml`: `repository` → `https://github.com/laudebugs/laminapdf`, and `homepage` → `https://laminapdf.com`.

**Out of scope**

- Rust type names (`LaminaId`) and the planned Swift and Kotlin names (`LaminaView`, `LaminaViewer`): only TypeScript names change.
- The product name inside plans, research notes, ADR bodies and todos, which are the project's working notes; done todos, frozen spikes, `CHANGELOG.md` and text written into PDFs and fixtures, as in P0b.
- Cloudflare resource names (`lamina-web`, `lamina-symbols`): renaming them is an owner action in the Cloudflare account.
- Removing the ratchet's rename exemption: P0b's post-merge check, once no open branch predates this rename.

## Tasks

- [x] Rename the public names listed above (a token-exact script), keeping PDF and fixture text
- [x] Regenerate what is generated (codes, schemas, explain pages) and format
- [x] Teach the ratchet's rename exemption the new names
- [x] A changeset for the npm packages (renamed exports and files)
- [x] Record the decision in ADR 0015 and ADR 0026

## Acceptance criteria

- [ ] `git grep` finds none of the old names outside the out-of-scope list
- [ ] Every PDF golden and checksummed input is unchanged byte for byte
- [ ] The docs site calls the product Lamina PDF, and its build passes
- [ ] CI green

## Post-merge checks

- [ ] Rename `LAMINA_*` variables set outside the repository (the Linux runner's environment, local shells) to `LAMINA_PDF_*`

## Tests

The existing suites, unchanged apart from the names; the error-contract tests check the new schema ID in both languages.

## Docs to update

- [x] ADR 0015 and ADR 0026 (amendments), [architecture §8 Q3](../../plans/architecture.md#8-open-questions-for-the-team), `CLAUDE.md`, READMEs and the docs site

## Risks and open questions

- Open branches conflict with the rename; Rex and Dona hold new work until it lands.
- A `LAMINA_*` variable set outside the repository stops working until it is renamed.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, added this file in `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
