# E3g — Version, conformance, fonts, colour profiles and object inspector

| Field | Value |
| --- | --- |
| ID | `E3g` |
| Status | `done` |
| Plan | [E3 plan §2.3](../../plans/editor/e3-navigation-attachments-inspection.md#23-functions-that-only-read-or-write-new-files) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E3a](./E3a-navigation-attachments-inspection-feature-plan.md), [T2c](./T2c-reveal-command.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3g-inspectors` |
| PR title | `feat(app): document inspectors and COS object browser` |
| Size | L (400–800): six read-only APIs and their bindings, and the inspector's five tabs |
| PR | [#714](https://github.com/laudebugs/laminapdf/pull/714) |

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

- Opening prior revisions read-only and diffing → [E14b](../ready/E14b-revision-browser.md)
- Editing raw objects: no round plans it ([E3 plan §11](../../plans/editor/e3-navigation-attachments-inspection.md#11-decisions-on-the-open-questions))
- The desktop column → [E3k](../ready/E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.07` inspect version and conformance; `J.08` identify fonts and colour profiles; `J.09` inspect internal objects, streams, references
- **Crates and packages:** `laminapdf-model`, `laminapdf-cos` (revision summary), `laminapdf-text` (font facts), `@laminapdf/ui`
- **`SaveConstraint`:** read-only
- **Web vs desktop:** identical (object inspector ships on both)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `J.07` → `read` (core, pdf, web); `J.08` → `read` (core, pdf, web); `J.09` → `read` (core, pdf, web)

## Tasks

- [x] APIs and bindings
- [x] Panels with virtualised trees
- [x] Stream viewer with decode, size cap and `Limits`
- [x] Tests vs qpdf `--json`

## Acceptance criteria

- [x] Object counts, types and xref facts match `qpdf --json` on the corpus
- [x] Font list matches `pdffonts` (external CLI in CI) on 20 files
- [x] Inspector never decodes a stream beyond `Limits` (test with a bomb fixture)

## As built

- **Model** (`laminapdf-model/src/inspect/`): `Document::conformance()` (`ConformanceClaims`: the header's and the catalog's versions, the PDF/A claim from `xmp::Packet::pdfa_claim`, which the metadata read uses too, the `pdfuaid:` part, and PDF/X's `GTS_PDFXVersion` from `pdfxid:`, then `pdfx:`, then the Info dictionary; an Info version that differs from the XMP one is reported as `pdfx_info_conflict`, not dropped); `color_profiles()` (`ColorProfiles`, `ColorProfile`: the catalog's and each page's output intents, and every `ICCBased` space, each profile's first MiB decoded for its ICC header and `desc` tag; `IccHeader::parse` is the one ICC header reader, which `laminapdf-build`'s `IccProfile` now uses); `objects()` and `object_entry(num)` (`ObjectEntry`: kind from `ObjectKind::ALL`, `/Type`, `/Subtype` or a structure element's `/S`, stored length, and where it is, from `laminapdf-cos`'s `Document::object_location`); `reference_index()` (holder numbers only) and `referrers(index, num)` (the key paths, found by reading the holders again). One walk over every object (`each_object`) and one generation (`object_id`) serve these and the disclosure scan. The font walk has a scope (`FontScope`): content only, as `Page::fonts()` and the comment summary's PDF/UA check keep it, or with annotations' appearance streams, as `pdffonts` lists them.
- **Façade:** `conformance`, `color_profiles`, `fonts(pages)` (with appearances and ToUnicode coverage), `trailer`, `object_count`, `object_entry`, `objects`, `object(num)` (`InspectedObject` with `CosValue`, plain data per ADR 0028 F5, `StreamFacts` and the last revision to write it; read once, the revision index cached), `stream_bytes(num, view, max)`, `references_to(num)` (the index built on the first call and kept until the document changes), `object_syntax(num)` and `revisions()` (`RevisionSummary`; `reveal` lists its revisions from the same function without building a save planner). `properties()` lists content fonts without coverage, as before. `FontSummary` gains `object`, `to_unicode` and `to_unicode_coverage`, which `laminapdf-text`'s `to_unicode_coverage` counts over the one CMap parser. Stream filter names have one home, `laminapdf-cos`'s `Document::stream_filters`, which extraction uses too. The façade no longer exports `ObjRef`; references are `CosValue::Reference { num, generation }`. No new error code: a missing object or a non-stream is `None`.
- **Bindings:** WASM (extract and text modules, beside the properties view), `@laminapdf/sdk` (`doc.conformance()`, `fonts()`, `colorProfiles()`, `trailer()`, `objectCount()`, `objectEntry()`, `objects()`, `object()`, `streamBytes()`, `referencesTo()`, `objectSyntax()`, `revisions()`, `revisionCount()`, flat methods as the SDK's other reads are; a test pins the TypeScript `kind` union to `ObjectKind::ALL`) and the viewer worker (`inspectionSummary`, `inspectionTab`, `objectDetail`, `objectReferrers`, `streamRead`, on the extract module's copy).
- **Differential:** `cargo xtask differential --live --inspect`, in the nightly lane after the poppler build (which now builds `pdffonts` too, run as a process, ADR 0019), over the external corpus and the committed PDFs. It compares each file's objects with `qpdf --json=2 --json-key=qpdf` and `qpdf --show-xref` (number, generation, kind as stream, dictionary or other, `/Type`, `/Subtype` or else `/S`, and where each is stored); the newest cross-reference section with qpdf's trailer (table, stream or hybrid through `/XRefStm`, `/Size`, `/Prev`; qpdf lists no sections or revisions, so no per-revision comparison is possible); and the fonts with `pdffonts` (object, name without the subset tag, type family, embedded, subset, ToUnicode, and encoding as pdffonts spells it, a font without `/Encoding` compared by its built-in encoding). A file either side repaired has its objects and section left out. `--update` writes `fixtures/differential/inspect.json` for 31 committed PDFs (a table, a stream and a hybrid file, `fixtures/revisions/hybrid.pdf`, among them), which `crates/laminapdf/tests/inspect_differential.rs` compares in the PR lane (at least 20 files with fonts must match). The `[[known]]` exceptions and the tools' time limits are shared with the XMP differential (`live/known.rs`, `runner::run_with_timeout`). Locally, 98 of 98 files match qpdf on objects and the section and 99 of 99 match `pdffonts`, with no exceptions.
- **Limits:** a view decodes a stream only as far as it shows: `laminapdf-syntax`'s `decode_prefix` (through `laminapdf-cos`'s `decode_stream_prefix`) decodes at most `max` + 1 bytes, so the inspector's 1 MiB view of a 64 MiB + 1 bomb decodes 1 MiB + 1; asked for more than `max_stream_decoded`, the read is refused with `limit-exceeded`. ToUnicode CMaps decode within `Limits` and an unreadable one leaves the coverage out; an ICC profile is read for its first MiB; an object stream past the limit leaves its members listed as `null`. Rust tests cover each, and the SDK and web tests the bomb.
- **Interface:** `ObjectInspector` in `@laminapdf/ui`'s dialogs entry, the web's `InspectorView` in `later.ts`, the commands `file.inspect-objects` and `file.go-to-object`, More › Inspect objects and Document properties' Inspect…, which opens the Conformance tab. Everything is read when first shown: the summary (trailer, counts, version) when it opens, a tab's facts when the tab is shown, an object and the entries of what it refers to when the tree or the detail needs them, what refers to it when its detail shows. A read that fails says why with Try again. The [area document](../../design/paper/pages-navigation-extraction.md#o15-object-inspector) records the differences from O15: no Developer submenu, a treegrid, an object gone to joins the tree as a root, and Show page doesn't outline the object (both kept, since the path to an arbitrary object needs the reference index first and the viewer has no object outline to show yet). The todo's "Properties → Conformance" is the inspector's Conformance tab, opened by Document properties' Inspect….
- **Sizes:** the reader's startup script measures 288,871 bytes (the More item, the inspector's state and cover hook, the client's three calls), so the waiver rises to 289,400. The extract and text modules grow about 21 KB gzipped each (1,447,281 and 2,148,648), with their glue (6,103 and 6,643), the SDK's JS (29,641), its worker client (5,233) and worker file (27,796); each budget is about 500 bytes over, recorded in ADR 0016 §6. The base module is unchanged in budget.
- **Not done here:** the desktop column (E3k); gallery screenshots (`--update` times out in Docker here).

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

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
