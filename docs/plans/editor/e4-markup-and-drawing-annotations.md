# E4: markup and drawing annotations

**Status:** Draft

The fourth editor round lets people mark up a document the way they mark up paper. They can highlight, underline and strike through text, leave notes and suggested insertions, draw ink and shapes, and type into text boxes and callouts. Every annotation they make carries an appearance stream that Lamina PDF generates, so Acrobat, pdf.js, PDFium and Preview show the same marks. This plan fixes the ops, where the appearance generator lives, how the viewer shows an annotation while it is being drawn, how the save planner gates each op, and the 10 pull requests that build the round.

The hard part is the appearance stream. An annotation without one looks different in every viewer and may not print. A FreeText box needs text laid out, fonts embedded and rich text kept, so a FreeText appearance needs a small layout engine of its own.

- **Round:** [full-editor §E4](../full-editor.md#e4-markup-and-drawing-annotations), phase 3, milestone M6 Annotator
- **Feature-plan todo:** [E4a](../../todos/done/E4a-markup-and-drawing-annotations-feature-plan.md)
- **Follows:** ADR [0004](../../decisions/0004-render-backend.md) (render backends), [0005](../../decisions/0005-document-model.md) (the overlay and `LaminaPdfId`s), [0006](../../decisions/0006-op-log-and-undo.md) (ops, undo and coalescing), [0007](../../decisions/0007-save-planner.md) (the save planner), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0024](../../decisions/0024-app-shell-ux.md) (modes and the command registry), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space)
- **Sequence:** the model, then the ops with each subtype's appearance, then the viewer's annotation layer and the tools; the desktop part and the round's close are their own todos ([full-editor §7](../full-editor.md#7-phases-and-rounds))
- **Builds on:** [E1](e1-editing-foundation.md) (the edit session, undo and the save strip), [E2](e2-page-organisation.md) (the content append API, E2e, the tree writers, E2j, and the blob store, E2p) and [E3](e3-navigation-attachments-inspection.md) (the link ops, E3c, which share the annotation core)
- **Design:** [C01 Comment tools](../../design/paper/comment-review.md#c01-comment-tools), [C02 Ink and shapes](../../design/paper/comment-review.md#c02-ink-and-shapes), [C03 Free text and callouts](../../design/paper/comment-review.md#c03-free-text-and-callouts), [C08 Phone annotate](../../design/paper/comment-review.md#c08-phone-annotate), [C09 Tablet stylus](../../design/paper/comment-review.md#c09-tablet-stylus)

## 1. Where things stand

The library reads links and lists every annotation for the disclosure scan, but it has no typed annotation view, no annotation op and no appearance generator.

- **Reading.** `laminapdf-model` reads Link annotations (`link.rs`: `/Rect`, `/QuadPoints`, `/Contents`, a destination or action, hidden ones left out). `Page::annots()` returns the `/Annots` references, and the disclosure scan (`disclosure/forms.rs`) records each annotation's subtype, author and contents. Nothing reads `/NM`, `/AP`, `/Popup`, `/IRT`, `/RC`, `/DA` or `/InkList`. `AnnotId` exists as a `LaminaPdfId` kind (`a12.0`) and nothing uses it yet.
- **Ops and the planner.** The op set is E1's (`SetInfo`, `RotatePage`, `SetMetadata`, `RemoveMetadata`) plus E2's and E3's as they merge; E3c's `AddLink` is still a todo. `ModificationClass::Annotation` exists. Under DocMDP P3 the planner permits it; P1 and P2 refuse it; FieldMDP locks and approval signatures leave it alone; and UR3 permits it where `/Annots` grants `/Create`, `/Modify` or `/Delete` (`constraint.rs`, `detect.rs`).
- **Drawing.** The builder writes Link annotations only, with no `/AP` and no Form XObjects. It already writes everything an appearance draws: paths, dashes, caps and joins (`surface/style.rs`), opacity and blend modes as ExtGStates (`resources.rs`), and text laid out by `Surface::text_box` over `laminapdf-text`'s `break_spans` and `pdf_run`. `laminapdf-build` depends on `laminapdf-ops`, so no op can call it.
- **Rendering.** `RenderFlags::annotations` turns every annotation on or off; nothing hides one. PDFium draws annotations with `FPDF_ANNOT` and hayro draws them too. After an edit, the viewer's worker saves the revision into memory, reopens the render document and redraws the touched pages (E1g), within E1's budget of 150 ms to the first new pixels.
- **The viewer and the interface.** The viewer has a link layer in its `annot` slot, a text layer and a find-highlight layer, but no annotation layer. `@laminapdf/ui` registers only the `read` mode and the `select`, `hand` and `textSelect` tools; the tool registry's header comment already shows how a round adds `annotate`.
- **Fixtures.** `fixtures/appearance/annotated.pdf` has one Highlight and one Square; each `fixtures/encrypted/` file has one Text annotation. No golden file has an E4 subtype. `fixtures/signed/` has approval, certified P1, P2 and P3, a FieldMDP lock and two-signature files, but no usage-rights file.
- **The matrix.** Every row in scope is in `docs/capabilities/manifest.toml` with `round = "E4"` and no target yet; this plan sets the targets (§2).

## 2. The operations

The round implements 8 rows of the [feature inventory](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms). Each row targets `round-trip`: the annotation is made, saved, and reopened unchanged by Lamina PDF, pdf.js and PDFium.

| ID | What it means | Ops (§2.1) | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| D.01 | Highlight, underline, strike through (and squiggly) | `AddAnnot` with `TextMarkup` | round-trip | E4c · E4f · E4h |
| D.02 | Sticky notes and text comments | `AddAnnot` with `Note` | round-trip | E4c · E4f · E4h |
| D.03 | Text-correction annotations | `AddAnnot` with `Caret`, grouped with a `TextMarkup` | round-trip | E4c · E4f · E4h |
| D.04 | Freehand ink | `AddAnnot` with `Ink` | round-trip | E4d · E4f, E4k · E4h |
| D.05 | Shapes, lines, arrows | `AddAnnot` with `Shape`, `Line`, `Poly` | round-trip | E4d · E4f · E4h |
| D.06 | Callouts and text boxes | `AddAnnot` with `FreeText` | round-trip | E4e · E4j · E4h |
| D.16 | Generate appearance streams (cross-cutting) | every op of the round | round-trip | E4c–E4e, E4g · E4j · E4h |
| B.06 | Arrows, paths, freehand drawing | as D.04 and D.05 | round-trip | E4d · E4f · E4h |

`B.06` is drawing as annotations, as [full-editor §E4](../full-editor.md#e4-markup-and-drawing-annotations) scopes it; drawing into the page content is E6's. `D.16` reaches `round-trip` only when E4g's parity suite shows every subtype's appearance alike in pdf.js, PDFium and hayro.

### 2.1 Ops

Every op is class `Annotation`, and none needs a full rewrite (`requires_rewrite` is false). Ops name pages by `PageId` and annotations by `AnnotId`, and carry every value they write: the `/NM`, the author, both dates and every coordinate come in the payload, never from a clock, the settings or the view. Geometry is in user space (`UserSpacePoint`, `UserSpaceRect`, and E3's `UserSpaceQuad`, which E4b adds if E3c hasn't), so a later rotation or crop leaves an op's meaning unchanged; the façade takes page space and converts (ADR 0027). An optional field is absent to leave a value alone and `null` to remove it.

| Op | Payload | Todo |
| --- | --- | --- |
| `AddAnnot` | `{ id: AnnotId, nm, page: PageId, common: AnnotCommon, spec: AnnotSpec }` | E4b (core), E4c–E4e (specs) |
| `UpdateAnnot` | `{ id, common?: AnnotCommonPatch, spec?: AnnotSpecPatch, translate?: Vector }`; `translate` moves any subtype, its appearance kept | E4b |
| `DeleteAnnot` | `{ id }`: removes it, its Popup, every reply to it and every other member of its group | E4b |

`AnnotCommon` holds `/Contents`, the author `/T`, `/Subj`, `/CreationDate`, `/M`, the colour `/C`, the opacity `/CA`, the flags `/F`, and `group_with: Option<AnnotId>`, which writes `/IRT` and `/RT /Group`. Replies (`/RT /R`) and review state are E5's. `AnnotSpec` has one variant per kind of mark:

| Spec | Subtypes | Payload | Todo |
| --- | --- | --- | --- |
| `TextMarkup` | Highlight, Underline, StrikeOut, Squiggly | `{ kind, quads: Vec<UserSpaceQuad> }` | E4c |
| `Note` | Text with a Popup | `{ at: UserSpacePoint, icon: NoteIcon, open: bool }`; the seven standard icons | E4c |
| `Caret` | Caret | `{ rect, symbol: None \| Paragraph }` | E4c |
| `Ink` | Ink | `{ strokes: Vec<InkStroke>, width, smoothing: bool }`; `InkStroke { points, widths: Option<Vec<f32>> }` (§11) | E4d |
| `Shape` | Square, Circle | `{ kind, rect, interior: Option<Color>, border: Border }` | E4d |
| `Line` | Line | `{ from, to, ends: [LineEnding; 2], interior, border, leader: Option<Leader> }` | E4d |
| `Poly` | Polygon, PolyLine | `{ kind, vertices, ends, interior, border }` | E4d |
| `FreeText` | FreeText | `{ rect, text: RichText, font: FontRef, size, align, intent: Plain \| Callout { line, knee } \| Typewriter, fill, border }`; `FontRef` is a standard 14 name or a font's bytes by `BlobId` (E2p) | E4e |

`Border` is a width and a style: solid, dashed with its pattern, or cloudy with an intensity (`/BE`). `LineEnding` names every `/LE` style. `RichText` is paragraphs of spans, each plain, bold or italic (§11).

A text correction is two `AddAnnot`s in one transaction: a StrikeOut over the replaced text and a Caret grouped with it, with the replacement text as the group's `/Contents`. The façade's `replace_text` helper makes both, so the edit is one undo step. Which member is the group's primary and holds the text follows Acrobat's files, which E4c checks against a reference file (§9).

`AddLink`, `UpdateLink` and `DeleteLink` stay the only way to make and change a link, with their frozen wire form (E3 plan §2.1). `AnnotSpec` has no `Link` variant. Whichever of E3c and E4b merges second rebuilds the link ops over the annotation core, and a test holds the two paths to the same bytes.

**Rules every op keeps:**

- **Every annotation the round makes has an appearance.** `/AP /N` is a Form XObject whose `/BBox` and `/Matrix` map onto `/Rect` (ISO 32000-2 §12.5.5), and `/Rect` grows to hold the stroke width, line endings and cloudy border. `UpdateAnnot` regenerates the appearance whenever it changes what the appearance draws; `translate` alone moves `/Rect` and the geometry keys and keeps the appearance, since §12.5.5 maps it onto the new `/Rect`. Editing an annotation of the round's subtypes that another app made regenerates its appearance from its keys, and the inspector says so before the first change.
- **Opacity and blending live in the appearance.** A highlight multiplies through an ExtGState `/BM /Multiply` inside its appearance, and opacity is set once, so no reader applies it twice. PDF 2.0 tells a reader that draws `/AP` to ignore the annotation's `/CA` (§12.5.2 errata), so a PDF 2.0 file gets opacity only from an ExtGState in the appearance. For earlier files, E4c measures whether pdf.js and PDFium apply `/CA` on top of an appearance before it freezes its goldens: if either does, opacity goes in the appearance alone; if neither does, `/CA` is written too, for readers that redraw from the keys.
- **Versions.** The ops never write the annotation-level `/BM` or `/ca`, which are PDF 2.0 keys, and never raise `/Version`. Caret, Polygon and PolyLine need PDF 1.5, and the callout's `/CL` and `/IT` need 1.6; in an older file they are written anyway, as Acrobat does, and readers that predate them ignore them.
- **Flags.** Every new annotation has `/F` Print set. Notes also have NoZoom and NoRotate, as Acrobat writes them. `UpdateAnnot` refuses a Locked annotation (`annot-locked`), and a LockedContents one refuses a change to its `/Contents` or text.
- **Unknown keys survive.** An update changes only the keys it owns; any other key, including another app's private ones, is kept.
- **`/NM` is a hint.** The payload's `nm` is written as `/NM`, unique on its page; the `AnnotId` stays the identity (ADR 0005 §5). The `/NM` index reports duplicates it reads as a warning, `annot-name-duplicate`.
- **Deleting is thorough.** `DeleteAnnot` removes the annotation from its page's `/Annots`, with its Popup and every reply whose `/IRT` reaches it. A group is one unit (ISO 32000-2 §12.5.6.2), so deleting any member of a `/RT /Group` removes the group's primary and every member grouped with it, whichever way the `/IRT` points. Chains are followed through cycles once. `Applied::also_deleted` lists them, and `still_in_earlier_revisions` is set, as for E3's deletions.
- **Undo.** Each op has an inverse; apply then undo leaves the overlay empty (ADR 0006 §3). A drag, a resize and a typing session coalesce into one undo step through `transact_coalescing`, keyed by the annotation and the kind of change.

### 2.2 Where appearances are generated

The appearance generator lives in `laminapdf-build`, which already draws everything an appearance needs. `laminapdf-ops` can't depend on it, so the ops call a trait instead. `laminapdf-ops` defines `Appearances`, with one method that writes a spec's Form XObject into the overlay through the op's `CosMut` and returns its reference and the grown `/Rect`. `laminapdf-build` implements it. `laminapdf_ops::Document` holds an optional implementation (`with_appearances`), and the façade's `Document` always sets it. Without one, `AddAnnot` and any `UpdateAnnot` that changes what the appearance draws fail with `appearance-unavailable`, so no path writes an annotation without an appearance. Tests below the façade (the round-trip suite, the `ops_json` fuzz entry) set the builder's implementation, which a dev-dependency on `laminapdf-build` allows.

The same split keeps the WebAssembly modules in budget. The base module's builder has no fonts, so it generates every appearance but FreeText's. A FreeText op in a document opened in the base module fails with `appearance-needs-text`. The viewer's worker opens every document in the extract module, which reader mode and copy need, so E4e adds the builder's fonts to the extract module and measures the cost against `packages/wasm/size-budgets.json`. A document can't move between modules once open, so the module that edits must be the one that reads. If the fonts push the extract module over its budget, E4e stops and asks the owner whether to raise it or split the module.

The generator is deterministic: the same op on the same document writes the same bytes on native and wasm32. A replayed op log regenerates appearances with the running version's generator, so its bytes can differ across versions, which ADR 0006's log allows.

### 2.3 Reading annotations

`doc.page_annotations(index)` returns each annotation on a page as a record in page space: its `AnnotId`, `/NM`, its index in the page's `/Annots`, subtype, rectangle, colour, opacity, flags, author, dates, `/Contents`, its geometry (quads, ink strokes, vertices, line, callout), `/IRT` and `/RT`, and whether it has an appearance. Unknown subtypes come back with the common fields alone. The viewer's annotation layer, the inspector and E5's comments panel all read this record. It is built from typed views in `laminapdf-model` (`Annotation`, `MarkupAnnotation`, `Popup` pairing), which tolerate every shape the corpus holds: quads in Acrobat's order or the standard's, `/State` as a name or a string, and a `/Popup` pointing anywhere.

## 3. Conformance the edits must keep

- **Tagged PDF and PDF/UA-1.** In a tagged file, a new annotation becomes an `Annot` structure element with an `OBJR`, a `/StructParent` and a `/ParentTree` entry, placed after the element that owns the marked content under it, else at the end of its page's elements (ISO 14289-1 §7.18.1). `/Contents` is its alternative description: the interface fills it with the quoted text for text markup and with the kind of mark for drawings, and a file that claims PDF/UA-1 raises `annot-without-contents` when it ends up empty. An annotation that can't be placed raises `annot-not-tagged` on a PDF/UA-1 file. A page that gains an annotation also gets `/Tabs /S` when it has no `/Tabs`, as ISO 14289-1 §7.18.3 requires of every page with annotations. Like E3c's links, these entries go with the annotation, so the op stays `Annotation`; §6 says how P3 treats them.
- **PDF/A.** PDF/A-2 and -3 require an appearance, the Print flag, and none of the hidden flags (ISO 19005-2 §6.3.2). New annotations meet all three; an `UpdateAnnot` that clears Print or sets Hidden, Invisible, NoView or ToggleNoView on a PDF/A file raises `pdfa-annot-flags`. Device colours must match the output intent (§6.2.4.3), so on a PDF/A file the appearance paints in the intent's device space, converting an RGB colour to CMYK by E3i's simple conversion when the intent is CMYK. PDF/A-1 forbids transparency, so a highlight or a translucent mark on a file that claims PDF/A-1 raises `pdfa-transparency`, and a FreeText in a standard 14 font raises `font-not-embedded` on any PDF/A file. Like E1's conformance warnings, these never block the op; the interface offers an embedded font for text and an opaque colour for marks.
- **Nothing hides text by accident.** A filled shape at full opacity over text keeps that text in the file, which is the classic failed redaction. E4f shows the covered-text notice, and the op itself changes nothing in the content.
- **Read-only permissions hold.** A file whose permissions forbid annotating (ISO 32000-2 Table 22, bit 6) opens with the tools disabled (C01). Only an encrypted file carries permissions, and saving one waits on E11 (§6), so the ops need no check of their own until E11 decides how an owner password lifts them.

## 4. Crate and package changes

- **`laminapdf-model`**: the annotation views, the markup and Popup views, the `/NM` index, and writers for the geometry keys (E4b).
- **`laminapdf-ops`**: `AddAnnot`, `UpdateAnnot`, `DeleteAnnot`, `AnnotSpec` and its patches, the `Appearances` trait, and `Applied`'s reports (E4b; specs in E4c–E4e).
- **`laminapdf-build`**: the generator for each spec: text markup, notes and caret (E4c); ink, shapes, lines and cloudy borders (E4d); FreeText layout, `/DA` and the rich-text subset (E4e).
- **`laminapdf-render`, `@laminapdf/render-pdfium` and `-hayro`**: `RenderRequest::hidden_annotations`, a list of annotations left out of one render (E4i, §5).
- **The façade and bindings.** Each op is a method on `laminapdf::Document` and on `@laminapdf/sdk`'s `Document`, with docs, a runnable example in Rust and TypeScript, tsify types and a frozen sample in `fixtures/ops/v1/` (requirement 7). In TypeScript they group as `doc.annotations`: `list(page)`, `add`, `update`, `move`, `delete` and `replaceText`.
- **`@laminapdf/viewer`**: the annotation layer in the `annot` slot (E4i, §5).
- **`@laminapdf/ui`**: `mode.comment` and its secondary toolbar, the `comment.*` tools and the `annot.*` inspector commands (E4f, E4j); the pen, highlighter, eraser and lasso for touch and stylus (E4k). Every command is in the command registry with its C-screen ID.

## 5. The viewer while drawing

PDFium draws every committed annotation from its saved appearance, and the annotation layer draws only what is being made. Full-editor §E4 first sketched two rendering paths, Lamina PDF's model until save and PDFium after. The two would differ in small ways, so a mark would shift on save. Since E1g, each edit already reopens the render document over the new revision, so the committed state is always PDFium's own picture of the bytes that will be saved.

- **Making a mark.** While a pointer draws, the layer draws a preview in SVG: the stroke, the shape or the box. On release the op applies, the page redraws, and the layer removes the preview in the same frame the new bitmap shows, so nothing flickers.
- **Moving, resizing and editing.** For the length of a gesture, the page renders with that annotation left out (`hidden_annotations`), and the layer draws the annotation's preview at its new place. PDFium leaves one annotation out by setting its Hidden flag on the loaded page for that render only. It finds the annotation by its index in the page's `/Annots`, which the record of §2.3 carries, since PDFium names annotations by index and `/NM` is only a hint. A render with hidden annotations has its own view key and never enters the bitmap cache, so a cancelled gesture or the thumbnail strip never shows the page without it. hayro, the CI oracle, takes the same request; E4i finds out whether it can honour it and records the answer in the adapter's docs. E4i also measures the extra render against E1's budget.
- **Selection and keyboard.** The layer hit-tests the page's annotation records (§2.3) and draws the selection handles of design language §7.32. Each annotation is a button in reading order, so Tab reaches it and the arrow keys move it (C02).
- **Text boxes.** While a FreeText is being typed into, a `contenteditable` box sits over it with the annotation hidden in the render; leaving it applies the coalesced `UpdateAnnot` (E4j).

## 6. Saving and the save constraint

Every save plans first, and the strip says what saving will do ([E1 plan §7](e1-editing-foundation.md#7-saving-and-the-save-constraint)). All of the round's ops are class `Annotation`. DocMDP P3 permits annotations, P1 and P2 forbid them (ISO 32000-2 §12.8.2.2), and approval signatures and FieldMDP locks restrict no annotation.

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update | Nothing; after a delete, the note that the annotation stays in earlier versions until Save as optimised |
| `AppendOnly`: approval signatures | Signed, not certified | An incremental update | The status-bar note "Signed file. Saving adds your changes after the signature; the signature stays valid." (C01) |
| `AppendOnly` with a FieldMDP lock | Locked form fields | An incremental update: no E4 op touches a field or a widget | The same note |
| `AppendOnly` with DocMDP P3 | Certified, annotations allowed | An incremental update | The same note |
| `AppendOnly` with DocMDP P1 or P2 | Certified | Every op violates; applied through Apply anyway…, it makes the constraint `Conflict` | The banner from the moment Comment mode opens (C01): Save a copy… · Details |
| Usage rights (UR3) | Extended by Adobe Reader rights | Allowed where `/Annots` grants the op's right: `/Create` for `AddAnnot`, `/Modify` for `UpdateAnnot`, `/Delete` for `DeleteAnnot` (§12.8.2.3); otherwise a violation | The note, or the banner |
| `RequiresRewrite` | A repaired file | A full rewrite | A neutral strip saying why |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11, as in E1 | The warning strip |
| `Conflict` | Ops in the session the constraint forbids | Save is refused; Save as invalidating the signatures, after a confirmation | The danger strip with the conflicting changes and a way to undo them |

**Tagged annotations under P3.** In a tagged file an annotation also adds a structure element and a `/ParentTree` entry. E3c already checks, with pyHanko's DocMDP check on the certified-P3 fixture, whether a certifier counts these structure entries as a change. E4b follows E3c's answer, or runs the same check itself if E3c hasn't merged: if they count, annotations on certified tagged files are added untagged and raise `annot-not-tagged`.

**Usage rights by op.** The planner grants the whole `Annotation` class when `/Annots` lists any right (`detect.rs`), which is looser than §12.8.2.3: a file that grants only `/Create` would let a delete through. E4b checks the right per op in `would_violate`, as E3f does for `/EF`, so each op is held to its own right.

**Annotations added after certification.** Annotations allowed at P3 can still change what a signed page appears to say, which the "Evil Annotation Attack" research showed. E10's signature panel lists annotations added after each signature; this round only keeps the revisions that make that list possible.

## 7. Differences between the shells

The ops, the tools and the inspector are the same on both shells; the differences come from the input device, and only fonts need a capability flag ([ADR 0010](../../decisions/0010-platform-adapter.md)).

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Pen pressure | none: `PointerEvent.pressure` where the device reports it | Pens and Force Touch trackpads vary the width; a mouse draws a constant width | The same, Wacom tablets included |
| Palm rejection and Draw with finger | none: `pointerType` | Tablets and phones (C08, C09) | Touch screens on Windows |
| Fonts for text boxes | `systemFonts` (proposed with E6c) | The bundled fonts and the document's embeddable ones | The same until E6c; then system fonts, passed to the engine as bytes |
| The native Comment menu | as D4b | — | Every tool in the menu bar |
| Edit session and save | as E1 | The document worker | The engine process (E1h, E1i) |

No server feature: every annotation is made, drawn and saved on the device ([ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md)). Sharing comments with other people is E18's.

## 8. Todos

Each todo is one reviewable pull request of at most L (400–800 changed lines) on the [todo template](../../todos/TEMPLATE.md)'s scale, and each says why its size fits. The core todos flip the `core` and `pdf` columns, the interface todos `web`, and E4h `desktop`.

| ID | Todo | Depends on |
| --- | --- | --- |
| [E4b](../../todos/ready/E4b-annotation-model-and-overlay.md) | Annotation views, `page_annotations`, the `/NM` index, `AddAnnot`'s core, `UpdateAnnot`, `DeleteAnnot` and the `Appearances` trait | E1d, E2j |
| [E4c](../../todos/ready/E4c-appearances-markup-notes-caret.md) | Text markup, notes and caret with their appearances; `replace_text` | C5b, E4b |
| [E4d](../../todos/ready/E4d-appearances-ink-and-shapes.md) | Ink, shapes, lines, arrows, polygons and cloudy borders with their appearances | E4b |
| [E4e](../../todos/ready/E4e-freetext-callouts.md) | FreeText text boxes, callouts and typewriter text: `/DA`, layout, the rich-text subset | B4b, B4c, E2p, E4b |
| [E4i](../../todos/ready/E4i-viewer-annotation-layer.md) | The viewer's annotation layer: records, hit-testing, handles, previews, `hidden_annotations` | E1g, E4b |
| [E4f](../../todos/ready/E4f-annotation-tools-ui.md) | Comment mode: the toolbar, the markup, note, caret, ink and shape tools, the inspector, the covered-text notice | E1e, E4c, E4d, E4i |
| [E4j](../../todos/ready/E4j-text-box-tools.md) | Text boxes and callouts in the interface: in-place editing, fonts, bold and italic | E4e, E4f, E4g |
| [E4k](../../todos/ready/E4k-touch-and-stylus-annotating.md) | Annotating on phones and tablets: the annotate bar, the tool shelf, pen, highlighter, eraser, lasso, palm rejection | E4f, U3a |
| [E4g](../../todos/ready/E4g-appearance-parity-suite.md) | The cross-viewer appearance parity suite and the corpus edit test | E4c, E4d, E4e, X3a |
| [E4h](../../todos/ready/E4h-desktop-annotations.md) | Desktop: the round in the engine, the native Comment menu, WebdriverIO suites; the round's close | D4b, E1i, E2i, E4f, E4g, E4j, E4k |

Every todo also depends on this plan (E4a). E4b starts once E2j's tree writers merge, since a tagged annotation needs the `/ParentTree` writer. E4c, E4d and E4e then run side by side, and so does E4i. The parity suite (E4g) needs only the core todos, so it can run before the interface lands.

## 9. Fixtures

Every new fixture is written by Lamina PDF's own scripts and licensed like the rest of Lamina PDF, `MIT OR Apache-2.0`; provenance goes in `fixtures/README.md`.

- **`fixtures/annotations/`** (new, E4b, from a dependency-free `tools/fixtures/annotations.py`): `every-subtype.pdf` (each subtype of the round with an appearance, and again without one), `quads.pdf` (text markup with quads in the standard's order and in Acrobat's), `threads.pdf` (Popups, replies, a group and an `/IRT` cycle), `unknown-keys.pdf` (private keys on each subtype), `flags.pdf` (Locked, LockedContents, NoZoom, NoRotate, Hidden) and `rotated.pdf` (notes and text boxes on pages turned 90°, 180° and 270°).
- **Reference files from other apps.** A text correction and a FreeText with rich text made by Acrobat, and the same marks made by Preview, are needed to check the group's order and `/RC` reading. Lamina PDF can't make them; the owner is asked for them in [issue #521](https://github.com/laudebugs/laminapdf/issues/521), and until they arrive E4c and E4e test against the pdf.js test corpus's `annotation-*.pdf` files (caret, ink, FreeText, highlight and the rest, with and without appearances), which `cargo xtask corpus fetch` already pins.
- **Signed files:** the pyHanko files in `fixtures/signed/` (approval, certified at P1, P2 and P3, a FieldMDP lock), with the usage-rights file E2i adds.
- **PDF/A-1:** `fixtures/annotations/pdfa1.pdf`, a page with a PDF/A-1b claim in its XMP and a CMYK output intent, for `pdfa-transparency` and the colour conversion (E4b, from `annotations.py`).
- **Tagged files:** the builder's `golden/tagged-report.pdf` and `golden/examples/pdf_ua_report.pdf` for tagged annotations, and `golden/examples/pdf_a_archive-2b.pdf` for the PDF/A warnings.
- **Appearance goldens:** `fixtures/golden/annots/` (E4c–E4e), written by each todo's golden example and rendered into `fixtures/render-golden/` by `cargo xtask render-golden`.

## 10. Tests

Each op gets the tests its todo lists, and the table says which tiers apply ([ADR 0018](../../decisions/0018-test-strategy.md)). "Round-trip" means Lamina PDF, pdf.js, PDFium and qpdf (`--check` clean) each reopen the saved file and report the annotation, and nothing else changed. The tagged tests in `crates/laminapdf-save/tests/roundtrip_suite.rs` flip the matrix, and `cargo xtask roundtrip` gains the ops.

| Op or function | Unit | Golden | Round-trip | Differential | Fuzz | e2e |
| --- | --- | --- | --- | --- | --- | --- |
| Views, `page_annotations` (E4b) | every fixture of `fixtures/annotations/`; unknown keys kept | — | — | annotation counts and subtypes vs pdf.js's `getAnnotations` over the corpus | new `annots` target | — |
| `UpdateAnnot`, `DeleteAnnot` (E4b) | translate, flags, locks, cascade through cycles | — | pdf.js and PDFium read the moved and remaining annotations | — | `ops_json` gains the ops | — |
| Text markup, notes, caret (E4c) | quads, icons, the group | `fixtures/golden/annots/markup.pdf` | pdf.js and PDFium draw each within the parity threshold | — | `ops_json` | — |
| Ink, shapes, lines (E4d) | every `/LE`, dashes, the cloudy border, `/Rect` growth | `annots/drawing.pdf` | as E4c | — | `ops_json` | — |
| FreeText (E4e) | `/DA` parse and write, `/RC` subset, layout in Latin, Arabic and CJK | `annots/freetext.pdf` | pdf.js and PDFium draw it; pdftotext and pdf.js extract its text | — | new `rich_text` target for `/RC` | — |
| Annotation layer (E4i) | hit-testing, coalescing, the hidden render | — | — | — | — | Playwright: drag, resize, keyboard move |
| Tools (E4f, E4j, E4k) | tool state machines (Vitest) | — | — | — | — | Playwright per tool, tagged with the row's ID; touch emulation for E4k |
| Parity suite (E4g) | — | — | — | subtype × colour × opacity × border × page rotation, rendered by pdf.js, PDFium and hayro against the goldens, nightly | — | — |

Across every op:

- **Undo:** apply then undo leaves the overlay empty and the next save byte-identical; redo writes the same bytes (ADR 0006 §3).
- **Determinism:** the same state saves to the same bytes on native and wasm32 (ADR 0007 §6), appearances included.
- **Signatures:** `would_violate` on every signed fixture in `fixtures/signed/`, asserted per op for each row of §6; E4h runs the whole table once E2i adds the usage-rights file.
- **Conformance:** veraPDF reports nothing new on the tagged and PDF/A fixtures after each op.
- **e2e:** Playwright in Chromium, Firefox and WebKit drives each tool as a user does and checks what the page shows, then saves, reopens the file in pdf.js and checks it; WebdriverIO on three systems repeats the suites in `apps/desktop/e2e/specs/` (E4h). Each test carries its capability tag.

A new fuzz target's `fixtures/regressions/<target>` folder must also join `laminapdf-model`'s regression guard list.

## 11. Decisions on the open questions

These are the round's defaults, and the owner is asked to confirm them in [issue #521](https://github.com/laudebugs/laminapdf/issues/521); the todos follow any change.

- **Rich text is a small subset, read wider than written.** E4e writes `/RC` only for text with bold or italic: an XHTML `body` of `p` elements holding `span`s styled by `font-weight` and `font-style`, the subset C03 offers, with `/DA` and `/DS` carrying the font, size, colour and alignment. It reads `b`, `i`, `span` and `p` with those properties and `font-size`, `color`, `font-family` and `text-align`, so an Acrobat-made box keeps its look when Lamina PDF lays it out. When a box is edited, styles outside the subset are dropped with `rich-text-simplified`, and the inspector says so before the first change. Lamina PDF never renders `/RC` directly: the appearance it generates is what every viewer shows. Reasoning (a recommendation): a wider subset needs a CSS engine for little gain, and C03's note already tells users that viewers which ignore `/RC` show plain text.
- **Ink keeps the captured points; smoothing is in the appearance.** `/InkList` holds each stroke's points as captured, thinned to 0.25 pt, so every reader can redraw them. With smoothing on (C02's default), the appearance draws Bézier curves through them; with it off, straight segments. PDF 2.0's `/Path` is not written, since readers before 2.0 ignore it and `/InkList` already says the same.
- **Pressure is kept as widths, in a private key.** A pressure-varying stroke stores a width per point, so a colour change can regenerate the same stroke. PDF has no standard key for it, so E4d writes it under a private key named by ISO 32000-2 Annex E's rules, and the appearance draws the varying outline. Other apps keep the key as an unknown one and show the appearance; one that redraws from `/InkList` shows a constant width.
- **One rendering path: PDFium draws what is saved.** The viewer shows committed annotations through PDFium's render of the revision E1g already keeps, and the annotation layer draws only previews during a gesture, with the original hidden for that render (§5). This replaces full-editor §E4's "overlay rendering from lamina's model until save": the two paths would never match exactly, and since E1g there is no gap between an edit and its saved bytes.
- **Popups only for notes.** A Note gets a Popup (`/Open` from the payload); other markup gets none, and pdf.js, PDFium and Acrobat show their `/Contents` without one. Acrobat also writes Popups for markup, and E5d's import keeps the ones it reads.
- **The UI splits into four todos.** E4b's viewer layer moves to E4i, FreeText editing in place to E4j, and the phone and tablet tools to E4k, so each todo stays under L and the desktop part, E4h, closes the round as E2n and E3k close theirs.
- **The plan's location:** `docs/plans/editor/`, as the plans index and E1–E3 use.
