# E15c — Tagging tools: headings, lists, tables, artifacts, alt text, language, form and link labels

| Field | Value |
| --- | --- |
| ID | `E15c` |
| Status | `ready` |
| Plan | [full-editor §E15](../../plans/full-editor.md#e15-accessibility) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E15b](./E15b-structure-tree-editing.md) |
| Blocks | [E15e](./E15e-pdfua-output.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e15c-tagging-tools` |
| PR title | `feat(ui): accessibility tagging tools` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Tools that make tagging practical: select content and tag as H1–H6/P/L/LI/Table/TR/TH/TD (with scope), mark artifacts, set alt text on Figures, document and element language, Form struct elements with `/TU`, Link struct elements with `/Contents`.

## Context

- **Design (Paper):** [X07 Tags and reading order](../../design/paper/convert-compare-produce.md#x07-tags-and-reading-order). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [03 §9.3 Language, alt text, reading order](../../research/pdf/03-text-and-fonts.md#93-language-alt-text-reading-order)

## Scope

**In scope**

- Tools
- Alt-text and language dialogs
- Form/link labelling helpers

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `L.03` alt text for images; `L.04` document language; `L.05` mark decorative artifacts; `L.06` accessible form labels; `L.07` accessible link descriptions
- **Crates and packages:** `@lamina/ui`, `lamina-ops`
- **`Op` variants:** E15b ops + `SetLang`, `SetAlt`, `MarkArtifact`
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Document` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations); ops noted as `Annotation` are allowed under P3 or when there is no certification. Violations are gated (override names the signatures). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock`
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `L.03` → `round-trip` (core, pdf, web, desktop); `L.04` → `round-trip` (core, pdf, web, desktop); `L.05` → `round-trip` (core, pdf, web, desktop); `L.06` → `round-trip` (core, pdf, web, desktop); `L.07` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Tools
- [ ] Dialogs
- [ ] e2e

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Tagging a fixture with the tools makes it pass the Matterhorn machine checks it previously failed

## Post-merge checks

None.

## Tests

- e2e
- veraPDF

## Docs to update

- User help
- [full-editor §E15](../../plans/full-editor.md#e15-accessibility) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
