# E6c — Add text and special characters as page content

| Field | Value |
| --- | --- |
| ID | `E6c` |
| Status | `done` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B4b](./B4b-shaping-bidi-line-breaking.md), [B4c](./B4c-standard-14-and-simple-fonts.md), [B8a](./B8a-structure-tree.md), [B8b](./B8b-conformance-profiles-and-verapdf.md), [E4e](./E4e-freetext-callouts.md), [E6b](./E6b-place-images-and-shapes.md), [X3a](./X3a-nightly-lane.md) |
| Dependency notes | [E6b](../done/E6b-place-images-and-shapes.md), `B4` ([pdf-library §B4](../../plans/pdf-library.md#b4-fonts-and-text): fonts, shaping, subsetting) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e6c-add-text` |
| PR title | `feat(content): add text to existing pages with font embedding` |
| Size | L (600–1,500) |
| PR | [#830](https://github.com/laudebugs/laminapdf/pull/830) |

## Goal

Add text blocks anywhere: shaped and laid out by laminapdf-text, subset-embedded fonts with ToUnicode, font fallback for symbols and special characters, tagged as a `P` element when the document is tagged (or as Artifact on request); on the desktop, fonts on the computer join the list in [E6h](../ready/E6h-desktop-system-fonts.md).

## Context

- **Design (Paper):** [A01 Edit mode: add text box](../../design/paper/add-edit-content.md#a01-edit-mode-add-text-box). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [03 §11 Implications](../../research/pdf/03-text-and-fonts.md#11-implications-for-laminas-design)
- [library B4](../../plans/pdf-library.md#b4-fonts-and-text)
- [library B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa): tagging APIs

## Scope

**In scope**

- `AddText { id, page, rect, runs, align, fonts, size, color, tag, modified }` (the plan's payload with the text's size and colour, one each, as a text box has) building a Form XObject with text, as the [E6 plan §2.1](../../plans/editor/e6-adding-content.md#21-ops) gives it; editing and deleting added text through E6b's `UpdateContentObject` and `DeleteContentObject`
- Font registry per session: bundled fonts and user-supplied files, as bytes into the core; fonts on the computer are E6h's
- Symbol picker and fallback chain
- Tagging: if the doc is tagged, add struct elements and MCIDs (B8 machinery)

**Out of scope** (and which todo covers it)

- Fonts on the computer in the desktop font picker → [E6h](../ready/E6h-desktop-system-fonts.md)
- The Add text tool → [E6e](../done/E6e-content-tools-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.01` add text anywhere on a page; `A.12` insert special characters and symbols
- **Crates and packages:** `laminapdf-text`, `laminapdf-build`, `laminapdf-content`
- **`Op` variants:** `AddText` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked unless the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock` (the signature panel, E10h, reports the later content change)
- **Web vs desktop:** identical in the core: fonts arrive as bytes; on the web the standard 14, an on-demand OFL set and the user's files (the owner's choice, [issue #532](https://github.com/laudebugs/laminapdf/issues/532)); on the desktop, the computer's fonts too ([E6h](../ready/E6h-desktop-system-fonts.md))
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.01` → `round-trip` (core, pdf); `A.12` → `round-trip` (core, pdf) (web and desktop columns flip once, in [E6e](../done/E6e-content-tools-ui.md), with the UI)

## Tasks

- [x] AddText op
- [x] Font sources on web, as the owner decided in [issue #532](https://github.com/laudebugs/laminapdf/issues/532): the standard 14, which need no bytes; bundled OFL fonts fetched from the reader's own origin when the font picker opens and cached by the service worker; and fonts the user adds. A font served renamed (Lamina Sans, from Source Sans 3, whose licence reserves "Source") is embedded under its served name. Fonts on the computer are [E6h](../ready/E6h-desktop-system-fonts.md)'s. Done: the registry, the standard 14, the bundled Lamina Sans and Noto Sans Arabic, the user's files and the early refusals (`apps/web/src/fonts/`). Newsreader, a monospace and Noto Sans Symbols 2 as files are [E6i](../done/E6i-bundled-fonts.md) ([#831](https://github.com/laudebugs/laminapdf/issues/831))
- [x] Fallback: the chain in the op, and the chain and the symbol picker's data on the web
- [x] Tagging integration
- [x] `appearance-needs-text` in the base module; E4e's fonts in the extract module reused

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [x] Added text in five scripts extracts exactly: lamina and pdftotext read each line in `laminapdf-save`'s `added_text` test (the full lane's suites job installs poppler and requires `pdftotext` there), and pdf.js reads each line once from every `addText` output of the round trip (spaces left out of the comparison: pdf.js splits a Devanagari cluster where a vowel sign is reordered)
- [x] Fonts with restricted `fsType` are refused with a coded error
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

- The three OFL files the owner chose (Newsreader, a monospace, Noto Sans Symbols 2) are not in `@laminapdf/fonts` yet: its `subset.py` pins npm sources and refuses a TTF it didn't write, so they need a pinned source and lock entries of their own. They are [E6i](../done/E6i-bundled-fonts.md) ([#831](https://github.com/laudebugs/laminapdf/issues/831)); the registry takes them as data, and until it lands the symbol picker's maths, dingbats and currency signs report `missing` from `fallbackChain`.
- Round-trip, veraPDF (the example runs on a PDF/UA-1 report, which the nightly validates), PDFium's reading of added text and the parity matrix's render goldens run on CI only; the pdf.js and qpdf legs of the round trip, pdftotext and lamina ran locally.
- One size and one colour per text: per-run sizes and colours come with [E7f](../ongoing/E7f-font-handling-and-styling.md)'s styling.
- A file that claims PDF/UA-1 needs every font embedded (ISO 14289-1 §7.21.4.1), and `AddText` doesn't check that: text in `helvetica`, `times` or `courier` is not embedded. A check would be a new refusal or warning code that the round trip's PDF/UA fixtures and the `add_text` example (a tagged PDF/UA-1 report in Helvetica) would meet; the recipe says to add a font's bytes for such a file. A later todo can add the check with the PDF/UA work of [E15e](../ready/E15e-pdfua-output.md).
- `AnnotFont` has no Symbol or ZapfDingbats, so added text can't use them; E4e's generator doesn't draw their encodings. Symbols come from an embedded font ([E6i](../done/E6i-bundled-fonts.md)).
- A tagged file gets a `P` element or an artifact for added text, never a `Span` inside a paragraph.
- A new height in a text patch keeps the box's top edge where it was (a `matrix` in the patch places the box itself).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
