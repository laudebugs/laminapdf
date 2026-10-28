# B3c — `SavePlanner`, `SaveConstraint` detection and round-trip in lamina, pdf.js, PDFium and qpdf

| Field | Value |
| --- | --- |
| ID | `B3c` |
| Status | `ready` |
| Plan | [pdf-library §7 B3](../../plans/pdf-library.md#b3-ops-and-save), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B3b](./B3b-lamina-save-writers.md), [X4](./X4-full-lane-gate.md) |
| Blocks | [B5a](./B5a-lamina-build-core.md), [C1b](./C1b-pdfium-wasm-spike.md), [E1a](./E1a-editing-foundation-feature-plan.md), [E1c](./E1c-metadata-ops.md), [X2](./X2-capability-matrix.md) |
| ADRs | Implements [0007](../../decisions/0007-save-planner.md) §1–§4 (planner, constraint, mode table, override); [0018](../../decisions/0018-test-strategy.md) tier 7 (round-trip) and fuzz target `ops_json`/`save_roundtrip`; [0006](../../decisions/0006-op-log-and-undo.md) §1 (classification) |
| Branch | `feat/b3c-save-planner` |
| PR title | `feat(save): SavePlanner and SaveConstraint with signature detection; round-trip suite` |
| Size | L (≈ 900–1,300 lines incl. the round-trip harness and signed fixtures) |
| PR | — |

## Goal

Make unsafe saves impossible by default. `SavePlanner` inspects the base and the overlay, reports every condition that restricts the save (signatures, DocMDP, rewrite reasons, violations), resolves `SaveMode::Auto`, and refuses a full rewrite of a signed file unless explicitly overridden. `Document::save(sink, SaveOptions)` becomes the one public entry point. A round-trip harness proves that open → op → save → reopen works in lamina and in pdf.js, PDFium and `qpdf --check`, and that undo restores byte-identical output.

## Context

- [ADR 0007 §1–§4](../../decisions/0007-save-planner.md#1-api-lamina-save): `plan()`, `constraint()`, `mode()`, `would_invalidate()`; `SaveConstraint { signatures, doc_mdp, rewrite_reasons, violations }`; `kind()` (`Free`, `AppendOnly`, `RequiresRewrite`, `Conflict`); mode table; `SaveError::WouldInvalidateSignatures`, `RequiresRewrite`; `invalidate_signatures` override removing `/V` and `/Perms` entries; `would_violate(&op)`; canonical order.
- [Research 04 §8](../../research/pdf/04-interactive-security.md#8-digital-signatures-128): signature dictionaries, `/ByteRange`/`/Contents`, DocMDP `/P` 1–3, FieldMDP, `/Lock`, UR3, DocTimeStamp.
- [ADR 0018 §1](../../decisions/0018-test-strategy.md#1-tiers) tier 7; [§2](../../decisions/0018-test-strategy.md#2-ci-lanes) full lane "round-trip with qpdf".
- `lamina-save` from [B3b](./B3b-lamina-save-writers.md), `lamina-ops` from [B3a](./B3a-overlay-and-ops.md).

## Scope

**In scope**

- `SavePlanner`, `SavePlan`, `SaveConstraint`, `ConstraintKind`, `SignatureInfo { field: FieldId?, kind: Approval | Certification | DocTimeStamp | UR3, revision }`, `DocMdp`, `RewriteReason` (`RepairedBase`, `EncryptedBase`, `OpRequiresRewrite`), `Violation`.
- Detection from base + overlay: `/AcroForm /Fields` walk for `/FT /Sig` with `/V` dict; `/Type /DocTimeStamp`; `/Perms /DocMDP` and `/UR3`; `/Reference` transform params (`/P`); FieldMDP `/Lock`; mapping each pending transaction's `ModificationClass` to violations (`SetInfo` = `Metadata`, `RotatePage` = `PageStructure`).
- Mode resolution exactly per the ADR table; errors with the whole constraint; override path that strips invalidated `/V` and matching `/Perms` and lists them in `SaveReport`.
- `Document::save(sink, SaveOptions)` and `Document::plan(&SaveOptions)`; `SaveOptions { mode, invalidate_signatures, output, renumber, xref }` with private fields + builders.
- Signed fixtures: approval-signed, certified with DocMDP P1/P2/P3, DocTimeStamp, generated reproducibly (pyHanko, MIT) with a script; provenance in `fixtures/README.md`.
- Round-trip harness: `cargo xtask roundtrip` — for each corpus file × proving op × mode: lamina save → reopen in lamina (facts), `qpdf --check`, pdf.js (`pdfjs-dist` Node script: page count, rotation, Info) and PDFium (`pypdfium2`: page count, rotation); a manifest of tool versions; runs in the full lane added by [X4](./X4-full-lane-gate.md).
- Fuzz targets `ops_json` (deserialize + apply arbitrary op logs on a fixture) and `save_roundtrip` (open → random ops → save → reopen, no panic, reopen succeeds).

**Out of scope** (and which todo covers it)

- Executing signatures (two-phase signing), validating signature cryptography: full-editor E10 (ADR 0020).
- Redaction and other rewrite-requiring ops: E12.
- Capability-matrix rows: [X2](./X2-capability-matrix.md).
- Nightly round-trip over the external corpus: [X3a](./X3a-nightly-lane.md).

## Tasks

- [ ] Load `/rust-best-practices`; read ADR 0007 §1–4 and 04 §8
- [ ] `crates/lamina-save/src/planner.rs`, `constraint.rs`, `detect.rs` (signatures, DocMDP, FieldMDP, UR3), `resolve.rs` (mode table)
- [ ] `Document::save`/`plan` in the façade order (lamina-save depends on lamina-ops/model); remove B3b's temporary `save_incremental`/`save_full` from the public API
- [ ] Override: strip `/V` of invalidated signature fields and matching `/Perms` keys via overlay changes made by the planner (not by ops), reported in `SaveReport::invalidated`
- [ ] `would_violate(&op)` helper for the editor
- [ ] `fixtures/signed/` + `fixtures/signed/make.py` (pyHanko pinned), README provenance
- [ ] `tools/roundtrip/` (Node script for pdf.js, Python for pypdfium2; pinned in `package.json`/`requirements.txt`) and `cargo xtask roundtrip [--tools pdfjs,pdfium,qpdf]`
- [ ] Add the round-trip job to the full lane workflow from X4
- [ ] `fuzz/fuzz_targets/{ops_json,save_roundtrip}.rs` + seeds from `fixtures/ops/v1`
- [ ] Undo byte-identity test: open → op → save A; op → undo → save B; B == A (incremental: empty overlay → base unchanged)

## Acceptance criteria

- [ ] Every row of ADR 0007 §3's table has a test (4 kinds × 3 requested modes, with and without override)
- [ ] Signed fixtures report `AppendOnly` (approval) or `AppendOnly` + `doc_mdp` (certified); a requested full rewrite fails with `WouldInvalidateSignatures` naming every signature; `Auto` chooses incremental
- [ ] DocMDP P1 + `SetInfo` → violation reported; P2 + `RotatePage` → violation; P3 + annotation-class op → allowed (unit-level with a fake op class)
- [ ] Override strips `/V` and `/Perms`, and the reopened file has unsigned fields and no `/Perms` entry for them
- [ ] Round-trip: open → `SetInfo`/`RotatePage` → save (incremental and full) → reopen is clean in lamina, `qpdf --check`, pdf.js and PDFium for every file in `fixtures/corpus` that opened without repair (repaired bases: full only)
- [ ] Undo restores byte-identical output (test above)
- [ ] `ops_json` and `save_roundtrip` fuzz 10 min without a panic (scheduled run linked in the PR)

## Tests

- `crates/lamina-save/tests/{planner,constraint,override,undo_bytes}.rs`.
- Round-trip suite via `cargo xtask roundtrip` (full lane; needs Node, Python, qpdf).
- Fuzz targets as above; regressions to `fixtures/regressions/`.

## Docs to update

- [ ] [pdf-library §10 B3](../../plans/pdf-library.md#10-to-dos): tick `SaveConstraint` detection, proving ops, round-trip
- [ ] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick `lamina-save`
- [ ] `CLAUDE.md` (`cargo xtask roundtrip`, tool needs), `fixtures/README.md`, ADR 0007 status note if any detail changed

## Risks and open questions

- pyHanko output and the exact DocMDP shapes in the wild vary; add at least one signed file from a permissively licensed external corpus if available.
- pdf.js/PDFium versions drift; pin and record them in the manifest so a failure is reproducible.
- The PR lane must not need these tools: the round-trip job is full-lane only, the fast lane runs the lamina-only half.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
