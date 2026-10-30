# E3i — Extract text, images, links and bookmarks

| Field | Value |
| --- | --- |
| ID | `E3i` |
| Status | `ready` |
| Plan | [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B9c](../ongoing/B9c-text-extraction-bindings-and-differential.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md) |
| Blocks | None |
| Dependency notes | [E3a](E3a-navigation-attachments-inspection-feature-plan.md), `B9` ([pdf-library §B9](../../plans/pdf-library.md#b9-text-extraction): content reading and text extraction) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3i-extraction` |
| PR title | `feat(text): document-level extraction of text, images, links and bookmarks` |
| Size | M (200–600) |
| PR | — |

## Goal

Export extracted content: text (plain, with positions as JSON), original embedded images (DCT/JPX passthrough, others decoded with `/Decode`, masks and colour spaces honoured), and links and bookmarks (JSON/CSV), from the library and both apps.

## Context

- **Design (Paper):** [O16 Export and extraction](../../design/paper/pages-navigation-extraction.md#o16-export-and-extraction). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [library B9](../../plans/pdf-library.md#b9-text-extraction)
- [05 §3.8 Image extraction](../../research/pdf/05-creating-pdfs-from-code.md#38-image-extraction)
- [03 §8 Text extraction](../../research/pdf/03-text-and-fonts.md#8-text-extraction)

## Scope

**In scope**

- `doc.extract_text(options)` (plain/JSON), `doc.extract_images()` with passthrough and decode paths, `doc.links()`, `doc.outline()` export
- Extract dialog in `@lamina/ui`

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `H.07` extract text and images; `H.08` extract hyperlinks and bookmarks; `B.03` extract original embedded images; `A.10` extract text with position and formatting
- **Crates and packages:** `lamina-text`, `lamina-content` (XObject walk), image decoders (licence-checked)
- **`SaveConstraint`:** read-only
- **Web vs desktop:** desktop writes to a folder; web zips
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `H.07` → `write` (core, pdf, web, desktop); `H.08` → `write` (core, pdf, web, desktop); `B.03` → `write` (core, pdf, web, desktop); `A.10` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] Image walk incl. inline images and Form XObject recursion
- [ ] Decode paths (Indexed, ICC, CMYK, 16-bit, masks)
- [ ] JSON schemas for exports
- [ ] Dialog

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
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- CMYK/ICC → sRGB conversion for non-passthrough images needs a colour module (lcms2-like); use a simple conversion now, proper CMS in E16d

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
