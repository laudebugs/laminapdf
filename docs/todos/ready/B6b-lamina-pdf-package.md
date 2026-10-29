# B6b — `@lamina/pdf`: the TypeScript API, `init()`, export conditions, publint and attw

| Field | Value |
| --- | --- |
| ID | `B6b` |
| Status | `ready` |
| Plan | [pdf-library §7 B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf), [§5 TS sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2b](../done/B2b-standard-security-decryption.md), [B6a](./B6a-lamina-wasm-and-adr-0016.md) |
| Blocks | [B6c](./B6c-runtime-matrix.md), [B7a](./B7a-docs-site.md), [B8a](./B8a-structure-tree.md), [B9c](./B9c-text-extraction-bindings-and-differential.md), [C1b](./C1b-pdfium-wasm-spike.md), [C2c](./C2c-render-packages.md), [C3a](./C3a-viewer-worker-rpc.md), [C4c](./C4c-remote-files-encryption-privacy.md), [C6a](./C6a-thumbnails-outline-page-labels.md), [E1c](./E1c-metadata-ops.md), [E1d](./E1d-edit-session-undo-ui.md) |
| ADRs | Implements 0016 (from [B6a](./B6a-lamina-wasm-and-adr-0016.md)), [0002](../../decisions/0002-binding-strategy.md), [0006](../../decisions/0006-op-log-and-undo.md) §2 (TS transaction handle), [0003](../../decisions/0003-reader-tolerance-and-limits.md) §4 (codes test in TS), [0018](../../decisions/0018-test-strategy.md) §5 (shared expectations) |
| Branch | `feat/b6b-lamina-pdf` |
| PR title | `feat(pdf): @lamina/pdf TypeScript API over @lamina/wasm with init() and export conditions` |
| Size | L (≈ 900–1,300 lines of TS + tests) |
| PR | — |

## Goal

Give TypeScript users the same library as Rust: `init()`, then a synchronous, typed API (`Builder`, `Document`, `Font`, `PdfError` with codes matching Rust) that runs unchanged in browsers, Node, Deno, Bun and Cloudflare Workers, packaged ESM-only with correct export conditions and verified by publint and `attw --pack`.

## Context

- ADR 0016 from [B6a](./B6a-lamina-wasm-and-adr-0016.md); [TypeScript guideline](../../guidelines/typescript.md) (strict, ESM-only, named exports, no Node-only APIs, Biome, Vitest), [§modules and packaging](../../guidelines/typescript.md#modules-and-packaging), [§errors](../../guidelines/typescript.md#errors).
- [ADR 0006 §2](../../decisions/0006-op-log-and-undo.md#2-transactions): `doc.transact(tx => …)` wraps `begin/apply/commit/rollback`; `doc.apply(ops)`.
- [Repo and packaging notes §4](../../research/architecture/notes/repo-and-packaging.md#4-package-design-esm-only-exports-maps-runtime-compatibility-lazy-wasm-bundlers-jsr).
- [Code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer): one `PdfError` class with string codes.

## Scope

**In scope**

- `packages/pdf` (`@lamina/pdf`, private for now): `init(input?)` (URL, `Response`, `ArrayBuffer`/`Uint8Array`, `WebAssembly.Module`; idempotent; text module loaded on first text use or via `init({ text: true })`), `Builder`, `Page`, `Surface`, `Font.fromBytes`, `Document.open(bytes, { password?, strict?, limits? })`, `doc.pages`, `doc.outline`, `doc.info`, `doc.warnings`, `doc.transact(tx => …)`, `doc.apply(ops)`, `doc.undo()/redo()`, `doc.save({ mode })` → `Uint8Array`, `doc.free()` + `Symbol.dispose` support.
- `PdfError` (`code`, `message`, `details`), `PdfWarning`; codes generated from `fixtures/codes.json` into a TS union type.
- Op types re-exported from tsify output; JSON samples in `fixtures/ops/v1` round-trip through TS.
- `package.json` `exports` with conditions `browser`, `workerd`, `node`, `deno`, `default`; `.wasm` shipped as an asset referenced by `new URL(..., import.meta.url)`; Workers path imports a `WebAssembly.Module`.
- Build with `tsc` (or tsup/tsdown — pick per guideline) to `dist/`; `publint` and `attw --pack` in `cargo xtask check ts`.
- TS example suite in `packages/pdf/examples/` (hello, invoice, multilingual, images-and-vectors, links-and-outlines, generate-then-edit) run as Vitest tests with byte goldens shared with Rust.
- TS half of the codes test (ADR 0003 §4) and of `fixtures/ops` tests (ADR 0018 §5).

**Out of scope** (and which todo covers it)

- Browser/Deno/Bun/workerd CI matrix: [B6c](./B6c-runtime-matrix.md).
- TypeDoc site: [B7a](./B7a-docs-site.md).
- `page.text()`: [B9c](./B9c-text-extraction-bindings-and-differential.md).
- Publishing to npm: [P1](./P1-reserve-names-and-trusted-publishing.md), [P2](./P2-first-registry-prerelease.md).

## Tasks

- [ ] Read the TS guideline and ADR 0016
- [ ] Create `packages/pdf` (`package.json`, `tsconfig.json` extending base, `src/`, `test/`, `examples/`)
- [ ] `src/init.ts` (input forms, environment detection only via export conditions, no `process`/`fs`)
- [ ] `src/document.ts`, `builder.ts`, `font.ts`, `errors.ts`, `transaction.ts`, `types.ts` (tsify re-exports)
- [ ] Code generator for error codes (`cargo xtask gen codes` → `src/codes.generated.ts`)
- [ ] Examples as tests comparing bytes with `fixtures/golden/examples/*.pdf`
- [ ] publint and `attw --pack` wired into `cargo xtask check ts`; JS glue + API min+gz measured by `cargo xtask size` against the ADR 0016 budget (≤ 20 KB per plan)
- [ ] `packages/pdf/README.md` quick start

## Acceptance criteria

- [ ] All TS examples produce bytes identical to the Rust goldens (Node, in the fast lane)
- [ ] `PdfError.code` values equal `fixtures/codes.json`; a wrong password surfaces as `wrong-password`
- [ ] `fixtures/ops/v1` samples apply through `doc.apply` and unknown ops/fields are rejected
- [ ] publint and `attw --pack` report no problems
- [ ] JS glue + TS API ≤ the ADR 0016 budget (min+gz), enforced in CI
- [ ] No Node-only API in `src/` (Biome rule or a grep check in xtask)

## Post-merge checks

None.

## Tests

- Vitest unit tests in `packages/pdf/test/` (Node environment); example goldens; codes and ops shared-fixture tests.

## Docs to update

- [ ] [pdf-library §10 B6](../../plans/pdf-library.md#10-to-dos): tick `packages/pdf`, export conditions + publint/attw
- [ ] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "`lamina-wasm` … and `@lamina/pdf`"
- [ ] `CLAUDE.md` (layout: `packages/pdf`), package README

## Risks and open questions

- Memory management across the boundary: explicit `free()` vs `FinalizationRegistry`; document both, test that handles are freed in a loop of 1,000 builds.
- Lazy text-module loading makes `text()` async on first use unless preloaded; ADR 0016 decides (recommendation: `init({ text: true })` preloads, otherwise `text()` throws `text-module-not-loaded`).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
