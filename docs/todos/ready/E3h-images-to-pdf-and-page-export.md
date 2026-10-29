# E3h — Images → PDF and page export to PNG/JPEG/TIFF

| Field | Value |
| --- | --- |
| ID | `E3h` |
| Status | `ready` |
| Plan | [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md) |
| Blocks | None |
| Dependency notes | [E3a](E3a-navigation-attachments-inspection-feature-plan.md), `C2` ([web-app §C2](../../plans/web-app.md#c2-render-backend-and-adrs): `lamina-render`, ADR 0011) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3h-images-to-pdf-and-page-export` |
| PR title | `feat(build): images to PDF and page image export` |
| Size | M (200–600) |
| PR | — |

## Goal

Create a PDF from images (one page per image, JPEG passthrough, PNG → Flate + SMask, EXIF orientation) and export pages or ranges as PNG/JPEG/TIFF at a chosen DPI through `lamina-render`; desktop exports to a folder, web zips.

## Context

- **Design (Paper):** [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents), [O16 Export and extraction](../../design/paper/pages-navigation-extraction.md#o16-export-and-extraction). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [02 §7 Images](../../research/pdf/02-imaging-model.md#7-images)
- [05 §3.9 rasterisation](../../research/pdf/05-creating-pdfs-from-code.md#39-pdf--image-rasterisation)
- [web C2](../../plans/web-app.md#c2-render-backend-and-adrs): `RenderBackend`

## Scope

**In scope**

- `lamina_build::images_to_pdf(images, options)`
- `render::export_pages(doc, pages, format, dpi)` via `RenderBackend` (PDFium default)
- TIFF encoder choice per E3a (licence-checked), multi-page TIFF optional
- Export dialog in `@lamina/ui`

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `H.02` images → PDF; `H.06` pages → PNG/JPEG/TIFF; `H.11` render pages to images
- **Crates and packages:** `lamina-build`, `lamina-render`, `@lamina/ui`
- **`SaveConstraint`:** new document (full write) for images → PDF
- **Web vs desktop:** desktop writes files to a folder (native render, multi-threaded); web zips and streams; web memory capped by `Limits`
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `H.02` → `write` (core, pdf, web, desktop); `H.06` → `write` (core, pdf, web, desktop); `H.11` → `write` (core, pdf, web, desktop)

## Tasks

- [ ] Images → PDF with orientation and DPI handling
- [ ] Page export through the render trait
- [ ] Dialog and outputs per shell

## Acceptance criteria

- [ ] JPEG input is embedded byte-identical (DCTDecode passthrough)
- [ ] Exported PNG at 150 dpi matches PDFium golden renders
- [ ] Exporting 500 pages on web streams into a zip without exceeding the memory budget

## Post-merge checks

None.

## Tests

- Golden PDFs for images → PDF
- Render goldens
- e2e

## Docs to update

- API docs
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
