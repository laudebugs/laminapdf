# E6c — Add text and special characters as page content

| Field | Value |
| --- | --- |
| ID | `E6c` |
| Status | `ready` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B4b](../ongoing/B4b-shaping-bidi-line-breaking.md), [B4c](../done/B4c-standard-14-and-simple-fonts.md), [B8a](./B8a-structure-tree.md), [B8b](./B8b-conformance-profiles-and-verapdf.md), [E6b](./E6b-place-images-and-shapes.md), [X3a](../ongoing/X3a-nightly-lane.md) |
| Blocks | [E6d](./E6d-watermarks-headers-footers.md), [E6e](./E6e-content-tools-ui.md), [E7f](./E7f-font-handling-and-styling.md) |
| Dependency notes | [E6b](E6b-place-images-and-shapes.md), `B4` ([pdf-library §B4](../../plans/pdf-library.md#b4-fonts-and-text): fonts, shaping, subsetting) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e6c-add-text` |
| PR title | `feat(content): add text to existing pages with font embedding` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Add text blocks anywhere: shaped and laid out by lamina-text, subset-embedded fonts with ToUnicode, font fallback for symbols and special characters, tagged as Span/P when the document is tagged (or as Artifact on request); desktop can use system fonts discovered at the edge and passed into the core as bytes.

## Context

- [03 §11 Implications](../../research/pdf/03-text-and-fonts.md#11-implications-for-laminas-design)
- [library B4](../../plans/pdf-library.md#b4-fonts-and-text)
- [library B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa): tagging APIs

## Scope

**In scope**

- `AddText { page, rect, runs, style, tag }` building a Form XObject with text
- Font registry per session: bundled fonts (web), user-supplied, system fonts (desktop edge discovery: CoreText, DirectWrite, fontconfig) → bytes into core
- Symbol picker and fallback chain
- Tagging: if the doc is tagged, add struct elements and MCIDs (B8 machinery)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.01` add text anywhere on a page; `A.12` insert special characters and symbols
- **Crates and packages:** `lamina-text`, `lamina-build`, `lamina-content`, `apps/desktop/src-tauri` (font discovery)
- **`Op` variants:** `AddText` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked unless the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock` (the signature panel, E10h, reports the later content change)
- **Web vs desktop:** desktop: system fonts (respect `fsType` embedding permissions); web: bundled OFL fonts and user-supplied files
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.01` → `round-trip` (core, pdf); `A.12` → `round-trip` (core, pdf) (web and desktop columns flip once, in [E6e](E6e-content-tools-ui.md), with the UI)

## Tasks

- [ ] AddText op
- [ ] Font registry and edge discovery (desktop, load `/tauri-v2`)
- [ ] Fallback
- [ ] Tagging integration
- [ ] Text tool UI

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Added text in five scripts extracts exactly (pdf.js, pdftotext, lamina)
- [ ] Fonts with restricted `fsType` are refused with a coded error
- [ ] Tagged fixtures stay valid in veraPDF structure checks

## Post-merge checks

None.

## Tests

- Extraction tests
- veraPDF nightly
- e2e

## Docs to update

- API docs: adding text
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
