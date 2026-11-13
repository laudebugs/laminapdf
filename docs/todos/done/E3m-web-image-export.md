# E3m — Images to PDF and page export in TypeScript and the web reader

| Field | Value |
| --- | --- |
| ID | `E3m` |
| Status | `done` |
| Plan | [E3 plan §2.3](../../plans/editor/e3-navigation-attachments-inspection.md#23-functions-that-only-read-or-write-new-files) and [§7](../../plans/editor/e3-navigation-attachments-inspection.md#7-differences-between-the-shells) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E3h](./E3h-images-to-pdf-and-page-export.md) |
| ADRs | Follows [0002](../../decisions/0002-binding-strategy.md), [0009](../../decisions/0009-web-client-architecture.md), [0016](../../decisions/0016-package-design.md), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) |
| Branch | `feat/e3m-web-image-export` |
| PR title | `feat(sdk,viewer): images to PDF and page export in TypeScript and the web reader` |
| Size | L (400–800): a binding and its example, the worker's export call, and a small WebAssembly module for TIFF |
| PR | [#541](https://github.com/laudebugs/laminapdf/pull/541) |

## Goal

Bring E3h's two functions to TypeScript and the web reader, so E3j's export dialog has something to call. Split out of E3h on 2026-11-12 (the owner's choice), which delivered the Rust core: `images_to_pdf`, `export_pages` and the three encoders.

## Context

- E3h's Rust API: `laminapdf::images_to_pdf`, `ImagesOptions`, `ImagePages`, `Orientation`; `laminapdf_render::export_pages`, `ExportOptions`, `ExportFormat` (feature `export`).
- [E3 plan §7](../../plans/editor/e3-navigation-attachments-inspection.md#7-differences-between-the-shells): the web encodes PNG and JPEG with the canvas in the document worker, and TIFF with the `tiff` crate in a WebAssembly module of its own, loaded on first use; [§11](../../plans/editor/e3-navigation-attachments-inspection.md#11-decisions-on-the-open-questions): multi-page TIFF on the web, within the export's memory budget.
- `crates/laminapdf/examples/images_to_pdf.rs` and its golden, which the TypeScript twin must match byte for byte; it is listed in `crates/laminapdf/examples/UNUSED.md` until this todo's recipe page shows it.

## Scope

**In scope**

- `@laminapdf/sdk`: `imagesToPdf(images, options)` over the WebAssembly façade, with tsify types, a runnable `images-to-pdf.ts` twin of the Rust example (a `[[pair]]` in `tools/xtask/docs-snippets.toml`), and a recipe page that shows both.
- `@laminapdf/viewer`: the document worker's export call: pages drawn one at a time and encoded as PNG or JPEG through `OffscreenCanvas.convertToBlob`, with the resolution and quality E3h's options take.
- TIFF on the web: a small WebAssembly module of its own around `laminapdf-render`'s TIFF encoder, loaded only when a TIFF export starts, so the reader's startup size doesn't change; multi-page within the export's memory budget, else one file per page.
- The capability matrix's `pdf` column for H.02, H.06 and H.11, and `web` where the worker proves it.

**Out of scope** (and which todo covers it)

- The export dialog, the zip and images → PDF in the interface → [E3j](../ready/E3j-export-dialog.md)
- The desktop column → [E3k](../ready/E3k-desktop-navigation-and-export.md)

## Tasks

- [x] The binding, its types, the TypeScript example and its recipe page; remove the example from `UNUSED.md`
- [x] The worker's export call for PNG and JPEG
- [x] The TIFF module, its build in `cargo xtask wasm`, its size budget and lazy loading

## Acceptance criteria

- [x] `images-to-pdf.ts` writes the Rust example's golden byte for byte
- [x] The reader's startup JS and WebAssembly sizes are unchanged
- [x] A page exported by the worker as PNG has the size the resolution gives

## Decisions and deviations

- **Where `imagesToPdf` lives.** It is built into the extract and text modules only. The web reader loads the base module at startup, and images to PDF (its EXIF and resolution readers) would add about 22 KB there. The SDK's `imagesToPdf` throws `extract-module-not-loaded` until `loadExtract()` or `init({ text: true })`.
- **Glue budget.** `imagesToPdf` put the extract module's JS glue at 5,011 bytes; the owner raised its budget to 5,200 on 2026-11-13 ([ADR 0016](../../decisions/0016-package-design.md)).
- **TIFF.** laminapdf-render's TIFF encoder became `TiffWriter` (feature `tiff`), so the module `lamina_pdf_tiff` (62,374 B gzip, budget 70,000) binds the writer alone. A multi-page file holds pages while their pixels fit 256 MiB, else each page gets a file.
- **Resolution.** A page is drawn as one tile, so an export is capped at the renderers' limit (16,777,216 pixels, Letter at about 424 dpi, and 16,384 a side); the result reports the resolution used. The canvas writes no resolution, so the worker writes it: PNG `pHYs`, JFIF density.
- **Capability matrix.** No row changes. No TypeScript suite can claim the `pdf` column yet (only the reader's end-to-end tests can claim `web`, which needs [E3j](../ready/E3j-export-dialog.md)'s dialog).

## Post-merge checks

None.

## Tests

- The SDK's example suite (the golden), the worker's export in the viewer's tests, the TIFF module in Node

## Docs to update

- The recipe page; [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`

## Risks and open questions

- The TIFF module's size and how it is packaged beside `@laminapdf/wasm`'s three builds (ADR 0016)

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
