# B6a — `lamina-wasm`, `@lamina/wasm` build, ADR 0016 and size budgets

| Field | Value |
| --- | --- |
| ID | `B6a` |
| Status | `done` |
| Plan | [pdf-library §7 B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf), non-functional targets [§4](../../plans/pdf-library.md#non-functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B5c](./B5c-lamina-facade-and-examples.md) |
| Blocks | [B6b](../ready/B6b-lamina-pdf-package.md) |
| ADRs | Writes **0016** (package design: ESM-only, export conditions, split WASM modules, size budgets); implements [0002](../../decisions/0002-binding-strategy.md) (wasm-bindgen + tsify, committed `@lamina/wasm` manifest, gitignored `dist/`, pinned binaryen with rustc's feature set) |
| Branch | `feat/b6a-lamina-wasm` |
| PR title | `feat(wasm): lamina-wasm bindings, split base/text modules, ADR 0016 and size budgets` |
| Size | L (≈ 900–1,300 lines incl. ADR) |
| PR | — |

## Goal

Ship the WASM build of the core with sizes we can defend. The `krilla-fit` spike measured **377 KB gzip for the builder stack before any shaper**, already over the plan's 350 KB budget for `@lamina/pdf` *including* shaping and subsetting. This todo builds `lamina-wasm`, splits it into a base module (COS, model, ops, save, build without text) and a lazily loaded text module (skrifa, subsetter, HarfRust, ICU4X data, standard-14 tables), measures both with `wasm-opt -Oz`, decides the budgets in ADR 0016 from those numbers, and makes CI fail above them.

## Context

- [krilla-fit WASM table](../../../spikes/krilla-fit/README.md#measurements): 59.9 KB gz lamina-cos only; 377 KB gz builder + pdf-writer + skrifa + subsetter (no wasm-opt; includes `anyhow` and `png`); krilla 830 KB.
- [ADR 0005 §6 point 4](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges): B6 must decide the split, re-measure with wasm-opt, revise the budget if needed.
- [ADR 0002](../../decisions/0002-binding-strategy.md): wasm-bindgen + tsify, owned handles across the boundary, `@lamina/wasm` layout.
- [Repo and packaging notes §3–4](../../research/architecture/notes/repo-and-packaging.md#3-rustwasmts-rustnode-rustmobile-ts-type-generation); [report: repository](../../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus).
- [pdf-library §5](../../plans/pdf-library.md#5-design-summary): split WASM modules "decided by measurement in B6"; [§4 non-functional](../../plans/pdf-library.md#non-functional) budgets.

## Scope

**In scope**

- `crates/lamina-wasm` (edge layer: wasm-bindgen glue; may use `unsafe` only via generated code): owned handles (`WasmDocument`, `WasmBuilder`, `WasmPage`, `WasmFont`), JSON ops via serde + tsify types, errors as `{ code, message }`, warnings list, `init` entry points; two cdylib targets or one crate with features producing `lamina_base.wasm` and `lamina_text.wasm` (decide: two crates `lamina-wasm` + `lamina-wasm-text`, or one crate built twice; the text module is linked at runtime through a narrow ABI — e.g. the base module calls into JS which calls the text module, passing bytes).
- `packages/wasm` (`@lamina/wasm`): committed `package.json` (private for now), gitignored `dist/` filled by `cargo xtask wasm` (wasm-bindgen CLI pinned, `wasm-opt -Oz` with rustc's enabled features, strip), generated `.d.ts`.
- Size measurement: `cargo xtask size` reporting raw/gzip/brotli for each module and the JS glue (min+gz via esbuild), writing `target/size-report.json`; CI fails above the budgets recorded in ADR 0016.
- Shrinking pass: no `anyhow` in core, `png` decoding only in the text/image path or replaced, `panic = "abort"`, `opt-level = "z"` vs `"s"` comparison, `codegen-units = 1`, ICU4X data sliced, `lto = "fat"`; record each step's effect.
- ADR 0016: ESM-only, export conditions (`browser`, `workerd`, `node`, `deno`, `default`), `init()` input forms (URL, `Response`, bytes, `WebAssembly.Module` for Workers), synchronous API after `init()`, split modules and when the text module loads, budgets per module (measured numbers, chosen limits, headroom), API naming rules for TS (camelCase, `Uint8Array` in/out).

**Out of scope** (and which todo covers it)

- The TS API package `@lamina/pdf`, publint/attw: [B6b](../ready/B6b-lamina-pdf-package.md).
- Runtime matrix in CI: [B6c](../ready/B6c-runtime-matrix.md).
- Edge `ByteSource`s for the web (Blob, OPFS, HTTP range): web C3.
- Text extraction bindings: [B9c](../ready/B9c-text-extraction-bindings-and-differential.md).

## Tasks

- [x] Load `/rust-best-practices`; read ADR 0002, the krilla-fit README, repo/packaging notes §3–4
- [x] Create `crates/lamina-wasm` (edge) and the text module crate/feature; `tsify` feature enabled on `lamina-ops` types (one crate built twice: feature `text`; `lamina-build` gained a `text` feature and the façade a `build` feature so the base module links no `lamina-text`)
- [x] `packages/wasm/package.json` (committed), `.gitignore` for `dist/` (the root `.gitignore`'s `dist/` already covers it)
- [x] Extend `cargo xtask wasm` (wasm-bindgen + wasm-opt pinned in `doctor.rs`), add `cargo xtask size` (binaryen 133 fetched into `target/tools/` with a pinned SHA-256 when not on `PATH`; twiggy 0.8.0 pinned for the growth diff)
- [x] Shrinking pass with a table of before/after sizes in the PR description (and in ADR 0016 §5: `lamina-build` `text` feature, JSON records, `opt-level = "z"`, optional SASLprep, flat drawing ABI, own PNG decoder; `wasm-opt -O1` because `-Oz` gzips 6–8 % larger)
- [x] Write `docs/decisions/0016-package-design.md` with the measured numbers; add to the decisions index and architecture §3
- [x] CI: size job in the fast lane (fails above budget), uploading `size-report.json` (the `wasm` job of `ci.yml`; `cargo xtask wasm` moved there from the `rust` job)
- [x] Smoke test in Node (`node --test` or Vitest) loading both modules and building `hello` (Vitest)

## Acceptance criteria

- [x] ADR 0016 states a budget **per module** (decided by the project owner, 2026-10-29: 350 KB gz for the base module) (base, text, and placeholders for extract, render and sign), not one budget for `@lamina/pdf` including shaping; CI posts a twiggy top-items diff when a module grows by more than 5 % (to the job summary, against the baseline in `size-budgets.json`; deviation: the base budget is 380 KB, revised by the owner on 2026-10-30 after the 368 KB measurement, ADR 0016 §5) ([shareable-libraries §8](../../plans/shareable-libraries.md#8-changes-to-existing-todos), [report: the 350 KB budget](../../research/libraries/shareable-libraries.md#the-350-kb-budget-fits-parse-edit-and-save-only))
- [x] `cargo xtask wasm` produces `lamina_base.wasm`, `lamina_text.wasm` and glue in `packages/wasm/dist/` reproducibly (same bytes twice)
- [x] Sizes are measured with wasm-opt and recorded in ADR 0016; CI fails if either module or the JS glue exceeds its budget (prove with a deliberately bloated build in a test branch or a unit test of the checker) (unit tests of the checker in `tools/xtask/src/wasm.rs`)
- [x] The base module alone can open, read, transact and save a document and build vector/image pages (no text)
- [x] Loading the text module enables `text()`; the `hello` example built through WASM is byte-identical to the native golden (`text` and `textBox` on the page builder; text *extraction* is B9c's)
- [x] ADR 0016 is Accepted (or Proposed with named open points approved by the owner)

## Post-merge checks

None.

## Tests

- `crates/lamina-wasm/tests/` with `wasm-bindgen-test` (Node runner) for the boundary types.
- `packages/wasm/test/smoke.test.ts` (Vitest, Node) loading the built modules.
- Byte-identity of `hello` vs `fixtures/golden/examples/hello.pdf`.

## Docs to update

- [x] [pdf-library §10 B6](../../plans/pdf-library.md#10-to-dos): tick `lamina-wasm` + `wasm-opt -Oz`, ADR 0016, size-budget checks; update §4 non-functional table with the ADR's budgets
- [x] [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs) (0016 → link, status), [§7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) (ADR 0016 half of "Write ADRs 0016, 0017"; `lamina-wasm` half)
- [x] `docs/decisions/README.md`, `CLAUDE.md` (layout: `crates/lamina-wasm`, `packages/wasm`; commands `wasm`, `size`)

## Risks and open questions

- The text module may still exceed any reasonable budget with ICU4X + HarfRust; options: smaller line-break crate, data slicing per script, or a third module for CJK — the ADR records the choice and the owner approves a revised budget.
- Cross-module linking via JS adds call overhead for text; measure `hello` and a 100-page report against the §4 targets.
- `tsify` output for adjacently tagged enums must match the JSON wire format exactly; test with `fixtures/ops/v1`.

## Outcome

- Deviations: the base budget is 380 KB gzip, not 350 KB (owner, 2026-10-30, option A of ADR 0016 §5); the text module is a superset of base (documents move between modules as bytes), not a run-time add-on; records cross as JSON through `serde_json`, not serde-wasm-bindgen (measured, ADR 0016 §7); `wasm-opt -O1`, not `-Oz`. The cross-module call overhead and the 100-page report against the §4 targets are not measured here: with no cross-module calls (superset), the first is moot; the second goes to B6b's TS example suite.
- Measured by `cargo xtask size`: base 368,180 B gzip, text 937,259 B; glue 3.6 KB and 3.8 KB min+gzip (ADR 0016 §6). `cargo xtask wasm` twice gives identical `dist/` bytes.
- Cfg-only change in `crates/lamina/src/data.rs` (three attributes: `Build` under `build`, its `Text` arm under `text`), agreed with the B9b session.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
