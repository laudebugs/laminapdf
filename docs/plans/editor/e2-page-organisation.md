# E2: page organisation

**Status:** Draft

The second editor round lets people reorganise a document's pages: change the page list (insert, delete, duplicate, move, rotate and reverse pages), bring pages in from other files, merge and split documents, change each page's geometry (crop, size, boxes and orientation), and edit page labels. Changing the page tree is easy; the work is in everything that points at a page. Outline items, links, named destinations, form fields, the structure tree and signatures all name pages, and so does the viewer, which finds pages by index. This plan fixes the ops, how each one treats those references, how the save planner gates them, and the 18 pull requests that build the round.

- **Round:** [full-editor §E2](../full-editor.md#e2-page-organisation), phase 2, milestone M5 Organiser
- **Feature-plan todo:** [E2a](../../todos/done/E2a-page-organisation-feature-plan.md)
- **Follows:** ADR [0005](../../decisions/0005-document-model.md) (the overlay and `LaminaPdfId`s), [0006](../../decisions/0006-op-log-and-undo.md) (ops and undo), [0007](../../decisions/0007-save-planner.md) (the save planner), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0023](../../decisions/0023-content-stream-reading.md) (the content crate), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space and indices), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (façade rules and jobs)
- **Design:** [O06 Organise mode](../../design/paper/edit-session-organise.md#o06-organise-mode), [O07 Insert pages and merge](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents), [O08 Split](../../design/paper/edit-session-organise.md#o08-split-document), [O09 Page geometry](../../design/paper/pages-navigation-extraction.md#o09-page-geometry), [O10 Page labels](../../design/paper/pages-navigation-extraction.md#o10-page-labels-editor), [O17 Phone](../../design/paper/pages-navigation-extraction.md#o17-phone-organise-pages)
- **Sequence:** the core todos come first and can run in two parallel lines (page tree, then content append); the viewer and interface follow; the desktop part is its own todo that waits on the desktop track ([full-editor §7](../full-editor.md#7-phases-and-rounds))

## 1. Where things stand

The library reads every page-level structure E2 edits, but writes almost none of them, and the viewer can't show an edit that changes the page list.

- **Ops.** `laminapdf-ops` has one page op, `RotatePage`, which sets one page's `/Rotate`, materialising an inherited value on the page (ISO 32000-2 §7.7.3.4). Its class is `PageStructure`; it proves `C.08` at core `round-trip` already, in `crates/laminapdf-save/tests/roundtrip_suite.rs`. The metadata ops (E1c) are the latest pattern: a payload struct per op, `deny_unknown_fields`, a frozen sample in `fixtures/ops/v1/`, an `Effect` with the touched pages. `BlobId` exists, but no blob store does: no op uses one yet.
- **Model.** `laminapdf-model` reads the page tree (`Pages`, `Page`, inherited boxes and rotation, `page_by_parents`), the outline and destinations, name and number trees, page labels (`PageLabels`, ranges and `label(index)`), the structure tree (`StructTree` with `/RoleMap`, the raw `/ParentTree` and `/ParentTreeNextKey`; no `/ClassMap` or `/IDTree`) and links. All of it is read-only. There is no AcroForm view: only the disclosure scan and the save planner walk the field tree. `LaminaPdfId` kinds are page, outline item, annotation and field.
- **Building.** `Builder::on(&mut doc, &options)` appends pages at the end of an open document's root `/Kids` through the overlay, outside the op log. Its pages write `/Rotate` and `/CropBox` themselves. The builder is prior art for inserting blank pages, but `laminapdf-build` depends on `laminapdf-ops`, so an op can't call it.
- **Content.** `laminapdf-content` only reads: a tokenizer (`ContentIter`) and an interpreter that reports to a `Visitor`. It has no write API.
- **Saving.** The planner checks each pending op class against DocMDP (§12.8.2.2), FieldMDP (§12.8.2.4) and UR3 (§12.8.2.3). Checks are per class, never per field: a FieldMDP lock is broken only by `FormFill` (`Rule::allows` in `crates/laminapdf-save/src/detect.rs`). A page op that deletes a locked field's widget, or a signed signature's widget, would therefore pass as `AppendOnly`.
- **The CLI.** `laminapdf pages rotate` turns pages through the façade and saves incrementally where it can. The qpdf range grammar with pdftk handles (ADR 0031 §3) lives in `crates/laminapdf-cli/src/ranges.rs`, and resolves to 1-based page numbers.
- **The viewer.** The document worker applies ops, writes the appended revision and reopens PDFium (E1g, E1e). Its guard refuses an edit that changes the page **count** (`samePages` in `packages/viewer/src/worker/edit.ts`, checked only for an edit that touched pages), but a reorder keeps the count and would get through. The view, the thumbnails, the text, link and find caches, the history steps and reading positions all name pages by index (`viewer.invalidate(pages, sizes)` takes indices).
- **The matrix.** Every row in scope is in `docs/capabilities/manifest.toml` with `round = "E2"`. Besides `C.08`, tagged tests already prove `C.04`, `C.09`, `C.11` and `C.14` at core `read`, and `I.10` at core `write` (the builder's page labels).

## 2. The operations

The round implements 16 rows of the [feature inventory](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms); the 17th, `C.14`, moves to E6 (§11).

| ID | What it means | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- |
| C.01 | Insert blank pages | round-trip | E2b · E2h · E2n |
| C.02 | Insert pages from other PDFs | round-trip | E2c · E2m · E2n |
| C.03 | Delete and duplicate pages | round-trip | E2k · E2h · E2n |
| C.04 | Reorder pages | round-trip | E2b · E2h · E2n |
| C.05 | Extract selected pages | write | E2c · E2m · E2n |
| C.06 | Merge multiple documents | round-trip | E2d · E2m · E2n |
| C.07 | Split by count, ranges, bookmarks | write | E2d · E2m · E2n |
| C.08 | Rotate pages 90/180/270 | round-trip | E2b (`pdf`; core is proved) · E2h · E2n |
| C.09 | Crop display area | round-trip | E2f · E2g · E2n |
| C.10 | Resize page dimensions | round-trip | E2s · E2g · E2n |
| C.11 | Modify margins and page boxes | round-trip | E2f · E2g · E2n |
| C.12 | Portrait ↔ landscape | round-trip | E2s · E2g · E2n |
| C.13 | Reverse page order | round-trip | E2b · E2h · E2n |
| C.14 | Labels, page numbers, headers/footers | round-trip in **E6** | [E6d](../../todos/ready/E6d-watermarks-headers-footers.md) |
| C.16 | Replace pages, preserving the rest | round-trip | E2c · E2m · E2n |
| H.10 | Export pages/ranges | write | E2c · E2m · E2n |
| I.10 | Page labels | round-trip | E2q · E2g · E2n |

Extract, split and export write new documents, so `write` is their target: the output passes qpdf `--check` and opens in pdf.js and PDFium, and the source is never changed. `C.05`, `C.07` and `H.10` targeted `round-trip` by default in the manifest; this plan sets them to `write`, as their todos already said.

### 2.1 Ops and functions

Every edit of the open document is an `Op` ([full-editor §5](../full-editor.md#5-requirements-that-hold-for-every-round), requirement 1). Ops name pages by `PageId` and carry every value they write; no op reads a clock, the platform or the page's position at the time it was made. A position is a `PagePosition`: `start`, `end`, `{ before: PageId }` or `{ after: PageId }`, never an index, because indices shift as the op runs. None of the round's ops needs a full rewrite.

| Op | Payload | Class | Todo |
| --- | --- | --- | --- |
| `InsertBlankPages` | `{ at: PagePosition, count, size: PageSize, rotate: Rotation }`; `size` is the `/MediaBox` width and height in points | `PageStructure` | E2b |
| `DeletePages` | `{ pages, references: ReferencePolicy }`: `retarget` (the default) or `remove` (§2.2) | `PageStructure` | E2b |
| `MovePages` | `{ pages, to: PagePosition }`: the pages keep their order among themselves and their `PageId`s (ADR 0005 §5) | `PageStructure` | E2b |
| `RotatePages` | `{ pages, angle: Rotation, mode: set \| turn }`: `turn` adds to each page's own rotation, as the toolbar's Rotate left and right do | `PageStructure` | E2b |
| `ReversePages` | `{ pages }`: reverses these pages within the slots they hold | `PageStructure` | E2b |
| `DuplicatePages` | `{ pages, at: PagePosition }` | `PageStructure` | E2k |
| `InsertPagesFrom` | `{ source: SourceId, pages: Vec<PageId>, at: PagePosition, carry: Carry }`; the pages are the source's IDs; `Carry { annotations: bool, fields: FieldClash }` | `PageStructure` | E2c |
| `ReplacePages` | `{ targets: Vec<PageId>, source: SourceId, pages: Vec<PageId>, carry: Carry }`; references to a replaced slot follow the page that takes it; slots left over are deleted under `retarget` | `PageStructure` | E2c |
| `MergeDocuments` | `{ sources: Vec<MergeSource>, at: PagePosition, options: MergeOptions }`; `MergeSource { source, pages: Option<Vec<PageId>>, title: Option<String> }`; `MergeOptions { outline: perFile \| keep \| none, fields: FieldClash }` | `PageStructure` | E2d |
| `SetPageBoxes` | `{ pages, boxes: PageBoxes }`: each of `media`, `crop`, `bleed`, `trim`, `art` absent (left alone), `null` (removed; not `media`) or a `UserSpaceRect` | `PageStructure` | E2f |
| `CropPages` | `{ pages, margins: Margins }`: top, right, bottom and left in points of the displayed page (ADR 0027 page space), so one value fits a selection of mixed sizes and rotations | `PageStructure` | E2f |
| `ResizePages` | `{ pages, size: PageSize, mode: scale \| extend { anchor: Anchor } }` | `PageStructure` | E2s |
| `SetOrientation` | `{ pages, orientation: portrait \| landscape, method: rotate \| swapBox }` | `PageStructure` | E2s |
| `SetPageLabels` | `{ ranges: Vec<LabelRange> }`: the whole `/PageLabels` tree; each range has its first page, style, prefix and first number (ISO 32000-2 §12.4.2) | `Document` | E2q |

`RotatePage` stays on the wire and in the API: `RotatePages` generalises it, and E1's rotate commands keep using it. `MergeDocuments` is one op so that a merge is one undo step; its outline, name, form and structure merges are covered by its one class, which every DocMDP level forbids anyway (ADR 0006 §1: exactly one class per op).

These work on new documents and are functions, not ops; long ones are jobs with a run-to-completion form that takes a `CancelToken` (ADR 0028 F10):

- `extract_pages(&doc, pages) -> Document` and `export_ranges(&doc, ranges) -> Vec<Document>` (E2c);
- `merge(sources, options) -> Document`, an empty base plus `MergeDocuments`, and `split(&doc, SplitBy::{Count(n), Ranges(..), Bookmarks { level }}) -> Vec<Document>` (E2d).

They take resolved page lists. Range text stays at the edges: the CLI keeps its qpdf parser (T6a reuses it for `pages split` and `pages extract`), and the interface reuses its print dialog's parser (`packages/ui/src/reader/print-pages.ts`, numbers and labels), so the app has one range grammar.

**Source documents.** `Document::add_source(bytes, password)` opens another file and returns a `SourceId`: the `BlobId` (SHA-256) of its bytes, so the same file gets the same ID on every device. The session holds the bytes in the blob store that ADR 0006 §1 describes, which [E2p](../../todos/done/E2p-source-documents-and-copier.md) builds. An op log names sources only by ID, and applying a log that names a source the session lacks fails with `unknown-source`. A password is given to `add_source` and never enters an op. An encrypted source is decrypted on read and its pages are written unencrypted.

### 2.2 What follows a page

Each op that removes, copies or moves pages reports what it changed to things that point at pages. `Applied` and the façade's `Edited` gain `references` (outline items, links, named destinations, `GoTo` actions, `/OpenAction`, structure elements, each retargeted or removed), `fields` (renamed, or a signature field not copied) and `page_order` (the page IDs in order, when the list changed). Undo restores all of it with the pages.

- **Deleted pages** follow the decision in §11: references are retargeted to the nearest surviving page, or removed with `references: remove`.
- **Moved pages** need nothing: references name page objects, which a move keeps.
- **Page labels** follow pages: `PageLabels::remap` rebuilds `/PageLabels` from page IDs after every op, so each surviving page keeps its label and new pages continue the range they land in. E2j writes it; every page op calls it.
- **Duplicated pages** share content streams and resources with their source, and get new annotation objects with new `/NM`. A widget becomes a new field named `<partial>#<n>`, signature widgets are not copied, and tagged content gets cloned structure elements with fresh `/StructParents` keys and `/ParentTree` entries (ISO 32000-2 §14.7.5.4; E2k, which keeps E2b's rules for this).
- **Imported pages** (`InsertPagesFrom`, `ReplacePages`, `MergeDocuments`) are deep-copied with fresh object numbers and new IDs; shared resources are copied once per source, matched by stream hash. Their links to pages that came with them are remapped, links to pages that didn't are removed and reported, and the named destinations their links use come with them, renamed on a clash. Fields come per `Carry::fields` (§11). Structure (the logical structure of §14.7): until E2o, imported pages lose their tags, the elements are dropped and `TagsDropped` is reported; E2o merges the trees.

## 3. Conformance the edits must keep

- **The page tree.** A page moved out of its parent first gets the attributes it inherited (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`; ISO 32000-2 §7.7.3.4), and every `/Count` on the path is kept right (§7.7.3.2). Trees edited by writers stay valid name and number trees: keys sorted, `/Limits` right, leaves bounded (§7.9.6, §7.9.7).
- **Deleted content stays in earlier revisions.** An incremental save appends; the deleted pages' objects are still in the bytes before it. `Edited::still_in_earlier_revisions`, which E1 added for removed metadata, is set by `DeletePages` and `ReplacePages` too, and the interface says so as it does for metadata. Only a full rewrite drops them ([full-editor §5](../full-editor.md#5-requirements-that-hold-for-every-round), requirement 4).
- **Cropping hides, it doesn't remove.** Content outside a new CropBox stays in the file and stays extractable. `CropPages` and `SetPageBoxes` report `CroppedContentRemains`, and a test proves the text is still there ([06 §10](../../research/pdf/06-feature-inventory.md#10-implications-for-lamina)).
- **New documents carry only what they use.** Extract, export and split write only objects reachable from the new trailer, so a part doesn't carry the whole original's fonts and images ([05 §3.1](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)).
- **PDF/A.** Pages imported or merged from a file that claims no PDF/A level (or another one) may break the target's claim: fonts not embedded, a different output intent. The op raises a `ConformanceWarning`; it warns and never blocks, as E1's do. Resizing and cropping don't affect a claim.
- **PDF/UA-1.** Imported pages without tags, a duplicate whose tags couldn't be cloned (`DuplicateTagsIncomplete`) and deleted pages whose elements other pages share each raise a `ConformanceWarning` on a file that claims PDF/UA-1. veraPDF must report nothing new on the tagged fixtures after each op (Tagged PDF, ISO 32000-2 §14.8).
- **Version.** A merge or import raises the catalog's `/Version` to the newest of its sources (§7.7.2), as `Builder::on` does for new features.

## 4. Crate and package changes

- **`laminapdf-model`** ([E2j](../../todos/done/E2j-page-tree-and-tree-writers.md)): a page-tree editor over `CosMut` (insert, remove and move kids, inherited attributes materialised, `/Count` kept, the tree rebalanced only when it passes a depth or fan-out threshold, a page dictionary listed twice copied on first change); writers for number trees and name trees; `PageLabels::remap`; and `PageReferences`, an index of what points at each page (outline items, links, named destinations in the name tree and in the PDF 1.1 `/Dests` dictionary, `GoTo` actions, `/OpenAction`, structure elements by `/Pg`, widgets of fields by `/P` or `/Annots`). [E2p](../../todos/done/E2p-source-documents-and-copier.md) adds the `ObjectCopier` (deep copy across documents, cycles and `Limits` bounded, resources matched by hash) and E2d the document-level merges (outline, names, AcroForm, `/OCProperties`). E2o adds the structure-tree merge and the read of `/ClassMap` and `/IDTree`.
- **`laminapdf-content`** ([E2e](../../todos/done/E2e-content-append-api.md)): the append API, step 2 of [full-editor §6](../full-editor.md#6-design-summary). `PageContentWriter` wraps a page's content in `q … Q` by adding streams to its `/Contents` array, appends and prepends streams, registers Form XObjects, graphics states and fonts under unused names (copying a shared `/Resources` first), and balances `q`/`Q` by a token scan over the lexer it already has. It changes an existing stream only to turn an unmatched `Q` into `q Q`. `ResizePages` and `SetOrientation { swapBox }` (E2s), flattening (E5e, E8g) and adding content (E6) use it. The crate now writes, so ADR 0023 gets a note.
- **`laminapdf-ops`**: the ops of §2.1, `Applied`'s new reports, and the blob store with `add_source` (E2p). `PendingChanges` also records, per transaction, the fields whose widgets an op deleted, rotated or moved on the page ([E2r](../../todos/done/E2r-planner-page-checks.md)), which the planner checks (§6).
- **`laminapdf-save`**: field-level checks for page ops: the planner indexes the pages that hold widgets of signed signature fields and of locked fields, and reports `Restriction::SignedWidget { field }` and `Restriction::FieldMdp` for the ops of §6 that touch them (E2r). Extract, split and merge write new documents with the full writer, keeping only reachable objects.
- **The façade and bindings.** Each op is a method on `laminapdf::Document` and on `@laminapdf/sdk`'s `Document`, with docs, a runnable example in Rust and TypeScript, tsify types and a frozen sample (requirement 7). In TypeScript the page ops sit together under `doc.pages`: `insertBlank`, `delete`, `move`, `rotate`, `reverse`, `duplicate`, `insertFrom`, `replace`, `setBoxes`, `crop`, `resize`, `setOrientation`, `setLabels`; and on the document: `addSource`, `extractPages`, `exportRanges`, `merge`, `split`. `Edited` carries `pageOrder`, `references`, `fields` and `stillInEarlierRevisions`. The CLI's page commands are [T6a](../../todos/ready/T6a-cli-page-commands.md), which waits for E2d.
- **`@laminapdf/viewer`** ([E2l](../../todos/ready/E2l-viewer-across-page-edits.md)): edits that change the page list (§5).
- **`@laminapdf/ui`**: the Organise mode (`mode.organise`, ADR 0024 §2) with its page grid, selection, drag with an insertion bar, Move to page…, and the single-document commands (E2h); the documents dialogs: insert from a file, replace, extract, export, merge and split (E2m); the page-setup dialogs: crop, resize, page boxes, orientation and page labels (E2g). Every command is in the command registry with its `pages.*` or `document.*` ID from O06, so the palette, the toolbar and the context menu share it. The new interaction is all in the Organise mode; no new interaction mode is needed.

## 5. The viewer across page-structure edits

E1 kept the page count fixed because everything in the viewer finds a page by its index ([E1 plan §6](e1-editing-foundation.md#6-redrawing-after-an-edit)). E2 is the first round that inserts, deletes and moves pages, so the viewer meets a changing page list for the first time. E2l does it before any interface todo needs it.

- **Pages by ID.** The worker's `EditResult` carries the new order of page IDs whenever an edit changed it, with each page's size. The viewer maps its state through IDs, not indices: layout, the bitmap cache (a moved page keeps its bitmaps, since it draws the same), the text, link and find caches, the thumbnail strip, the navigation history and the reading position. A deleted page's history entries and position go to the nearest surviving page.
- **The guard goes.** The worker stops refusing page-list changes, and checks instead that the model's page order after the edit matches what it reports. The check catches a reorder that keeps the count, which today's count-only guard misses.
- **History steps name page IDs.** The edit session's steps keep the IDs of the pages they touched, so a step's pages are right after later moves. Undo and redo report their order too.
- **Restarted workers.** The page's copy of the edit log replays ops that name source documents only after re-adding those sources, from the bytes it kept (E2m, the first todo with sources in the interface).
- **Measured.** The viewer's performance harness gains three cases on the 1,000-page file: delete one page, move 200 pages, insert 10 blank pages. Each is held to E1's budget, 150 ms from the edit to the first new pixels.

## 6. Saving and the save constraint

Every save plans first, and the strip says what saving will do ([E1 plan §7](e1-editing-foundation.md#7-saving-and-the-save-constraint)). Every op in §2.1 is `PageStructure` or `Document`. DocMDP permits no change of either class at any level: level 1 permits none, level 2 permits form filling, page templates and signing, and level 3 adds annotations (ISO 32000-2 §12.8.2.2). No E2 op instantiates a page template.

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update; Save as optimised rewrites, dropping deleted pages from the file | Nothing; after a delete or replace, the same note as E1's metadata removal: the old pages stay in the file's earlier versions until Save as optimised |
| `AppendOnly`: approval signatures | Signed, not certified | An incremental update: the signatures stay valid, and the signature panel (E10h) shows the page change as made after signing | The accent strip "Signed: your changes are added after the signatures" |
| `AppendOnly` → `Conflict`: a signed signature's widget (recommended rule, below) | `DeletePages`, `ReplacePages`, `RotatePages`, `CropPages`, `SetPageBoxes`, `ResizePages` or `SetOrientation` on a page that holds the widget of a signed `/FT /Sig` field | The op removes or changes the signature's visible appearance, so the planner reports `Restriction::SignedWidget`; the save is refused without the override | Before the op applies, O06's dialog names the signature and offers to change only the other pages, or to remove the signature (the O02 confirm) |
| `AppendOnly` with a FieldMDP lock (recommended rule, below) | Locked form fields (§12.8.2.4, or a signature field's `/Lock`) | Moving, reversing, inserting and duplicating keep the locked fields and their appearance, so they save incrementally. Deleting, replacing, rotating, cropping or resizing a page that holds a locked field's widget changes that field and is a violation | The accent strip; the same named-signature dialog before a violating op |
| Usage rights (UR3) | Extended by Adobe Reader rights | UR3 grants form filling, annotations and signing only (§12.8.2.3), so every E2 op is a violation and the constraint becomes `Conflict` | The same warning and confirmation as a certified file |
| `AppendOnly` with DocMDP P1, P2 or P3 | Certified | Every E2 op violates every level. The op applies only through Apply anyway…, which makes the constraint `Conflict`; it saves only through the override that removes the signatures | Before the op applies, `would_violate`'s verdict refuses it: a dropped drag animates back, and the banner names the certification with Save a copy · Apply anyway… · Details (O06). Saving then asks, naming the signatures that will be removed |
| `RequiresRewrite` | A repaired file, or a new document (extract, export, split, merge into a new file) | A full rewrite; new documents keep only reachable objects | A neutral strip saying the file will be rewritten and why; none for new documents, which have no older file |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11, as in E1; extract, export and split write unencrypted files | The warning strip; the export and split dialogs say the new files won't have a password |
| `Conflict` | Ops in the session the constraint forbids | Save is refused; Save as invalidating the signatures, after a confirmation | The danger strip with the conflicting changes and a way to undo them |

**A rule stricter than the standard (a recommendation).** ISO 32000-2 §12.8 lets approval signatures alone forbid no change, and a FieldMDP lock covers the values of the fields it names, not the keys of the page their widgets sit on; ADR 0007 §2's class rules follow that. This plan still treats deleting, replacing, rotating, cropping or resizing a page that holds a signed signature's widget as a conflict, and the same ops on a locked field's page as a FieldMDP violation. The reason: each of these changes how the signed appearance or the locked field is shown, and someone reading the saved file can't tell from a still-valid signature that the page no longer looks as it did when signed. Moving, reversing, inserting and duplicating leave that appearance as it was, so they stay `AppendOnly`. [E2r](../../todos/done/E2r-planner-page-checks.md) records the rule in ADR 0007 when it builds the checks.

New documents carry no signature: a signature covers the whole original file ([05 §3.1](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)), so extracted, split and merged files keep their signature fields unsigned, and the dialogs say so on a signed source (O08).

**The planner's new checks.** Today a FieldMDP lock is broken only by form filling, because op classes don't name fields (ADR 0007 §9). [E2r](../../todos/done/E2r-planner-page-checks.md) gives page ops field-level checks, before any page op ships. `PendingChanges` records the fields whose widgets each op deleted, rotated or moved on the page, and the planner compares them with the signed signature fields and the fields each lock covers (`/Action` with `/Fields`). `would_violate(&op)` gives the same verdict before the op applies, from the planner's index of which pages hold which widgets. This table corrects [full-editor §E2](../full-editor.md#e2-page-organisation), which said only P1 and P2 block page changes: P3 does too, and so does UR3.

## 7. Differences between the shells

The ops and dialogs are the same on both shells; the differences are where files come from and where new files go, and both are capability flags ([ADR 0010](../../decisions/0010-platform-adapter.md)).

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Several files opened together from the OS: "Merge into one document" or "Open each" | `hasFileHandlers` (exists) | The installed app in Chromium, whose launch queue can hand over several files | Finder and Explorer selections, through Open with or a drop on the icon ([D3a](../../todos/done/D3a-open-from-os.md)) |
| Files dropped on the window or the grid | none: both shells | Drop on the page | Drop on the window |
| Extract and export one file | `canSaveAsDialog` (exists) | A save dialog in Chromium, else a download | A save dialog |
| Split and export several files | `canWriteDirectory` (**new**): the platform can write files into a folder the user picks | A folder in Chromium (`showDirectoryPicker`); else one zip download, or two separate downloads for two files | A folder chosen with the dialog plugin, with Show in Finder or Explorer |
| Edit session and save | as E1 | The document worker | The engine process (E1h, E1i) |

The full-editor plan said only the desktop merges a multi-file selection from the OS. The installed web app in Chromium receives several files through its file handlers too, and `hasFileHandlers` already says when, so the interface offers the merge wherever the flag is on.

No server feature: every page edit, merge and split happens on the device, and nothing leaves it ([ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md)).

## 8. Todos

Each todo is one reviewable pull request of at most L (400–800 changed lines) on the [todo template](../../todos/TEMPLATE.md)'s scale, and each says why its size fits.

| ID | Todo | Depends on |
| --- | --- | --- |
| [E2j](../../todos/done/E2j-page-tree-and-tree-writers.md) | Page-tree editing, the number- and name-tree writers, `PageLabels::remap` and the page reference index | E2a |
| [E2r](../../todos/done/E2r-planner-page-checks.md) | The save planner's field-level checks for page ops | E2j |
| [E2b](../../todos/done/E2b-page-structure-ops.md) | Insert blank, delete, move, rotate and reverse pages, reference cleanup, bindings | E2j, E2r |
| [E2k](../../todos/done/E2k-duplicate-pages.md) | Duplicate pages, with their annotations, form fields and tags | E2b |
| [E2q](../../todos/done/E2q-page-labels-op.md) | The `SetPageLabels` op | E2j |
| [E2e](../../todos/done/E2e-content-append-api.md) | The `laminapdf-content` append API | E2a |
| [E2p](../../todos/done/E2p-source-documents-and-copier.md) | Source documents, the blob store and the object copier | E2a |
| [E2c](../../todos/done/E2c-cross-document-pages.md) | Insert and replace from other files, extract and export | E2k, E2p |
| [E2d](../../todos/ongoing/E2d-merge-and-split.md) | Merge with outlines, destinations, fields and layers; split by count, ranges and bookmarks | E2c |
| [E2o](../../todos/ready/E2o-structure-tree-merge.md) | Merge structure trees, for merged and imported pages | E2d |
| [E2f](../../todos/ready/E2f-page-geometry.md) | Crop pages and set their boxes | E2r |
| [E2s](../../todos/ready/E2s-resize-and-orientation.md) | Resize pages and change their orientation | E2e, E2f |
| [E2l](../../todos/ready/E2l-viewer-across-page-edits.md) | The viewer and edit session across edits that change the page list | E2b |
| [E2h](../../todos/ready/E2h-page-organiser-ui.md) | The Organise mode: page grid, selection, drag, Move to page… and the single-document commands | E2k, E2l |
| [E2m](../../todos/ready/E2m-document-dialogs.md) | Insert from a file, replace, extract, export, merge and split in the interface | E2d, E2h |
| [E2g](../../todos/ready/E2g-page-labels.md) | The page-setup dialogs: page labels, crop, resize, page boxes and orientation | E2h, E2q, E2s |
| [E2i](../../todos/ready/E2i-signed-gating-and-close.md) | The signed-document gating suite for every op, the cross-op labels test, and the round's close | E2g, E2m, E2o |
| [E2n](../../todos/ready/E2n-desktop-organise.md) | Desktop: Organise in the engine, merging files opened together, writing parts to a folder | E2g, E2m, E1i, D3a |

E2j, E2e and E2p can start as soon as this plan merges, in parallel; E2r and E2q follow E2j. The web and shared work, E2b to E2m, waits on no desktop todo; E2i closes the round's web and shared columns, and E2n completes the desktop column when the desktop track is ready.

## 9. Fixtures

Every new fixture is written by Lamina PDF's own scripts and licensed like the rest of Lamina PDF, `MIT OR Apache-2.0`; provenance goes in `fixtures/README.md`.

- **`fixtures/pages/`** (new, E2j, from a dependency-free `make.py` like `fixtures/metadata/make.py`): `inherited.pdf` (a three-level tree with inherited `/Resources`, `/MediaBox`, `/CropBox` and `/Rotate`), `unbalanced.pdf` (a deep one-kid chain and a page dictionary listed twice), `references.pdf` (an outline item, a named destination in both the name tree and the PDF 1.1 `/Dests` dictionary, a link, a `GoTo` action and `/OpenAction`, all at page 3), `labels.pdf` (roman front matter, arabic body, a prefixed appendix), `boxes.pdf` (all five boxes, `/UserUnit`, each rotation), `forms.pdf` (a merged field and widget, a kid widget, an unsigned signature field, names that clash with a second copy), `layers.pdf` (two optional content groups) and `chapters.pdf` (three top-level bookmarks, one through a named destination).
- **`fixtures/content/append/`** (E2e): surplus `q`, surplus `Q`, `2 0 0 2 0 0 cm Q 0 0 10 10 re f`, an inline image whose data holds `Q` bytes, and both cases split across a `/Contents` array, with a shared content stream.
- **Tagged files:** the builder's `golden/tagged-report.pdf` and `golden/tagged-across-pages.pdf` (PDF/UA-1, an element across pages) already exist; E2k adds a tagged form page with a tagged link annotation.
- **Signed files** (E2i, `fixtures/signed/make.py`, pyHanko, MIT): approval-signed with a visible signature widget on page 2, an outline, a form and tags; certified at P2 with an outline; a FieldMDP lock whose widget is on page 1. A usage-rights file's `/Perms /UR3` can't be made with a valid signature without Adobe's key, so the script writes the dictionary only; the planner reads the structure, not the signature's validity.
- **Encrypted sources:** the qpdf-encrypted copies in `fixtures/text/encrypted/`.
- **Merge benchmark:** 100 generated 10-page files, built at bench time and never committed.

## 10. Tests

Each op gets the tests its todo lists; the table says which tiers apply ([ADR 0018](../../decisions/0018-test-strategy.md)). "Round-trip" means saved, then reopened by Lamina PDF, pdf.js, PDFium and qpdf (`--check` clean), each reporting the page facts the op changed and the rest unchanged; the tagged tests in `crates/laminapdf-save/tests/roundtrip_suite.rs` flip the matrix, and `cargo xtask roundtrip` gains the ops.

| Op or function | Unit | Golden | Round-trip | Differential | Fuzz | e2e |
| --- | --- | --- | --- | --- | --- | --- |
| Tree writers, `PageLabels::remap` (E2j) | property: random edits give valid trees | — | qpdf and pdf.js read the same entries | — | — | — |
| `InsertBlankPages`, `DeletePages`, `MovePages`, `RotatePages`, `ReversePages` (E2b) | each op, its undo, its references report | an incremental save of each on `inherited.pdf` | page count, order (by each page's marker text), rotation, labels, outline targets | page facts vs pypdf on 20 corpus files | `ops_json` gains the ops | Playwright (E2h), WebdriverIO (E2n) |
| `DuplicatePages` (E2k) | tags and widget cloning | — | fields filled apart in pdf.js and PDFium; veraPDF no worse | — | `ops_json` | Playwright (E2h) |
| `ObjectCopier`, `add_source` (E2p) | copier map, dedupe, limits | — | copied pages drawn as their source in PDFium | — | new `copy_pages`: a fuzzed file's pages copied into an empty file and into `outline.pdf` | — |
| `InsertPagesFrom`, `ReplacePages`, `extract_pages`, `export_ranges` (E2c) | `/P` fix-up, links and fields that come along | `extract-pages` example output | imported pages drawn as their source in PDFium | — | `ops_json` | Playwright (E2m) |
| `MergeDocuments`, `merge`, `split` (E2d, E2o) | outline, names, field clash, layers, ParentTree renumbering | the split of `chapters.pdf` | merged forms fillable; split parts' outlines are the chapters | — | `ops_json` | Playwright (E2m) |
| `PageContentWriter` (E2e) | the scan's depth and unmatched `Q`s | wrapped streams byte for byte | 30 corpus files with a rectangle appended | PDFium and pdf.js pixels unchanged | new `content_append_scan` | — |
| `SetPageBoxes`, `CropPages` (E2f) | box validation in all four rotations | — | boxes in pdf.js; cropped text still extracted by pdftotext | — | `ops_json` | Playwright (E2g) |
| `ResizePages`, `SetOrientation` (E2s) | the destination transform for every destination kind | — | destinations in pdf.js land on the same content | — | `ops_json` | Playwright (E2g) |
| `SetPageLabels` (E2q) | range validation; labels kept after each op (E2i) | — | labels in pdf.js and PDFium match the preview | — | `ops_json` | Playwright (E2g) |

Across every op:

- **Undo:** apply then undo leaves the overlay empty and the next save byte-identical; redo writes the same bytes (ADR 0006 §3).
- **Determinism:** the same state saves to the same bytes on native and wasm32 (ADR 0007 §6).
- **Signatures:** `would_violate` on every signed fixture, asserted per op, for each row of §6; E2i runs the whole table and checks pyHanko's validation of each approval-signed save.
- **Merge budget** (a recommendation for E2d to confirm): merging 100 files of 10 pages takes at most 1 s native and 4 s in wasm32 on the CI runner, measured in `crates/laminapdf/benches/merge.rs`. E2d records what it measures and adjusts the budget, as E1g did for the redraw.
- **e2e:** Playwright in Chromium, Firefox and WebKit drives each command as a user does and checks what the page shows, then saves, reopens and checks the page facts; WebdriverIO on three systems repeats the Organise suite and the OS merge in `apps/desktop/e2e/specs/` (E2n). Each test carries its capability tag.

A new fuzz target's `fixtures/regressions/<target>` folder must also join `laminapdf-model`'s regression guard list. No E2 op draws an annotation or a field, so the appearance-stream requirement waits for E4; E2s moves widget rectangles but keeps their appearance streams.

## 11. Decisions on the open questions

The first three are the round's defaults; the owner is asked to confirm them in [issue #467](https://github.com/laudebugs/laminapdf/issues/467), and the todos follow any change.

- **Deleting a page that outline items, links or named destinations point at: retarget.** Each destination that names a deleted page is retargeted to the nearest surviving page (the next one, else the one before) with the view `/Fit`, since coordinates on another page mean nothing (§12.3.2.2). This holds for outline items, named destinations (the name stays, so links from other files and `#nameddest=` keep working), links and `GoTo` actions on surviving pages, and `/OpenAction`. Structure elements whose content was all on deleted pages are removed with their `/ParentTree` entries. Everything is reported and the toast says it ("1 bookmark now points to page 4", O06). Reasoning: an explicit destination must name a page object (§12.3.2.2), so one left pointing at a deleted page points at nothing, and dropping an outline item silently loses the user's navigation; the nearest page keeps the item useful and the report lets the user fix it. `references: remove` is there for the CLI and for scripts that want the stricter result. Deleting every page is refused with a new code that E2b adds, since a document needs a page to show.
- **Merge field-name clashes: rename by default, link on request.** An added field whose fully qualified name is already used is renamed `<partial>#<n>`, with `n` the smallest number from 2 that makes the name unique; `#` is allowed in a partial name, which only may not contain a period (ISO 32000-2 §12.7.4.2). `FieldClash::link` (O07's "keep them linked") makes the added widgets kids of the existing field so they share one value, but only when both have the same `/FT` and the same value; otherwise that field is renamed and reported. Reasoning: a naive merge turns two `invoice.total` fields into one shared value ([05 §3.1](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)); in our view (a recommendation) a rename loses nothing and a silent link changes what the user typed. The same pattern names duplicated fields (E2k) and imported ones (E2c).
- **Headers, footers and page numbers stay in E6; only labels are here.** Headers and footers are drawn content, ideally marked as artifacts (06 §3), so they need the content tools of E6 and its fonts; E6d already flips `C.14`. E2 ships page labels (`I.10`) and the label captions in Organise. The manifest's `C.14` row moves to round E6.
- **The plan's location:** `docs/plans/editor/`, as the plans index and E1 use.
