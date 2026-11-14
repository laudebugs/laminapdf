# E3i — Extract text, images, links and bookmarks

| Field | Value |
| --- | --- |
| ID | `E3i` |
| Status | `done` |
| Plan | [E3 plan §2.3](../../plans/editor/e3-navigation-attachments-inspection.md#23-functions-that-only-read-or-write-new-files) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B9c](./B9c-text-extraction-bindings-and-differential.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3i-extraction` |
| PR title | `feat(text): document-level extraction of text, images, links and bookmarks` |
| Size | M (200–400): the image walk and its decoders, and two JSON schemas; the dialog is E3j's |
| PR | [#565](https://github.com/laudebugs/laminapdf/pull/565) |

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

- [x] Image walk incl. inline images and Form XObject recursion
- [x] Decode paths (Indexed, ICC, CMYK, 16-bit, masks)
- [x] JSON schemas for exports
- [x] The `extract_images` fuzz target

## Acceptance criteria

- [x] Extracted JPEGs are byte-identical to the stream data
- [x] Image extraction matches `pdfimages -list` counts on the corpus (external CLI)
- [x] Text export matches B9 acceptance fixtures

## Decisions and deviations

- **Filters.** `laminapdf-syntax` decoded Flate only; it now decodes every general filter (LZW with `/EarlyChange`, ASCIIHex, ASCII85, RunLength) and the TIFF predictor, for every stream, not just images. `reportlab.pdf`'s content now interprets, and its pdf.js and pdftotext exceptions are gone; this covers part of [#108](https://github.com/laudebugs/laminapdf/issues/108) (Brotli and image codecs in `/Contents` stay unsupported). The nightly text differential's entries for these filters may now excuse nothing.
- **Where the code lives.** The walk, the colour spaces and the PNG writer are in `laminapdf-extract` (`images/`), not `laminapdf-content`: the content crate only gained `ImageDraw::resources`, which an inline image's named colour space needs. The links and outline exports are the façade's (`navigation_export.rs`), since their records are its types; their schemas are in `crates/laminapdf/schemas/` (feature `schema`).
- **Masks.** A decoded image carries its soft mask, `/Mask` stencil or colour key as transparency. A JPEG or JPEG 2000 image can't, so its mask comes out as an image of its own, as `pdfimages` writes it. The `pdfimages` comparison counts a decoded image's mask with its image for that reason.
- **Colour.** CMYK and Lab use simple formulas, until a colour-managed conversion (E16d). ICCBased takes its components' device space. A soft mask's `/Matte` is taken out of the colour. A colour space is named as `pdfimages -list` names it (`icc`, `sep`, `devn`…), and a skipped image's reason names only a family, never a name from the file. Separation comes out as its tint (darker is more ink), and DeviceN with more than one colourant is skipped: the core can't evaluate PDF functions yet.
- **JBIG2 and size.** CCITT fax and JBIG2 decode through `hayro-ccitt` and `hayro-jbig2` (Apache-2.0 OR MIT), used as decoders only. JBIG2 is a feature (`jbig2`) the WASM modules leave out, about 70 KB: there a JBIG2 image is listed as skipped. The PNG writer is extraction's own (Flate and a CRC-32) rather than the `png` crate, for the same reason. The owner raised the extract module's budget to 1,195,000 and the text module's glue to 6,300 ([ADR 0016](../../decisions/0016-package-design.md)).
- **Differential.** `pdfimages -list` runs as an outside tool (ADR 0019) through `tools/fixtures/image_facts.py` over `fixtures/images/extract.pdf`, the corpus and the goldens (66 files, 13 with images); the test reads its output, `fixtures/images/extract-facts.json`, as the text differential does, and compares each file's count of images, an image once per object as extraction gives it. There is no pdf.js image differential: pdf.js has no way to export an image's stored bytes, only rendered pixels, so `pdfimages` is the reference for both counts and descriptions. The fixture `fixtures/images/extract.pdf` is written by `tools/fixtures/extract_images.py`, including CCITT images (with and without `/BlackIs1`) encoded by T.4's code tables; no JBIG2 encoder exists here, so the JBIG2 path has no fixture, only the fuzz target.
- **TIFF predictor.** `/Predictor 2` decodes 8- and 16-bit samples; 1-, 2- and 4-bit ones are reported as an unsupported filter, as they are rare.
- **Fuzz target.** `extract_images` runs locally (`cargo +nightly fuzz run extract_images`); adding it to the nightly matrix is the owner's change.
- **Capability matrix.** B.03, H.07 and H.08 read in the core column. The `pdf` column has no collector for SDK tests yet, as E3d and E3e recorded; `extraction-exports.test.ts` names the rows in its titles. A.10 already read in the core column; the text half of H.07 is the B9 differential, now tagged.
- **E2e.** The web column's end-to-end tests belong to the export dialog (E3j).

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

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
