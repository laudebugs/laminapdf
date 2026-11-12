# E3e — General XMP editor (custom and extension schemas)

| Field | Value |
| --- | --- |
| ID | `E3e` |
| Status | `ready` |
| Plan | [E3 plan §2.1](../../plans/editor/e3-navigation-attachments-inspection.md#21-ops) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B8b](../done/B8b-conformance-profiles-and-verapdf.md), [E1c](../done/E1c-metadata-ops.md), [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md), [X3a](../done/X3a-nightly-lane.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3e-xmp-editor` |
| PR title | `feat(ops): general XMP editing with extension schemas` |
| Size | M (200–400): three ops over the packet E1b reads, the extension schemas, and the editor |
| PR | — |

## Goal

Edit any XMP property, including custom and PDF/A extension schemas (`pdfaExtension:`), and the raw packet for power users, without breaking the Info sync from E1 or PDF/A identification.

## Context

- **Design (Paper):** [O04 Document info and XMP editor](../../design/paper/edit-session-organise.md#o04-document-info-and-xmp-editor). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §5 Metadata](../../research/pdf/04-interactive-security.md#5-metadata)
- ISO 16684-1; ISO 19005-2 §6.6 (extension schemas)

## Scope

**In scope**

- `SetXmpProperty { namespace, prefix, path, value }`, `RemoveXmpProperty`, `ReplaceXmpPacket { xml }` (parsed before it applies; the packet travels in the op, not as a blob)
- `pdfaid:` and `pdfuaid:` are refused (`xmp-claim-read-only`), in a property op or a replaced packet ([E3 plan §11](../../plans/editor/e3-navigation-attachments-inspection.md#11-decisions-on-the-open-questions))
- Extension-schema declarations generated when adding custom properties to PDF/A files
- XMP panel: tree editor plus raw XML view with validation errors

**Out of scope** (and which todo covers it)

- Converting a file to PDF/A → E16 (`H.12`)
- The desktop column → [E3k](E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.04` manage XMP
- **Crates and packages:** `laminapdf-model` (`xmp`), `laminapdf-ops`, `@laminapdf/ui`
- **`Op` variants:** the three ops above (class `Metadata`)
- **`SaveConstraint`:** as E1c
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `J.04` → `round-trip` (core, pdf, web)

## Tasks

- [ ] Ops over the E1b tree
- [ ] Extension schema generator
- [ ] Panel with raw view
- [ ] Keep Info sync when Info-mapped properties change

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] veraPDF passes on a PDF/A-2b fixture after adding a custom property (extension schema declared)
- [ ] Invalid raw XML is rejected before applying

## Post-merge checks

None.

## Tests

- Unit tests
- veraPDF nightly
- e2e

## Docs to update

- API docs: XMP
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
