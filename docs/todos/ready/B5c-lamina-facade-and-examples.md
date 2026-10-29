# B5c — The `lamina` façade crate, Rust examples, and byte and render goldens

| Field | Value |
| --- | --- |
| ID | `B5c` |
| Status | `ready` |
| Plan | [pdf-library §7 B5](../../plans/pdf-library.md#b5-builder-and-the-lamina-crate), [§6 docs and examples](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B5b](./B5b-builder-text-and-document-features.md), [X4](./X4-full-lane-gate.md) |
| Blocks | [B6a](./B6a-lamina-wasm-and-adr-0016.md), [B8a](./B8a-structure-tree.md), [B9b](./B9b-text-extraction-core.md), [L4a](./L4a-lamina-cli.md), [L6a](./L6a-lamina-ffi-spike.md), [E1a](./E1a-editing-foundation-feature-plan.md), [E1b](./E1b-xmp-packet-model.md), [E2e](./E2e-content-append-api.md), [E6b](./E6b-place-images-and-shapes.md) |
| ADRs | Follows [0005 §1](../../decisions/0005-document-model.md#1-layers) (façade on top), [0018](../../decisions/0018-test-strategy.md) tiers 3 and 8, [0001](../../decisions/0001-core-language-and-licence.md); code-organisation [§4 public API](../../guidelines/code-organization.md#4-public-api-surface-and-stability) |
| Branch | `feat/b5c-lamina-facade` |
| PR title | `feat(lamina): façade crate with examples, byte goldens and PDFium/hayro render goldens` |
| Size | L (≈ 700–1,200 lines: re-exports, docs, six examples, render-golden tooling) |
| PR | — |

## Goal

Give Rust users one crate, `lamina`, whose docs open with the invoice example and whose API covers open → read → transact → save and build → finish → save, re-exporting exactly the public surface of the core crates. Back every example with a byte-identical golden and with render goldens in PDFium and hayro, so visual regressions are caught before the WASM and TS layers are built on top.

## Context

- [pdf-library §5 Rust sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final) and [§6](../../plans/pdf-library.md#6-docs-examples-and-publishing) (minimum example set).
- [ADR 0018 §1](../../decisions/0018-test-strategy.md#1-tiers) tier 8 (render goldens, perceptual thresholds, `cargo xtask render-golden --update`).
- [Rendering notes §1](../../research/architecture/notes/rendering.md#1-candidate-engines-version-maintenance-licence-fidelity-size-features) (PDFium, hayro).
- [Rust guideline: documentation](../../guidelines/rust.md#documentation) (`missing_docs` denied, doc examples).

## Scope

**In scope**

- `crates/lamina` (core layer): re-exports `Document`, `OpenOptions`, `Limits`, `Warning`, views, `Op`/`transact`/undo, `SaveOptions`/`SaveReport`, `Builder` and surface types, `Font`; one `lamina::Error` wrapping the crate errors with `code()`; feature flags `text` (default on; gates `lamina-text`) so a structure-only build exists for WASM splitting.
- `lib.rs` crate docs opening with the invoice example (compiled as a doc test).
- Examples in `crates/lamina/examples/`: `hello`, `invoice`, `multilingual` (Latin + Arabic + Devanagari + Hebrew + CJK), `vectors` (paths, colour, opacity, images), `links_and_outlines`, `generate_then_edit` (build → save → open → `SetInfo` → incremental save). Fonts from `fixtures/fonts/` (OFL), passed as bytes (examples may read files; the library never does).
- `cargo xtask examples` runs every example into `target/examples-out/` and compares with `fixtures/golden/examples/*.pdf`; `cargo xtask golden --update` regenerates them.
- Render goldens: `cargo xtask render-golden [--update]` renders each example page with PDFium (`pypdfium2`, pinned) and hayro (Rust, as an xtask/tool dependency — not a core dependency) at 72 and 150 dpi into `fixtures/render-golden/<example>/<page>-<engine>.png`, compares with a perceptual threshold (e.g. per-pixel ΔE / SSIM ≥ 0.99, configured per file), reports diffs as images; job added to the full lane.
- `qpdf --check` over every example output (full lane).

**Out of scope** (and which todo covers it)

- TS examples and runtime matrix: [B6b](./B6b-lamina-pdf-package.md), [B6c](./B6c-runtime-matrix.md).
- PDF/UA and PDF/A examples: [B8b](./B8b-conformance-profiles-and-verapdf.md).
- Worker endpoint example: [B6c](./B6c-runtime-matrix.md)/[B7a](./B7a-docs-site.md).
- `lamina-render` / `RenderBackend`: web C2.
- Nightly hayro-vs-PDFium diffs over external corpora: [X3b](./X3b-render-diff-nightly.md).

## Tasks

- [ ] Load `/rust-best-practices`; read code-organisation §4 and Rust guideline docs section
- [ ] Create `crates/lamina` with re-exports, `Error`, features, crate docs
- [ ] Write the six examples; keep each under ~80 lines
- [ ] `tools/xtask`: `examples`, `render-golden [--update]`; hayro as an xtask dependency (licence check), pypdfium2 via `tools/render/requirements.txt`
- [ ] Commit byte goldens and render goldens (PNG size budget: keep low DPI for the PR lane)
- [ ] Add jobs to the full lane workflow from [X4](./X4-full-lane-gate.md): examples + qpdf + render goldens
- [ ] `cargo doc` with `-D missing_docs` for `lamina`

## Acceptance criteria

- [ ] The façade meets the binding rules ADR 0028 sets (from [L1a](./L1a-contract-adrs.md)): owned handles instead of borrowed returns, no generics across the boundary, a string-payload error enum, `CancelToken` and resumable `Job::step` for long work, and the thread-safety outcome ADR 0028 picks (lock in the binding edge crate, or a `Sync` core amending ADR 0005 §7); a compile-time test asserts the auto traits that outcome requires ([shareable-libraries §8](../../plans/shareable-libraries.md#8-changes-to-existing-todos))
- [ ] Every example produces a byte-identical golden (twice, and native vs wasm32 for the generation part where feasible)
- [ ] `qpdf --check` is clean for every example output
- [ ] Render goldens match within thresholds in PDFium and hayro; a deliberate one-pixel-wide change in an example makes the job fail with a diff image
- [ ] `lamina` docs build with no missing docs; the crate docs' first example compiles and runs
- [ ] `cargo xtask portability` passes for `lamina` (core) with and without the `text` feature

## Post-merge checks

None.

## Tests

- Doc tests in `crates/lamina`; `crates/lamina/tests/examples.rs` (runs examples, compares goldens).
- Render goldens via xtask in the full lane.

## Docs to update

- [ ] [pdf-library §10 B5](../../plans/pdf-library.md#10-to-dos): tick façade crate, Rust examples, byte-identical and render goldens
- [ ] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "`lamina` façade crate" and the golden half of the harness to-do
- [ ] `CLAUDE.md` (layout: `crates/lamina`; `cargo xtask examples`, `render-golden`), `fixtures/README.md`

## Risks and open questions

- hayro's rendering may differ from PDFium on text anti-aliasing; thresholds per engine, and a known-differences list rather than loosened global thresholds.
- PNG goldens bloat the repository; keep resolution low, consider Git LFS only if it gets large (owner decision).
- ADR 0018 places render goldens in Round C; this todo starts them earlier for the builder (as pdf-library B5 requires); C2 reuses the tooling.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
