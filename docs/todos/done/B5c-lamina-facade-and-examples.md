# B5c — The `lamina` façade crate, Rust examples, and byte and render goldens

| Field | Value |
| --- | --- |
| ID | `B5c` |
| Status | `done` |
| Plan | [pdf-library §7 B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate), [§6 docs and examples](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B5b](./B5b-builder-text-and-document-features.md), [L1a](./L1a-contract-adrs.md), [L1c](./L1c-error-contract-core.md), [X4](./X4-full-lane-gate.md) |
| Blocks | [B6a](./B6a-lamina-wasm-and-adr-0016.md), [B8a](./B8a-structure-tree.md), [B9b](./B9b-text-extraction-core.md), [L4a](./L4a-lamina-cli.md), [L6a](./L6a-lamina-ffi-spike.md), [E1a](../ready/E1a-editing-foundation-feature-plan.md), [E1b](../ready/E1b-xmp-packet-model.md), [E2e](../ready/E2e-content-append-api.md), [E6b](../ready/E6b-place-images-and-shapes.md) |
| ADRs | Follows [0005 §1](../../decisions/0005-document-model.md#1-layers) (façade on top), [0018](../../decisions/0018-test-strategy.md) tiers 3 and 8, [0001](../../decisions/0001-core-language-and-licence.md); code-organisation [§4 public API](../../guidelines/code-organization.md#4-public-api-surface-and-stability) |
| Branch | `feat/b5c-lamina-facade` |
| PR title | `feat(lamina): façade crate with examples, byte goldens and PDFium/hayro render goldens (B5c)` |
| Size | L (≈ 700–1,200 lines: re-exports, docs, six examples, render-golden tooling) |
| PR | [#65](https://github.com/laudebugs/lamina/pull/65) |

## Goal

Give Rust users one crate, `lamina`, whose docs open with the invoice example and whose API covers open → read → transact → save and build → finish → save, re-exporting exactly the public surface of the core crates. Back every example with a byte-identical golden and with render goldens in PDFium and hayro, so visual regressions are caught before the WASM and TS layers are built on top.

## Context

- [pdf-library §5 Rust sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final) and [§6](../../plans/pdf-library.md#6-docs-examples-and-publishing) (minimum example set).
- [ADR 0018 §1](../../decisions/0018-test-strategy.md#1-tiers) tier 8 (render goldens, perceptual thresholds, `cargo xtask render-golden --update`).
- [Rendering notes §1](../../research/architecture/notes/rendering.md#1-candidate-engines-version-maintenance-licence-fidelity-size-features) (PDFium, hayro).
- [Rust guideline: documentation](../../guidelines/rust.md#documentation) (`missing_docs` denied, doc examples).

## Scope

**In scope**

- `crates/lamina` (core layer): re-exports `Document`, `OpenOptions`, `Limits`, `Warning`, views, `Op`/`transact`/undo, `SaveOptions`/`SaveReport`, `Builder` and surface types, `Font`; one `lamina::Error` wrapping the crate errors with `code()`, `kind()`, `location()` (from [L1c](../done/L1c-error-contract-core.md)), `hint()`, `docs_url()` and `serde::Serialize` into the [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) §1 shape (document-derived strings only in `data`), and the same for `Warning`; jobs per [ADR 0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) with the codes `cancelled`, `job-finished` and `document-changed` added to `fixtures/codes.json` (kinds per ADR 0026 §2) and their `docs/errors/` pages; feature flags `text` (default on; gates `lamina-text`) so a structure-only build exists for WASM splitting.
- `lib.rs` crate docs opening with the invoice example (compiled as a doc test).
- Examples in `crates/lamina/examples/`: `hello`, `invoice`, `multilingual` (Latin + Arabic + Devanagari + Hebrew + CJK), `vectors` (paths, colour, opacity, images), `links_and_outlines`, `generate_then_edit` (build → save → open → `SetInfo` → incremental save). Fonts from `fixtures/fonts/` (OFL), passed as bytes (examples may read files; the library never does).
- `cargo xtask examples` runs every example into `target/examples-out/` and compares with `fixtures/golden/examples/*.pdf`; `cargo xtask golden --update` regenerates them.
- Render goldens: `cargo xtask render-golden [--update]` renders each example page with PDFium (`pypdfium2`, pinned) and hayro (Rust, as an xtask/tool dependency — not a core dependency) at 72 and 150 dpi into `fixtures/render-golden/<example>/<page>-<engine>.png`, compares with a perceptual threshold (e.g. per-pixel ΔE / SSIM ≥ 0.99, configured per file), reports diffs as images; job added to the full lane.
- `qpdf --check` over every example output (full lane).

**Out of scope** (and which todo covers it)

- TS examples and runtime matrix: [B6b](../done/B6b-lamina-pdf-package.md), [B6c](../done/B6c-runtime-matrix.md).
- PDF/UA and PDF/A examples: [B8b](../done/B8b-conformance-profiles-and-verapdf.md).
- Worker endpoint example: [B6c](../done/B6c-runtime-matrix.md)/[B7a](./B7a-docs-site.md).
- `lamina-render` / `RenderBackend`: web C2.
- Nightly hayro-vs-PDFium diffs over external corpora: [X3b](./X3b-render-diff-nightly.md).

## Tasks

- [x] Load `/rust-best-practices`; read code-organisation §4 and Rust guideline docs section
- [x] Create `crates/lamina` with re-exports, `Error`, features, crate docs
- [x] Write the six examples; keep each under ~80 lines
- [x] `tools/xtask`: `examples`, `render-golden [--update]`; hayro as a tool dependency (licence check), pypdfium2 via `tools/render/requirements.txt` (hayro lives in `tools/render`, its own workspace with its own `Cargo.lock` outside the root one, so the fast lane never compiles it; `render-golden` runs its fmt, clippy and tests, and `cargo xtask deny` checks its lockfile)
- [x] Commit byte goldens and render goldens (PNG size budget: keep low DPI for the PR lane)
- [x] Add jobs to the full lane workflow from [X4](../done/X4-full-lane-gate.md): examples + qpdf + render goldens
- [x] `cargo doc` with `-D missing_docs` for `lamina`

## Acceptance criteria

- [x] The façade meets the binding rules ADR 0028 sets (from [L1a](../done/L1a-contract-adrs.md)): owned handles instead of borrowed returns, no generics across the boundary, a string-payload error enum, `CancelToken` and resumable `Job::step` for long work, and the thread-safety outcome ADR 0028 picks (lock in the binding edge crate, or a `Sync` core amending ADR 0005 §7); a compile-time test asserts the auto traits that outcome requires ([shareable-libraries §8](../../plans/shareable-libraries.md#8-changes-to-existing-todos)). ADR 0028 decided: rules F1–F12 ([§1](../../decisions/0028-facade-rules-jobs-and-cancellation.md#1-façade-rules)); the lock lives in the binding edge, so the test asserts `lamina::Document: Send` and `!Sync`; jobs take the document per `step`, and **every** job, write jobs included, checks a document **revision** counter (plus an instance identifier) that this todo adds, in the core, on every step (`document-changed`, `job-finished`, `cancelled`: added to `fixtures/codes.json` here, with their `docs/errors/` pages); `Budget` and progress counts are `u32` (F5); `Budget` in work units with measured units/ms recorded in the crate README; `lamina::Error` serialises to the [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) shape
- [x] Every example produces a byte-identical golden (twice, and native vs wasm32 for the generation part where feasible). Twice natively, in `crates/lamina/tests/examples.rs`; native vs wasm32 not run (no wasm runtime in the test setup yet; the crate builds for wasm32 in `portability`/`wasm`): left to B6a, which adds `lamina-wasm`
- [x] `qpdf --check` is clean for every example output
- [x] Render goldens match within thresholds in PDFium and hayro; a deliberate one-pixel-wide change in an example makes the job fail with a diff image (checked by hand: a 1 pt hairline added to `hello` failed both engines' goldens, 60 and 122 pixels, with expected | actual | diff images; `tools/render`'s unit tests cover the metric)
- [x] `lamina` docs build with no missing docs; the crate docs' first example compiles and runs
- [x] `cargo xtask portability` passes for `lamina` (core) with and without the `text` feature

## Outcome and deviations

- **`lamina` crate** (`crates/lamina`, layer `core`): `Document` (owned reads, `apply(Vec<Op>)`/`apply_log`, `undo`/`redo`, `save(&SaveOptions, &CancelToken)` → `Saved { bytes, report }`, rebased), `SaveJob` with `Budget`/`Step`/`Phase`/`CancelToken`, a revision counter and instance identity checked at the start of every step, `Error` (crate errors flattened, plus `page-index-out-of-range`, `cancelled`, `job-finished`, `document-changed`), `Warning`, `WarningCursor`, `Diagnostic` (the ADR 0026 §1 record, `Serialize`), `Builder`; re-exports of the core crates' public types. `Document: Send + !Sync` is asserted at compile time (`tests/facade.rs`). Units per millisecond are in `crates/lamina/README.md` (`tests/budget.rs`, ignored, release).
- **Deviation: `text` gates `lamina-build` too.** `lamina-build` depends on `lamina-text`, so the structure-only build has no builder; making the builder's text optional is left to B6a's WASM splitting if it needs a vector-only builder.
- **Deviation: the builder keeps its Rust shape.** `lamina::Builder` wraps `lamina-build`'s and finishes into a `lamina::Document`, but pages are borrowed `PageBuilder<'_>`s and paths are drawn with closures (F1/F3 hold for `Document`, not for the builder). Its binding form (owned page handles, path records) is defined by the edge crates, starting with B6a.
- **Deviation: `SaveJob` is resumable only before the write.** Planning is 1 unit; a full rewrite of an opened document parses and caches the base's objects at 1 unit each; writing and rebasing is one indivisible unit, because `lamina-save`'s writers are not resumable. Measured overshoot: 12–17 ms for a 2,000-page file. Cancellation lands before the write.
- **Render goldens at 72 dpi only** (316 kB for 9 pages × 2 engines), rendered from the committed byte goldens; at 150 dpi hayro and PDFium are compared with each other (the hayro-parity spike's metric), with nothing committed. All pages pass both without known differences. Zero differing pixels relies on pinned engines and one CPU architecture (aarch64: hayro's run-time SIMD level is always NEON there); see `fixtures/render-golden/README.md`.
- **Review (`/code-review high`, 20 findings):** 19 fixed in one commit, one rejected (the extra copy in the save sink: two buffers are the minimum while both the caller and the document own the output as `Vec<u8>`). Additive in `lamina-build`: `Builder::finish_with_warnings`, so the façade surfaces font warnings.
- **Hints and summaries** for `hint()` and the warnings' `message` are generated from `fixtures/codes.json` by the crate's `build.rs` (L1b replaces this with its generator).  `crates/lamina/tests/data_fields.rs` checks every code's `data` keys against its page, so a variant added to a wrapped crate fails until mapped.
- **Capability matrix:** no tags added. The façade tests cover rows (Info, rotation, saving) that the core already proves at `round-trip` in `lamina-save/tests/roundtrip_suite.rs`, the only place write and round-trip claims count.

## Post-merge checks

None.

## Tests

- Doc tests in `crates/lamina`; `crates/lamina/tests/examples.rs` (runs examples, compares goldens).
- Render goldens via xtask in the full lane.

## Docs to update

- [x] [pdf-library §10 B5](../../plans/pdf-library.md#10-to-dos): tick façade crate, Rust examples, byte-identical and render goldens
- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "`lamina` façade crate" and the golden half of the harness to-do
- [x] `CLAUDE.md` (layout: `crates/lamina`; `cargo xtask examples`, `render-golden`), `fixtures/README.md`

## Risks and open questions

- hayro's rendering may differ from PDFium on text anti-aliasing; thresholds per engine, and a known-differences list rather than loosened global thresholds.
- PNG goldens bloat the repository; keep resolution low, consider Git LFS only if it gets large (owner decision).
- ADR 0018 places render goldens in Round C; this todo starts them earlier for the builder (as pdf-library B5 requires); C2 reuses the tooling.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings): left to the owner's review session
- [ ] CI green (`cargo xtask ci` passed locally; the PR's lanes run on the self-hosted runners)
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
