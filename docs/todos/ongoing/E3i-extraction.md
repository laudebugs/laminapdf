# E3i — Extract text, images, links and bookmarks

| Field | Value |
| --- | --- |
| ID | `E3i` |
| Status | `ongoing` |
| Plan | [E3 plan §2.3](../../plans/editor/e3-navigation-attachments-inspection.md#23-functions-that-only-read-or-write-new-files) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B9c](../done/B9c-text-extraction-bindings-and-differential.md), [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3i-extraction` |
| PR title | `feat(text): document-level extraction of text, images, links and bookmarks` |
| Size | M (200–400): the image walk and its decoders, and two JSON schemas; the dialog is E3j's |
| PR | — |

## Goal

Give the library what it lacks for the round's extraction rows: the original embedded images (DCT and JPX passed through, others decoded with `/Decode`, masks and colour spaces honoured), and links and bookmarks as JSON and CSV. Text already comes from `laminapdf-extract` and its `@laminapdf/sdk` binding; this todo tags their tests for `A.10` and the text half of `H.07`. The dialog is [E3j](../ready/E3j-export-dialog.md)'s.

## Context

- **Design (Paper):** [O16 Export and extraction](../../design/paper/pages-navigation-extraction.md#o16-export-and-extraction). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [library B9](../../plans/pdf-library.md#b9-text-extraction)
- [05 §3.8 Image extraction](../../research/pdf/05-creating-pdfs-from-code.md#38-image-extraction)
- [03 §8 Text extraction](../../research/pdf/03-text-and-fonts.md#8-text-extraction)

## Scope

**In scope**

- `extract_images(&doc, pages)` with passthrough (DCT, JPX) and decode paths
- `doc.links()` and `doc.outline()` as JSON (`laminapdf.links/1`, `laminapdf.outline/1`, schemas committed) and CSV
- Text stays `laminapdf-extract`'s, as it is now

**Out of scope** (and which todo covers it)

- The export dialog → [E3j](../ready/E3j-export-dialog.md)
- The desktop column → [E3k](../ready/E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `H.07` extract text and images; `H.08` extract hyperlinks and bookmarks; `B.03` extract original embedded images; `A.10` extract text with position and formatting
- **Crates and packages:** `laminapdf-content` (the XObject walk), `laminapdf-extract`, image decoders (licence-checked), `laminapdf`, `@laminapdf/sdk`
- **`SaveConstraint`:** read-only
- **Web vs desktop:** the same functions in both; where the output goes is E3j's and E3k's
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `H.07` → `read` (core, pdf); `H.08` → `read` (core, pdf); `B.03` → `read` (core, pdf); `A.10` → `read` (pdf)

## Tasks

- [ ] Image walk incl. inline images and Form XObject recursion
- [ ] Decode paths (Indexed, ICC, CMYK, 16-bit, masks)
- [ ] JSON schemas for exports
- [ ] The `extract_images` fuzz target

## Acceptance criteria

- [ ] Extracted JPEGs are byte-identical to the stream data
- [ ] Image extraction matches `pdfimages -list` counts on the corpus (external CLI)
- [ ] Text export matches B9 acceptance fixtures

## Post-merge checks

None.

## Tests

- Differential vs pdfimages and pdf.js
- e2e

## Docs to update

- API docs: extraction
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- CMYK/ICC → sRGB conversion for non-passthrough images needs a colour module (lcms2-like); use a simple conversion now, proper CMS in E16d

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
