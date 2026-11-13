# E3h — Images → PDF and page export to PNG/JPEG/TIFF

| Field | Value |
| --- | --- |
| ID | `E3h` |
| Status | `done` |
| Plan | [E3 plan §2.3](../../plans/editor/e3-navigation-attachments-inspection.md#23-functions-that-only-read-or-write-new-files) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3h-images-to-pdf-and-page-export` |
| PR title | `feat(build): images to PDF and page image export` |
| Size | M (200–400): two functions and three encoders; the dialog is E3j's |
| PR | [#515](https://github.com/laudebugs/laminapdf/pull/515) |

## Goal

Create a PDF from images (one page per image, JPEG passthrough, PNG → Flate + SMask, EXIF orientation) and export pages or ranges as PNG/JPEG/TIFF at a chosen resolution through `laminapdf-render`, one page at a time. The dialog and where the files go are [E3j](../ready/E3j-export-dialog.md)'s and [E3k](../ready/E3k-desktop-navigation-and-export.md)'s.

## Context

- **Design (Paper):** [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents), [O16 Export and extraction](../../design/paper/pages-navigation-extraction.md#o16-export-and-extraction). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [02 §7 Images](../../research/pdf/02-imaging-model.md#7-images)
- [05 §3.9 rasterisation](../../research/pdf/05-creating-pdfs-from-code.md#39-pdf--image-rasterisation)
- [web C2](../../plans/web-app.md#c2-render-backend-and-adrs): `RenderBackend`

## Scope

**In scope**

- `laminapdf_build::images_to_pdf(images, options)`
- `export_pages(&render_doc, pages, &ExportOptions)` via `RenderBackend` (PDFium by default): format, resolution, one page at a time
- Encoders: PNG through `png`, JPEG through `jpeg-encoder` (`(MIT OR Apache-2.0) AND IJG` since 0.6.1: a deny.toml exception, owner decision 2026-11-12), TIFF through `tiff` (MIT), multi-page included ([E3 plan §11](../../plans/editor/e3-navigation-attachments-inspection.md#11-decisions-on-the-open-questions)), behind `laminapdf-render`'s `export` feature
- The façade's `images_to_pdf`, and a Rust example with its golden
- EXIF orientation applied by the drawing transform; page size from the image's resolution or fitted to a paper size

**Out of scope** (and which todo covers it)

- The TypeScript binding and example, the worker's export call (PNG and JPEG through the canvas) and the TIFF WebAssembly module → [E3m](../ongoing/E3m-web-image-export.md), split out on 2026-11-12 (the owner's choice) to keep this PR to the Rust core

- The export dialog, the web zip and images → PDF in the interface → [E3j](../ready/E3j-export-dialog.md)
- The desktop column → [E3k](../ready/E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `H.02` images → PDF; `H.06` pages → PNG/JPEG/TIFF; `H.11` render pages to images
- **Crates and packages:** `laminapdf-build`, `laminapdf-render`, `laminapdf` (the worker's export call in `@laminapdf/viewer` is E3m's)
- **`SaveConstraint`:** new document (full write) for images → PDF
- **Web vs desktop:** the same functions; the web encodes PNG and JPEG with the canvas and TIFF in its own small WebAssembly module, the desktop natively ([E3 plan §7](../../plans/editor/e3-navigation-attachments-inspection.md#7-differences-between-the-shells))
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `H.02` → `write` (core); `H.06` → `read` (core); `H.11` → `read` (core); the `pdf` column is E3m's

## Tasks

- [x] Images → PDF with orientation and DPI handling
- [x] Page export through the render trait
- [x] The three encoders, cargo-deny clean

## Acceptance criteria

- [x] JPEG input is embedded byte-identical (DCTDecode passthrough)
- [x] Exported PNG at 150 dpi matches PDFium golden renders

## Post-merge checks

None.

## Tests

- Golden PDFs for images → PDF (the `images_to_pdf` example), and the round-trip suite's image cases (qpdf, pdf.js, PDFium)
- Render goldens (PDFium, through `laminapdf-render-pdfium`'s ignored tests)
- e2e: E3m and E3j

## Docs to update

- API docs
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
