# B6b — `@lamina/pdf`: the TypeScript API, `init()`, export conditions, publint and attw

| Field | Value |
| --- | --- |
| ID | `B6b` |
| Status | `done` |
| Plan | [pdf-library §7 B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf), [§5 TS sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2b](./B2b-standard-security-decryption.md), [B6a](./B6a-lamina-wasm-and-adr-0016.md) |
| Blocks | [B5d](../ongoing/B5d-text-report-generation-performance.md), [B6c](./B6c-runtime-matrix.md), [B7a](../ready/B7a-docs-site.md), [B8a](../ready/B8a-structure-tree.md), [B9c](../ready/B9c-text-extraction-bindings-and-differential.md), [L1b](../ready/L1b-error-contract-codegen.md), [L2a](../ready/L2a-worker-client.md), [L2b](../ready/L2b-disposal-and-memory.md), [L2c](./L2c-range-byte-source.md), [C1b](../ready/C1b-pdfium-wasm-spike.md), [C2c](../ready/C2c-render-packages.md), [C3a](../ready/C3a-viewer-worker-rpc.md), [C4c](../ready/C4c-remote-files-encryption-privacy.md), [C6a](../ready/C6a-thumbnails-outline-page-labels.md), [E1c](../ready/E1c-metadata-ops.md), [E1d](../ready/E1d-edit-session-undo-ui.md) |
| ADRs | Implements 0016 (from [B6a](../done/B6a-lamina-wasm-and-adr-0016.md)), [0002](../../decisions/0002-binding-strategy.md), [0006](../../decisions/0006-op-log-and-undo.md) §2 (TS transaction handle), [0003](../../decisions/0003-reader-tolerance-and-limits.md) §4 (codes test in TS), [0018](../../decisions/0018-test-strategy.md) §5 (shared expectations) |
| Branch | `feat/b6b-lamina-pdf` |
| PR title | `feat(pdf): @lamina/pdf TypeScript API over @lamina/wasm with init() and export conditions` |
| Size | L (≈ 900–1,300 lines of TS + tests) |
| PR | [#85](https://github.com/laudebugs/lamina/pull/85) |

## Goal

Give TypeScript users the same library as Rust: `init()`, then a synchronous, typed API (`Builder`, `Document`, `Font`, `LaminaError` with codes matching Rust; the class name is fixed by [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) §7) that runs unchanged in browsers, Node, Deno, Bun and Cloudflare Workers, packaged ESM-only with correct export conditions and verified by publint and `attw --pack`.

## Context

- ADR 0016 from [B6a](../done/B6a-lamina-wasm-and-adr-0016.md); [TypeScript guideline](../../guidelines/typescript.md) (strict, ESM-only, named exports, no Node-only APIs, Biome, Vitest), [§modules and packaging](../../guidelines/typescript.md#modules-and-packaging), [§errors](../../guidelines/typescript.md#errors).
- [ADR 0006 §2](../../decisions/0006-op-log-and-undo.md#2-transactions): `doc.transact(tx => …)` wraps `begin/apply/commit/rollback`; `doc.apply(ops)`.
- [Repo and packaging notes §4](../../research/architecture/notes/repo-and-packaging.md#4-package-design-esm-only-exports-maps-runtime-compatibility-lazy-wasm-bundlers-jsr).
- [Code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer): one `LaminaError` class with string codes ([ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md)).

## Scope

**In scope**

- `packages/pdf` (`@lamina/pdf`, private for now): `init(input?)` (URL, `Response`, `ArrayBuffer`/`Uint8Array`, `WebAssembly.Module`; idempotent; text module loaded on first text use or via `init({ text: true })`), `Builder`, `Page`, `Surface`, `Font.fromBytes`, `Document.open(bytes, { password?, strict?, limits? })`, `doc.pages`, `doc.outline`, `doc.info`, `doc.warnings`, `doc.transact(tx => …)`, `doc.apply(ops)`, `doc.undo()/redo()`, `doc.save({ mode })` → `Uint8Array`, `doc.free()` + `Symbol.dispose` support.
- `LaminaError` with every [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) §1 field (`code`, `kind`, `message`, `hint`, `location`, `data`, `docsUrl`; `didYouMean` from the unknown-key check), built from the JSON `lamina-wasm` returns (B5c's `Serialize`), and `LaminaWarning`. Kinds and hints come from `fixtures/codes.json` v2 ([L1c](../done/L1c-error-contract-core.md)); [L1b](../ready/L1b-error-contract-codegen.md) later generates the unions, the schema and `explain`, without changing the shape; codes generated from `fixtures/codes.json` into a TS union type.
- Op types re-exported from tsify output; JSON samples in `fixtures/ops/v1` round-trip through TS.
- `package.json` `exports` with conditions `browser`, `workerd`, `node`, `deno`, `default`; `.wasm` shipped as an asset referenced by `new URL(..., import.meta.url)`; Workers path imports a `WebAssembly.Module`.
- Build with `tsc` (or tsup/tsdown — pick per guideline) to `dist/`; `publint` and `attw --pack` in `cargo xtask check ts`.
- TS example suite in `packages/pdf/examples/` (hello, invoice, multilingual, images-and-vectors, links-and-outlines, generate-then-edit) run as Vitest tests with byte goldens shared with Rust.
- TS half of the codes test (ADR 0003 §4) and of `fixtures/ops` tests (ADR 0018 §5).

**Out of scope** (and which todo covers it)

- Browser/Deno/Bun/workerd CI matrix: [B6c](./B6c-runtime-matrix.md).
- TypeDoc site: [B7a](../ready/B7a-docs-site.md).
- `page.text()`: [B9c](../ready/B9c-text-extraction-bindings-and-differential.md).
- Publishing to npm: [P1](../ready/P1-reserve-names-and-trusted-publishing.md), [P2](../ready/P2-first-registry-prerelease.md).

## Tasks

- [x] Read the TS guideline and ADR 0016
- [x] Create `packages/pdf` (`package.json`, `tsconfig.json` extending base, `src/`, `test/`, `examples/`)
- [x] `src/init.ts` (input forms, environment detection only via export conditions, no `process`/`fs`)
- [x] `src/document.ts`, `builder.ts`, `font.ts`, `errors.ts`, `transaction.ts`, `types.ts` (tsify re-exports)
- [x] Code generator for error codes (`cargo xtask gen codes` → `src/codes.generated.ts`)
- [x] Examples as tests comparing bytes with `fixtures/golden/examples/*.pdf`
- [x] publint and `attw --pack` wired into `cargo xtask wasm` (deviation: not `check ts`, they need the generated dist/; see Outcome); JS glue + API min+gz measured by `cargo xtask size` against the ADR 0016 budget (≤ 20 KB per plan)
- [x] `packages/pdf/README.md` quick start

## Acceptance criteria

- [x] Each export condition has its own wasm loader (workerd: static `.wasm` import; Node: read the file next to the module; browser: `compileStreaming` on a `new URL(…, import.meta.url)`), with `init({ module })` as the escape hatch, tested in Vite with the package excluded from dependency pre-bundling ([shareable-libraries §8](../../plans/shareable-libraries.md#8-changes-to-existing-todos), [report: one `.wasm` file per export condition](../../research/libraries/shareable-libraries.md#one-wasm-file-resolved-per-export-condition-with-an-escape-hatch))
- [x] All TS examples produce bytes identical to the Rust goldens (Node, in the fast lane)
- [x] `LaminaError.code` values equal `fixtures/codes.json`; a wrong password surfaces as `wrong-password`
- [x] `fixtures/ops/v1` samples apply through `doc.apply` and unknown ops/fields are rejected
- [x] publint and `attw --pack` report no problems
- [x] JS glue + TS API ≤ the ADR 0016 budget (min+gz), enforced in CI
- [x] No Node-only API in `src/` (Biome rule or a grep check in xtask)

## Post-merge checks

None.

## Tests

- Vitest unit tests in `packages/pdf/test/` (Node environment); example goldens; codes and ops shared-fixture tests.

## Docs to update

- [x] [pdf-library §10 B6](../../plans/pdf-library.md#10-to-dos): tick `packages/pdf`, export conditions + publint/attw
- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "`lamina-wasm` … and `@lamina/pdf`"
- [x] `CLAUDE.md` (layout: `packages/pdf`), package README

## Risks and open questions

- Memory management across the boundary: explicit `free()` vs `FinalizationRegistry`; document both, test that handles are freed in a loop of 1,000 builds.
- Lazy text-module loading makes `text()` async on first use unless preloaded; ADR 0016 decides (recommendation: `init({ text: true })` preloads, otherwise `text()` throws `text-module-not-loaded`).

## Outcome

- Delivered: `packages/pdf` (`@lamina/pdf`, private): `init(input?)` (URL or string, `Response` or a promise of one, `ArrayBuffer`/`Uint8Array`, `WebAssembly.Module`; `{ module, text }`; idempotent, retryable), `loadText()`, `isInitialized()`, `isTextLoaded()`, `wasmMemory()`; `Document` (`open` with the password as given, then NFKC after `wrong-password`, `strict`, `limits`; `pageCount`, `pages`, `pageInfo`, `page`/`pageById` → `PageRef`, `info`, `warnings`, `apply`, `applyLog`, `transact`, `undo`, `redo`, `save`, `saveWithReport`, `saveJob` → `SaveJob`, `rebase`, `free`, `Symbol.dispose`); `Builder` (`page`, `font`, `destination`, `outline`, `pageLabels`, `metadata`, `lang`, `viewerPreferences`, `finish`, `free`), `Page` (boxes, rotation, `finish`), `Surface` (state, colours, opacity, blend, `transform` with `Matrix`, `path` → `PendingPath`, `image`, `link`, `text`, `textBox`), `Font.fromBytes`/`Font.standard`, `Image.jpeg`/`png`/`rgba`; `LaminaError` (every ADR 0026 §1 field, `toJSON`), `isLaminaError`, `LaminaWarning`; the tsify records re-exported (`Op`, `SaveOptions`, `LinkTarget`, …).
- Export conditions `workerd` (static `.wasm` imports), `deno`/`browser`/`default` (the glue's `new URL(…, import.meta.url)`, streaming compile), `node` (the file read as bytes); a Vite test (dev server with the package excluded from pre-bundling, and a production build whose bundle runs) covers the browser condition.
- Codes: `cargo xtask gen codes [--check]` writes `src/codes.generated.ts` (`ErrorCode`, `WarningCode`, `LimitName`, the code lists, and kind + hint of the codes the TS layer raises); `check ts` runs `--check`. New `packageErrors` group in `fixtures/codes.json` with pages: `already-initialized`, `not-initialized`, `text-module-not-loaded` (all `state`), left out of the façade's hint tables (TS-only).
- Tests (Vitest, Node, against the built `dist/`): 65, in `test/` (examples byte-identical to the six Rust goldens; codes; ops samples; init forms, the browser entry over HTTP; Vite; builder errors and 1,000 freed builds at constant WASM memory; packaging: same exports per entry, no Node API in `src/` outside the Node loader).
- Sizes (`cargo xtask size`): base 372,711 B gzip and text 943,924 B (unchanged: the TS-only `packageErrors` group is left out of the façade's hint tables); `@lamina/pdf` JS (both glues + API, esbuild min+gzip) 10,041 B against its 20,000 B budget, now in `size-budgets.json` (`packages`).
- Deviations:
  - `doc.outline` is not bound: the façade has no outline read yet, and adding it (façade + binding) is Rust work measured against the base module's 2 % headroom; left to [C6a](../ready/C6a-thumbnails-outline-page-labels.md), which needs it.
  - `transact` collects the ops and applies them as one transaction when the callback returns (the binding has no `begin`/`commit`): atomic, but reads inside the callback see the document before it. Inside a callback, any edit or save of the same document (a nested `transact`, `apply`, `undo`, `save`, …) throws `nested-transaction`; an async callback is refused (`invalid-argument`, nothing applied); a handle used afterwards is `no-transaction`.
  - publint and `attw --pack` run in `cargo xtask wasm` (the fast lane's `wasm` job), not `check ts`: they and the package's typecheck and tests need the generated `@lamina/wasm` `dist/`. `check ts` runs Biome (with `noNodejsModules` on `src/` and `examples/`) and `gen codes --check`.
  - The `.wasm` files are not copied into `@lamina/pdf`: the browser loader lets each wasm-bindgen glue fetch the file next to itself (its own `new URL(…, import.meta.url)`), so bundlers emit each module once; whether `@lamina/wasm` is published or bundled stays with P1/P2 (ADR 0016 §2).
  - The entries re-export `src/api.ts` with `export *` (that module lists every public name), so the three entries can't drift; a test compares their exports.
  - A use-after-`free` still fails in the glue, not with a coded error: `disposed` is [L2b](../ready/L2b-disposal-and-memory.md)'s. Finished pages and builders throw `builder-finished` from TS (their WASM side is freed at `finish`).
  - Plan §4's timing targets (left here by B6a), measured once in Node 24 on the M-series Mac (not a test: ADR 0018 keeps wall-clock asserts out): `hello` 2.6 ms after `init()` (target ≤ 5 ms); a 100-page report (a Helvetica-Bold heading and six 4-sentence Noto Sans text boxes per page) 2.96 s (target ≤ 500 ms: **missed**, the text layout path needs profiling: [B5d](../ongoing/B5d-text-report-generation-performance.md)), 85 KB output, text-module memory 5 MB (target < 64 MB).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
