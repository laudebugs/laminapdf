# E3g — Version, conformance, fonts, colour profiles and object inspector

| Field | Value |
| --- | --- |
| ID | `E3g` |
| Status | `ready` |
| Plan | [E3 plan §2.3](../../plans/editor/e3-navigation-attachments-inspection.md#23-functions-that-only-read-or-write-new-files) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md), [T2c](../done/T2c-reveal-command.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3g-inspectors` |
| PR title | `feat(app): document inspectors and COS object browser` |
| Size | L (400–800): six read-only APIs and their bindings, and the inspector's five tabs |
| PR | — |

## Goal

Give power users and developers read-only inspection: header/catalog version and conformance claims (PDF/A, PDF/UA, PDF/X), fonts (type, embedded/subset, encoding, ToUnicode coverage) and colour profiles/output intents, and a COS object browser (objects, streams decoded or raw, references, xref sections per revision).

## Context

- **Design (Paper):** [O15 Object inspector](../../design/paper/pages-navigation-extraction.md#o15-object-inspector). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [06 §3 group J](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms)
- [01 §6](../../research/pdf/01-file-structure.md#6-cross-reference-streams-object-streams-hybrid-files-757758) and [§7](../../research/pdf/01-file-structure.md#7-incremental-updates-756): xref streams, object streams, incremental updates
- qpdf `--json` and `mutool show` as reference behaviour

## Scope

**In scope**

- Core APIs: `doc.conformance()` (claims "claimed, not verified", PDF/X's `GTS_PDFXVersion` too), `doc.fonts(pages)` (via B9 font reading), `doc.color_profiles()` (output intents and `ICCBased` spaces), `doc.objects()` iterator, `doc.object(ref)`, `doc.references_to(ref)`, `doc.revisions()` summary (from [T2c](../done/T2c-reveal-command.md)'s `revision_index`)
- Inspector panels in `@laminapdf/ui`: Properties → Conformance; Fonts; Colour; Object browser (tree from trailer, jump to ref, hex/text stream views with size caps)
- Bindings for all of the above in `laminapdf-wasm` / `@laminapdf/sdk`

**Out of scope** (and which todo covers it)

- Opening prior revisions read-only and diffing → [E14b](E14b-revision-browser.md)
- Editing raw objects: no round plans it ([E3 plan §11](../../plans/editor/e3-navigation-attachments-inspection.md#11-decisions-on-the-open-questions))
- The desktop column → [E3k](E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.07` inspect version and conformance; `J.08` identify fonts and colour profiles; `J.09` inspect internal objects, streams, references
- **Crates and packages:** `laminapdf-model`, `laminapdf-cos` (revision summary), `laminapdf-text` (font facts), `@laminapdf/ui`
- **`SaveConstraint`:** read-only
- **Web vs desktop:** identical (object inspector ships on both)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `J.07` → `read` (core, pdf, web); `J.08` → `read` (core, pdf, web); `J.09` → `read` (core, pdf, web)

## Tasks

- [ ] APIs and bindings
- [ ] Panels with virtualised trees
- [ ] Stream viewer with decode, size cap and `Limits`
- [ ] Tests vs qpdf `--json`

## Acceptance criteria

- [ ] Object counts, types and xref facts match `qpdf --json` on the corpus
- [ ] Font list matches `pdffonts` (external CLI in CI) on 20 files
- [ ] Inspector never decodes a stream beyond `Limits` (test with a bomb fixture)

## Post-merge checks

None.

## Tests

- Differential tests vs qpdf and pdffonts (nightly lane)
- e2e for panels

## Docs to update

- User help: inspectors
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- `pdffonts` is GPL (poppler): external CLI only in CI (ADR 0019)

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
