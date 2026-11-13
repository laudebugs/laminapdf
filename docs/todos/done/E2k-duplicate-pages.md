# E2k — Duplicate pages, with their annotations, form fields and tags

| Field | Value |
| --- | --- |
| ID | `E2k` |
| Status | `done` |
| Plan | [E2 page organisation §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2b](./E2b-page-structure-ops.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2k-duplicate-pages` |
| PR title | `feat(ops): duplicate pages with fresh fields and structure` |
| Size | L (400–800): one op whose work is cloning annotations, widgets and structure elements, with bindings |
| PR | [laudebugs/laminapdf#543](https://github.com/laudebugs/laminapdf/pull/543) |

## Goal

`DuplicatePages { pages, at: PagePosition }`: copy pages within a document so that the copy never shares structure keys or form fields with its source. Split out of [E2b](../done/E2b-page-structure-ops.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos); with it, `C.03` (delete and duplicate) is complete.

## Context

- **Design (Paper):** [O06 Organise mode](../../design/paper/edit-session-organise.md#o06-organise-mode) ("Duplicate on a form" toast)
- [E2 plan §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page) and [§11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions) (the `<partial>#<n>` rule)
- [03 §9 Tagged PDF](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149) and [04 §3 Forms](../../research/pdf/04-interactive-security.md#3-forms)
- ISO 32000-2 §12.7.4.2 (field names), §14.7 (logical structure: `/StructParents`, `/ParentTree`, `/ParentTreeNextKey`)

## Scope

**In scope**

- Copy the page dictionary and its annotations (new objects, new IDs, new `/NM`), sharing content streams and resources
- **Tagged files:** the copy gets a fresh `/StructParents` key (from `/ParentTreeNextKey`, which advances), and so does every copied annotation with `/StructParent`. Elements whose marked content lies only on the source page are cloned as siblings after the originals with `/Pg` set to the copy, and the new `/ParentTree` entries point at the clones (the shared stream keeps its MCIDs, so the mapping is one-to-one). MCIDs owned by elements that span pages map to null, and the report carries `DuplicateTagsIncomplete`; on a file that claims PDF/UA, the planner adds a `ConformanceWarning`. The source page's key is never reused
- **Widgets:** a duplicated widget becomes a new field, never a second widget of the original. A widget merged with its field is copied with a unique `/T` (`<partial>#<n>`, `n` the smallest number from 2 that makes the fully qualified name unique); a kid widget gets a new terminal field (`/FT`, `/V`, `/DV`, `/Ff`, `/DA`, `/Opt` copied) named `<partial>#<n>` in the original parent's `/Kids` (or `/AcroForm /Fields` for a top-level field). Signature widgets (`/FT /Sig`) are not copied; the report lists them. The naming helper is public for E2c and E2d
- `PageLabels::remap` (E2j) for the copies; `Edited::fields` lists renamed and skipped fields

**Out of scope** (and which todo covers it)

- Copying pages from other documents → [E2c](../done/E2c-cross-document-pages.md)
- The Organise command → [E2h](../ready/E2h-page-organiser-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.03` delete and duplicate pages (delete from E2b)
- **Crates and packages:** `laminapdf-ops`, `laminapdf-model` (struct and field writes), the façade and bindings
- **`Op` variants:** `DuplicatePages` (class `PageStructure`)
- **`SaveConstraint`:** as E2b. Duplicating a page that holds a locked field's widget makes a new field and leaves the locked one as it was, so with approval signatures or a FieldMDP lock it is `AppendOnly`; under a certification it violates every level ([E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint))
- **Web vs desktop:** identical (core)
- **Server features:** none
- **Capability-matrix rows to flip:** `C.03` → `round-trip` (core, pdf)

## Tasks

- [x] `DuplicatePages` with undo; annotations copied with new `/NM`
- [x] Structure cloning and `/ParentTree` entries through E2j's number-tree writer
- [x] The field-naming helper and widget-to-field rules
- [x] Bindings (`doc.duplicatePages()`; see As built), a Rust and a TS example, the frozen sample in `fixtures/ops/v1/`
- [x] A tagged form page with a tagged link annotation for the fixtures (provenance in `fixtures/README.md`)

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): the op is exposed in `laminapdf` and `@laminapdf/sdk` with docs, an example, tsify types and a frozen sample (unknown ops and fields rejected)
- [ ] Apply then undo leaves the overlay empty and the next save byte-identical; redo writes the same bytes
- [ ] `would_violate` on every signed fixture matches the [E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) table
- [ ] Round-trip: Lamina PDF, pdf.js and PDFium read the copy and everything else unchanged; `qpdf --check` is clean
- [ ] Duplicating a page of a tagged fixture gives the copy its own `/StructParents` key; the `/ParentTree` maps the copy's MCIDs to cloned elements whose `/Pg` is the copy (or to null with `DuplicateTagsIncomplete` for cross-page elements); no key is shared; veraPDF's PDF/UA result is no worse
- [ ] Duplicating a page of `forms.pdf` leaves every widget in its parent's `/Kids` (or in `/Fields`), gives each copied field a distinct fully qualified name and its own value (filling one in pdf.js and PDFium leaves the other alone), and skips signature widgets with a report entry

## Post-merge checks

None.

## Tests

- `crates/laminapdf-ops/tests/duplicate.rs`; round-trip cases in `crates/laminapdf-save/tests/roundtrip_suite.rs` tagged `C.03`
- `ops_json` fuzz dictionary gains the op

## Docs to update

- API docs: duplicating pages
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and the [E2 plan §8](../../plans/editor/e2-page-organisation.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Elements with `/K` arrays mixing MCIDs of several pages: map them to null and report rather than guess

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **The op** (`crates/laminapdf-ops/src/pages/duplicate.rs`): `DuplicatePages { pages, at }`, class `PageStructure`, no `page_changes` (a copy's widgets belong to new fields, so E2 plan §6's `AppendOnly` holds beside approval signatures and field locks; a certification still refuses it by class). A copy starts from its source's dictionary without `/Parent`, `/Annots`, `/StructParents`, `/B` and `/Type`, with the four inheritable attributes set from what the source showed, so references keep sharing content and resources. The copies go in at `at` in the order given; `relabel` continues the range each lands in (E2j's remap keeps every page's label, so a copy of "1" placed after it is "2" like the page after it).
- **Annotations and fields** (`copies.rs`): new objects (reserved first, so `/Popup`, `/Parent` and `/IRT` within the page are remapped), `/P` the copy, `/NM` `<old>-<new object number>`, `/StructParent` dropped for the structure step. A page stored inside its parent (a direct dictionary) is refused with `DirectPage`: its annotations and structure have no page object to name. Widgets: signature fields (`/FT /Sig`, inherited too) are not copied and are reported `NotCopied`; a merged field and widget gets a new `/T`; a field's kid widgets on the page get one new field (its keys but `/Kids`, `/Parent`, `/T`), so a radio group stays one group. Each new field joins its parent's `/Kids` or `/AcroForm /Fields` (`forms.rs`' list helpers, now shared by delete and duplicate). `FieldChange { field, page, outcome: FieldOutcome::{Renamed { to, name }, NotCopied} }` in `Applied::fields`.
- **Naming** (`laminapdf-model`'s `widgets.rs`): `Document::field_name` (moved from the save planner, which now calls it), `Document::field_names` and `FieldNames::unique_partial(parent, partial)` → `<partial>#<n>`, `n` the smallest from 2 not taken, recorded so the next call moves on; a copy of `name#2` continues the series as `name#3`. The names in use include fields that `/Fields` doesn't reach (every page widget's field). Public for E2c and E2d.
- **Structure** (`tags.rs`): one `/ParentTree` rewrite per op. Each copy gets the next key from `/ParentTreeNextKey` (or past the last key); each MCID's owner is cloned when all its kids are MCIDs, `MCR`s and `OBJR`s of the source page (an `OBJR` only to a copied annotation), else null. A clone has `/Pg` the copy, no `/ID`, and goes after the original in its parent's `/K` (inside the array object when `/K` is indirect). A page drawing a Form XObject with its own `/StructParents` shares that XObject's tags with its copy, so the copy counts as partly untagged. Copied annotations with `/StructParent` get keys the same way. Copies left partly untagged are `Applied::tags_incomplete`; `PendingChanges` keeps them and `pending_conformance_warnings` (which the save planner now calls) adds `duplicate-tags-incomplete` (new in `opWarnings`, with its error page) on a file whose packet claims PDF/UA-1, while the copy is still a page (undoing the duplication drops the warning).
- **Façade, WASM, SDK:** `Edited::fields` and `tags_incomplete` (`FieldChange`, `FieldOutcome`, `FieldId` re-exported); `EditedJs.fields` (`FieldEditJs`: `renamed` with `to` and `name`, or `notCopied`) and `tagsIncomplete`; `doc.duplicatePages(pages, at)` on `Document` and `Transaction` and in the worker client, with a `FieldEdit` type. **Deviation:** as in E2b, the methods sit on the document, not under `doc.pages`, which already returns the page list.
- **Examples and docs:** `duplicate_pages` and `duplicate-pages.ts` (a copy of `forms.pdf`'s first page; golden `fixtures/golden/examples/duplicate_pages.pdf`, which PDFium reads with fields `name#2` and `agree#2` holding their own values), the recipe "Duplicate pages", a paragraph in "Edit a PDF". Sizes stay within ADR 0016's budgets: base 577,855, extract 898,877, text 1,567,052 bytes gzipped; the SDK's JS 22,576.
- **Tests:** `crates/laminapdf-ops/tests/duplicate.rs` (placement and shared content, inherited attributes, labels, field names and values, `/Fields`, signatures, annotations, structure keys and clones, the cloned link's `OBJR`, the PDF/UA warning, undo, an unknown page, a copy of a copy), `duplicate_edges.rs` (hand-built files: an indirect `/K`, a field outside `/Fields`, a direct page, a tagged Form XObject, a delete on a form without `/Fields`), `pages_wire.rs` (the sample), `crates/laminapdf-save/tests/page_ops.rs` (duplicate's verdict on every signed fixture), the round-trip suite (`duplicatePages` on every fixture, tagged C.03), and `packages/sdk/test/pages.test.ts`.
