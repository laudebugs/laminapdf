# B3c — `SavePlanner`, `SaveConstraint` detection and round-trip in lamina, pdf.js, PDFium and qpdf

| Field | Value |
| --- | --- |
| ID | `B3c` |
| Status | `done` |
| Plan | [pdf-library §7 B3](../../plans/pdf-library.md#b3-ops-and-save), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2b](./B2b-standard-security-decryption.md), [B3b](./B3b-lamina-save-writers.md), [X4](./X4-full-lane-gate.md) |
| Blocks | [B5a](./B5a-lamina-build-core.md), [C1b](../ready/C1b-pdfium-wasm-spike.md), [E1a](../ready/E1a-editing-foundation-feature-plan.md), [E1c](../ready/E1c-metadata-ops.md), [X2](./X2-capability-matrix.md) |
| ADRs | Implements [0007](../../decisions/0007-save-planner.md) §1–§4 (planner, constraint, mode table, override); [0018](../../decisions/0018-test-strategy.md) tier 7 (round-trip) and fuzz target `ops_json`/`save_roundtrip`; [0006](../../decisions/0006-op-log-and-undo.md) §1 (classification) |
| Branch | `feat/b3c-save-planner` |
| PR title | `feat(save): SavePlanner and SaveConstraint with signature detection; round-trip suite` |
| Size | L (≈ 900–1,300 lines incl. the round-trip harness and signed fixtures) |
| PR | [#50](https://github.com/laudebugs/lamina/pull/50) |

## Goal

Make unsafe saves impossible by default. `SavePlanner` inspects the base and the overlay, reports every condition that restricts the save (signatures, DocMDP, rewrite reasons, violations), resolves `SaveMode::Auto`, and refuses a full rewrite of a signed file unless explicitly overridden. `Document::save(sink, SaveOptions)` becomes the one public entry point. A round-trip harness proves that open → op → save → reopen works in lamina and in pdf.js, PDFium and `qpdf --check`, and that undo restores byte-identical output.

## Context

- [ADR 0007 §1–§4](../../decisions/0007-save-planner.md#1-api-lamina-save): `plan()`, `constraint()`, `mode()`, `would_invalidate()`; `SaveConstraint { signatures, doc_mdp, rewrite_reasons, violations }`; `kind()` (`Free`, `AppendOnly`, `RequiresRewrite`, `Conflict`); mode table; `SaveError::WouldInvalidateSignatures`, `RequiresRewrite`; `invalidate_signatures` override removing `/V` and `/Perms` entries; `would_violate(&op)`; canonical order.
- [Research 04 §8](../../research/pdf/04-interactive-security.md#8-digital-signatures-128): signature dictionaries, `/ByteRange`/`/Contents`, DocMDP `/P` 1–3, FieldMDP, `/Lock`, UR3, DocTimeStamp.
- [ADR 0018 §1](../../decisions/0018-test-strategy.md#1-tiers) tier 7; [§2](../../decisions/0018-test-strategy.md#2-ci-lanes) full lane "round-trip with qpdf".
- `lamina-save` from [B3b](../done/B3b-lamina-save-writers.md), `lamina-ops` from [B3a](../done/B3a-overlay-and-ops.md).

## Scope

**In scope**

- `SavePlanner`, `SavePlan`, `SaveConstraint`, `ConstraintKind`, `SignatureInfo { field: FieldId?, kind: Approval | Certification | DocTimeStamp | UR3, revision }`, `DocMdp`, `RewriteReason` (`RepairedBase`, `EncryptedBase`, `OpRequiresRewrite`), `Violation`.
- Detection from base + overlay: `/AcroForm /Fields` walk for `/FT /Sig` with `/V` dict; `/Type /DocTimeStamp`; `/Perms /DocMDP` and `/UR3`; `/Reference` transform params (`/P`); FieldMDP `/Lock`; mapping each pending transaction's `ModificationClass` to violations (`SetInfo` = `Metadata`, `RotatePage` = `PageStructure`).
- Mode resolution exactly per the ADR table; errors with the whole constraint; override path that strips invalidated `/V` and matching `/Perms` and lists them in `SaveReport`.
- `Document::save(sink, SaveOptions)` and `Document::plan(&SaveOptions)`; `SaveOptions { mode, invalidate_signatures, remove_encryption, output, renumber, xref }` with private fields + builders.
- **Encrypted base** ([ADR 0007 §2](../../decisions/0007-save-planner.md#2-saveconstraint-all-the-conditions-not-one-of-them): "an encrypted base while encryption on write is missing (full-editor E11; today's `EncryptedBase`), which can only be saved by removing encryption"). The planner reports `RewriteReason::EncryptedBase` (kind `RequiresRewrite`, or `Conflict` if also signed), but a save never drops or corrupts encryption silently: with `remove_encryption` unset (the default), `Auto`, `FullRewrite` and `Incremental` all fail with `SaveError::EncryptedBase` (code `encrypted-base`, in `fixtures/codes.json`), before anything is written. With `SaveOptions::remove_encryption(true)` (explicit opt-in; needs a document decrypted by [B2b](../done/B2b-standard-security-decryption.md)) `Auto`/`FullRewrite` write a full rewrite with every string and stream decrypted and no `/Encrypt` in the trailer, and `SaveReport` records `encryption_removed`. Keeping encryption (`keep_encryption`: re-encrypt on rewrite, encrypted incremental saves) is encryption on write, full-editor [E11b](../ready/E11b-encrypt-on-write.md); until then there is no way to keep it and the default refuses. The owner-password authorisation policy for removal is E11a/[E11c](../ready/E11c-remove-security.md)'s (its `RemoveEncryption` op goes through this path).
- Signed fixtures: approval-signed, certified with DocMDP P1/P2/P3, DocTimeStamp, generated reproducibly (pyHanko, MIT) with a script; provenance in `fixtures/README.md`.
- Round-trip harness: `cargo xtask roundtrip` — for each corpus file × proving op × mode: lamina save → reopen in lamina (facts), `qpdf --check`, pdf.js (`pdfjs-dist` Node script: page count, rotation, Info) and PDFium (`pypdfium2`: page count, rotation); a manifest of tool versions; runs in the full lane added by [X4](../done/X4-full-lane-gate.md).
- Nightly round-trip over the external corpus: `cargo xtask roundtrip --corpus external` (the `corpus fetch` inputs, sampled if needed) as a job in `.github/workflows/nightly.yml` ([X3a](../done/X3a-nightly-lane.md) built the lane; this todo adds the job).
- Fuzz targets `ops_json` (deserialize + apply arbitrary op logs on a fixture) and `save_roundtrip` (open → random ops → save → reopen, no panic, reopen succeeds).

**Out of scope** (and which todo covers it)

- Executing signatures (two-phase signing), validating signature cryptography: full-editor E10 (ADR 0020).
- Redaction and other rewrite-requiring ops: E12.
- Capability-matrix rows: [X2](./X2-capability-matrix.md).

## Tasks

- [x] Load `/rust-best-practices`; read ADR 0007 §1–4 and 04 §8
- [x] `crates/lamina-save/src/planner.rs`, `constraint.rs`, `detect.rs` (signatures, DocMDP, FieldMDP, UR3), `resolve.rs` (mode table)
- [x] `Document::save`/`plan` in the façade order (lamina-save depends on lamina-ops/model); remove B3b's temporary `save_incremental`/`save_full` from the public API
- [x] Override: strip `/V` of invalidated signature fields and matching `/Perms` keys via overlay changes made by the planner (not by ops), reported in `SaveReport::invalidated`
- [x] `would_violate(&op)` helper for the editor
- [x] Encrypted base: `RewriteReason::EncryptedBase` detection from the base trailer; `SaveError::EncryptedBase` (+ code) for every mode unless `remove_encryption`; the removal path in `FullWriter` (decrypted objects, `/Encrypt` and the encryption dictionary dropped, `/ID` kept per ADR 0007 §5); `SaveReport::encryption_removed`
- [x] `fixtures/signed/` + `fixtures/signed/make.py` (pyHanko pinned), README provenance
- [x] `tools/roundtrip/` (Node script for pdf.js, Python for pypdfium2; pinned in `package.json`/`requirements.txt`) and `cargo xtask roundtrip [--tools pdfjs,pdfium,qpdf]`
- [x] Add the round-trip job to the full lane workflow from X4
- [x] `fuzz/fuzz_targets/{ops_json,save_roundtrip}.rs` + seeds from `fixtures/ops/v1`
- [x] Undo byte-identity test: open → op → save A; op → undo → save B; B == A (incremental: empty overlay → base unchanged)

## Acceptance criteria

- [x] Every row of ADR 0007 §3's table has a test (4 kinds × 3 requested modes, with and without override)
- [x] Signed fixtures report `AppendOnly` (approval) or `AppendOnly` + `doc_mdp` (certified); a requested full rewrite fails with `WouldInvalidateSignatures` naming every signature; `Auto` chooses incremental
- [x] DocMDP P1 + `SetInfo` → violation reported; P2 + `RotatePage` → violation; P3 + annotation-class op → allowed (unit-level with a fake op class)
- [x] Override strips `/V` and `/Perms`, and the reopened file has unsigned fields and no `/Perms` entry for them
- [x] Round-trip: open → `SetInfo`/`RotatePage` → save (incremental and full) → reopen is clean in lamina, `qpdf --check`, pdf.js and PDFium for every file in `fixtures/corpus` that opened without repair and is neither signed nor encrypted (repaired bases: full only). Signed bases (`signed.pdf`, and any corpus file where the planner finds a signature) instead assert the refusals: `Auto`/`Incremental` succeed and round-trip, requested `FullRewrite` fails with `WouldInvalidateSignatures`. `tampered.pdf` is both signed and a repaired base (its newest catalog is damaged and repairs to the older one), so it is the `Conflict` case: the planner reports the signature (detection finds `/Type /Sig` dictionaries in the file, not only those reachable from the repaired catalog) and `RewriteReason::RepairedBase`; without the override `Auto`, `Incremental` and `FullRewrite` all fail with `WouldInvalidateSignatures` (carrying the rewrite reason); with `invalidate_signatures` `Auto`/`FullRewrite` write a full rewrite with the signature value removed and the report naming it, while `Incremental` still fails (rewrite reasons rule it out) Encrypted bases (`enc.pdf`) assert `SaveError::EncryptedBase` for every mode without `remove_encryption`, and with it a full rewrite that reopens clean (no `/Encrypt`) in all four tools
- [x] Encrypted base, per mode: `Auto`, `FullRewrite`, `Incremental` without `remove_encryption` → `EncryptedBase` and the sink untouched; with `remove_encryption` → `Auto` resolves to `FullRewrite`, output has no `/Encrypt`, page count/Info/content streams equal the decrypted original, `SaveReport::encryption_removed`; encrypted + signed → `Conflict` (needs `invalidate_signatures` too)
- [x] Undo restores byte-identical output (test above)
- [x] `ops_json` and `save_roundtrip` run 10 min each locally (`cargo xtask fuzz <t> --time 600`) without a panic; summary in the PR

## Post-merge checks

- [ ] `ops_json` and `save_roundtrip` are green in the first scheduled fuzz run on `main` (link it in a follow-up PR or the done todo)

## Tests

- `crates/lamina-save/tests/{planner,constraint,override,undo_bytes,encrypted_base}.rs` (`encrypted_base.rs` over `fixtures/corpus/enc.pdf` and `fixtures/encrypted/`: refusal per mode, removal path, signed + encrypted conflict).
- Round-trip suite via `cargo xtask roundtrip` (full lane; needs Node, Python, qpdf).
- Fuzz targets as above; regressions to `fixtures/regressions/`.

## Docs to update

- [x] [pdf-library §10 B3](../../plans/pdf-library.md#10-to-dos): tick `SaveConstraint` detection, proving ops, round-trip
- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick `lamina-save`
- [x] `CLAUDE.md` (`cargo xtask roundtrip`, tool needs), `fixtures/README.md`, `fixtures/codes.json` (`encrypted-base`)
- [x] ADR 0007 implementation note: an encrypted base without encryption on write refuses every mode unless `remove_encryption` (the §3 table's `RequiresRewrite` → `FullRewrite` applies only with it); `keep_encryption` arrives with E11b

## Risks and open questions

- pyHanko output and the exact DocMDP shapes in the wild vary; add at least one signed file from a permissively licensed external corpus if available.
- pdf.js/PDFium versions drift; pin and record them in the manifest so a failure is reproducible.
- `remove_encryption` lets anyone who could decrypt (user or owner password) write a plaintext copy; that is no weaker than what decryption already gives ([04 §7.5](../../research/pdf/04-interactive-security.md#75-user-vs-owner-passwords-why-owner-only-protection-isnt-security)), but E11a decides whether the editor requires the owner password; keep the option off by default and never set it implicitly.
- The PR lane must not need these tools: the round-trip job is full-lane only, the fast lane runs the lamina-only half.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Implementation notes and deviations

Recorded in [ADR 0007 §9](../../decisions/0007-save-planner.md#9-implementation-notes-b3c). Where the work differs from the text above:

- **`Document::save`/`plan` are `Save` trait methods** (`lamina_ops::Document` is defined below `lamina-save`): `plan(&options)`, `save(&mut sink, &options)` and `save_and_rebase(sink, &options)` (a `ReadableSink`, replacing `save_full_and_rebase`). No `doc.write(plan, sink)`: `save` plans again, so a plan can't go stale.
- **`tampered.pdf` is not a repaired base.** Its changed byte is inside the signed range and the file opens without repair, so it is `AppendOnly` like `signed.pdf` (Auto/Incremental round-trip, a requested full rewrite is refused). The `Conflict` case is `tampered.pdf` with its newest catalog damaged as well (byte 590, the mutation `lamina-cos`'s `policy.rs` already uses), built in memory by the tests and the round-trip suite: it repairs to the older, unsigned catalog, the planner finds the signature by scanning the repaired file's objects and reports `RepairedBase`, and the refusals and the override behave as specified.
- **Override mechanics:** the writers write the planner's changes (no `/V` on the invalidated fields, their `/Perms` entries gone; direct fields and a direct `/Perms` edited in their containing object) over the document's values; the document changes only through the rebase onto the output. An overridden incremental save also invalidates every signature that restricts changes.
- **Detection by scanning** runs only for a repaired base (every object loaded), not for every save.
- **An unchanged document saves incrementally** in `Auto` and `Incremental` whatever its rewrite reasons (the base bytes unchanged break nothing); a document without base bytes has `RewriteReason::NoBase`.
- **Violations err on the side of reporting:** the classes of every transaction since the last rebase (undo and redo add theirs), cleared only when the overlay is empty again; unknown when a model document is wrapped while dirty. FieldMDP: op classes don't name fields yet, so any form change counts against every lock that may cover a field. Unreadable parts of the form make the signatures unknown (counted as signed), and an unreadable DocMDP permission is taken as `/P 1`.
- **`SignatureInfo`** also has `value` (the signature dictionary) and `covers` (the end of its `/ByteRange`); `revision` counts the `/Prev` chain's xref sections that start before it (`lamina_cos::Document::xref_section_offsets`, added for this). The UR3 kind is `SignatureKind::UsageRights`.
- **Signed fixtures:** also `locked.pdf` (FieldMDP `/Lock`) and `signed-encrypted.pdf` (the encrypted + signed `Conflict`). Not byte-reproducible (fresh throwaway keys each run, never committed). No signed file from an external corpus was added.
- **pdf.js** is not in the pnpm workspace: `tools/roundtrip/package.json` pins `pdfjs-dist` 6.4.299 and `cargo xtask roundtrip` installs it with npm (no lockfile; `$LAMINA_PDFJS` overrides). **PDFium** reads no `/Title` from a direct `/Info` dictionary (`text-example.pdf`): `pdfium.py` accepts a missing title when PDFium reads none from the source either. qpdf isn't pinned in `doctor.rs`; every tool's version is recorded in `target/xtask/roundtrip/versions.json`.
- **Error codes:** `would-invalidate-signatures` (kind `security`) and `requires-rewrite` (kind `state`) in `fixtures/codes.json` (`saveErrors`) with pages in [`docs/errors/`](../../errors/README.md) ([ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) §6); `SaveError::EncryptedBase` reuses `encrypted-base` (already listed), whose page now says every mode is refused without `remove_encryption`.
- **Fuzzing (local, 10 min each):** `cargo xtask fuzz ops_json --time 600`: 5,261,872 runs, cov 5122, no crash; `cargo xtask fuzz save_roundtrip --time 600`: 187,057 runs, cov 9565, no crash (macOS ARM64, nightly, default `Limits`).

