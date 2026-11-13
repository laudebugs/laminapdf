# E7: editing existing content

**Status:** Draft

The seventh editor round changes what a PDF already draws: the text, images and vector graphics that were on the page when it was opened. Text can be retyped, restyled and replaced across the document. Images and drawings can be moved, resized, rotated, cropped, recoloured, reordered, unclipped or deleted. This plan fixes the ops, how the save planner gates them, the tests and fixtures, and the ten pull requests that build the round after this one.

[ADR 0022](../../decisions/0022-content-stream-editing.md) settled the hard part, how to edit a content stream at all. Its intermediate representation (IR), `ContentOps`, keeps every operator's source bytes, groups operators into objects with their graphics state, and writes back only what an edit changed. This round builds on it, and its own hard part is text: an edit has to work with the font that is already embedded, or say plainly when it can't.

- **Round:** [full-editor §E7](../full-editor.md#e7-editing-existing-content), phase 4, milestone M7 Content editor
- **Feature-plan todo:** [E7b](../../todos/ongoing/E7b-editing-existing-content-feature-plan.md)
- **Follows:** ADR [0022](../../decisions/0022-content-stream-editing.md) (the IR, identity, encoder, fonts, tagging), [0023](../../decisions/0023-content-stream-reading.md) (the tokenizer and interpreter), [0006](../../decisions/0006-op-log-and-undo.md) (ops and undo), [0007](../../decisions/0007-save-planner.md) (the save planner), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space)
- **Builds on:** [E6](e6-adding-content.md) (Edit mode's tools and handles in E6e, fonts for added text in E6c, system fonts in E6h), [B9](../pdf-library.md#b9-text-extraction) (extraction's glyphs and runs), [C5b](../../todos/done/C5b-find-in-document.md) (find)
- **Design:** [A04 Edit existing text inline](../../design/paper/add-edit-content.md#a04-edit-existing-text-inline), [A05 Select and transform existing objects](../../design/paper/add-edit-content.md#a05-select-and-transform-existing-objects), [A06 Search and replace](../../design/paper/add-edit-content.md#a06-search-and-replace-across-the-document), [A07 Structure and conformance warning](../../design/paper/add-edit-content.md#a07-structure-and-conformance-warning)

## 1. Where things stand

- **Reading content.** `laminapdf-content` tokenizes and interprets content with full graphics state, Form XObjects and marked content (ADR 0023). It has no editable form yet.
- **Writing content.** E2e's `PageContentWriter` wraps a page and appends streams without parsing what is there. E6's ops place new objects as Form XObjects through it and find them again by their `/PieceInfo`; they never touch original content.
- **Text.** Extraction gives every glyph its code, Unicode, position, quad, MCID and font (`PageText::glyphs`) and groups glyphs into runs (`PageText::runs`); `lines()` gives only strings. `laminapdf-text` can shape, subset and embed new fonts; it can't yet tell whether an embedded subset holds a glyph for a new character.
- **The matrix.** A.02–A.08, B.07, B.08, B.11 and B.12 have `round = "E7"`. B.01 and B.02 have `round = "E6"` with `target = "write"`, since E6 changes only what it added; this PR moves them to E7 at `round-trip`, which E7d reaches by editing existing images.

## 2. The operations

| ID | What it means | Ops (§2.1) | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| A.02 | Edit existing text | `EditText` | round-trip | E7e, E7f · E7l · E7l |
| A.03 | Delete or replace text | `EditText` | round-trip | E7e · E7l · E7l |
| A.04 | Change font family, size, colour, style | `SetTextStyle` | round-trip | E7f · E7l · E7l |
| A.05 | Bold, italic, underline, super/subscript | `SetTextStyle` | round-trip | E7f · E7l · E7l |
| A.06 | Alignment and line spacing | `SetTextStyle` | round-trip | E7f · E7l · E7l |
| A.07 | Move, resize, rotate text blocks | `EditContent` (`Transform`) | round-trip | E7d · E7k · E7k |
| A.08 | Search and replace throughout the document | `ReplaceText` | round-trip | E7g · E7g · E7g |
| B.01 | Insert, delete, replace images | `EditContent` (`Delete`, `ReplaceImage`) | round-trip | E7d · E7k · E7k |
| B.02 | Resize, rotate, crop, reposition | `EditContent` (`Transform`, `Crop`) | round-trip | E7d · E7k · E7k |
| B.07 | Edit vector paths, fills, strokes | `EditContent` (`EditPath`, `SetPaint`) | round-trip | E7h · E7h · E7h |
| B.08 | Change stacking order | `EditContent` (`Reorder`) | round-trip | E7d · E7k · E7k |
| B.11 | Clipping paths and masks | `EditContent` (`SetClip`, `RemoveMask`) | round-trip | E7h · E7h · E7h |
| B.12 | Transformations on graphical objects | `EditContent` (`Transform`) | round-trip | E7d · E7k · E7k |

### 2.1 Ops

Every op names objects by `ContentObjectRef`: the owner (`PageId`, or a Form XObject's `LaminaPdfId`), the object's ordinal among the owner's objects, and the 64-bit hash of its source bytes (ADR 0022 §2), written as 16 hex digits on the wire. A text run is a `TextRunRef`: a text object's `ContentObjectRef` and a glyph range in it. Geometry is in page space (ADR 0027), converted once through the page's `PageFrame`. Every op is class `Content` and none needs a full rewrite, except E7i's claim op, which is class `Metadata`.

| Op | Payload | Todo |
| --- | --- | --- |
| `EditContent` | `{ page: PageId, edits: Vec<ContentEdit> }`, applied in order; `ContentEdit` is `Transform { object, matrix }`, `Delete { object }`, `ReplaceImage { object, blob: BlobId }`, `Crop { object, rect: RectPt }`, `Reorder { object, to: Front \| Back \| Before(ref) \| After(ref) }` | E7d |
| `EditContent`, later variants | `EditPath { object, path: PathSpec }`, `SetPaint { object, fill, stroke, line: LineStyle }`, `SetClip { object, clip: Option<PathSpec> }`, `RemoveMask { object }` | E7h |
| `EditText` | `{ run: TextRunRef, text: String }`; an empty `text` deletes the run | E7e |
| `SetTextStyle` | `{ runs: Vec<TextRunRef>, font: Option<FontRef>, size, colour, bold, italic, underline, baseline: Normal \| Super \| Sub, align, line_spacing }`, each field optional | E7f |
| `ReplaceText` | `{ edits: Vec<{ run: TextRunRef, range, text }> }`, one transaction, a report of skipped matches | E7g |
| `RemoveConformanceClaims` | `{ pdfa: bool, pdfua: bool }`: removes `pdfaid:` and `pdfuaid:` from the XMP packet, which every other op refuses (`xmp-claim-read-only`) | E7i |

**Each op reports what it did.** `Applied::content` carries each edit's fidelity (ADR 0022 §7: exact, reflowed or substituted), the objects it created, the objects it copied on write (a shared stream or form), and for `ReplaceText` the matches it skipped with their reasons. A reference whose hash no longer matches fails with `content-object-stale`; an object that can't be edited fails with `content-not-editable` and a reason (`type3-font`, `vertical-text`, `no-unicode-map`, `inside-shared-form`, `too-deep`), a new code E7c adds with its error page.

**No ops for raw operators.** As in E6, no wire op carries content bytes: every change is typed, so nothing a caller sends can unbalance a stream.

### 2.2 Text

`EditText` follows ADR 0022 §6's order. It re-encodes with the embedded subset when every new character has a glyph in it (E7e); otherwise E7f's chain applies: a new subset of the same font when the full font is available, then a metrics-matched substitute for the missing characters only, never silently. Reflow stays on the edited line: glyphs after the edit on the same line move by the width difference, and the line's later lines don't move. A line that would overflow its text block is reported (`reflowed`, with the overflow in points), not wrapped. Paragraph reflow is a later round's (§10, question 1).

The glyph check needs the font program, so `EditText`, `SetTextStyle` and `ReplaceText` work in the text and extract modules; in the base module they fail with `appearance-needs-text`, the code E4's plan adds for the same split ([E4 §2.2](e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated)); whichever of E4 and E7e lands first adds it. The viewer opens documents in the extract module.

### 2.3 Objects

`EditContent` rewrites only the objects it names, through the IR's encoder, and copies on write any content stream or Form XObject another page or form also uses (ADR 0022 §3). Deleting an object removes its operators; a resource no operator names any more leaves the page's own copy of its resources. A replaced image is a new XObject from the blob store under a fresh name, so a shared original stays where else it is drawn.

## 3. Conformance the edits must keep

- **Tagging.** ADR 0022 §4's rules, implemented by E7i: a moved or rewritten object keeps its marked content and MCID; a deleted one clears its MCID's slot in the page's `/ParentTree` array and leaves its structure element's `/K`; an element left empty leaves its parent. Edited text keeps its `/ActualText` and `/Alt` in step: a span whose text changes gets the new text, and an image replaced in a `Figure` keeps the element and asks for new alternative text.
- **PDF/A and PDF/UA.** Every font an edit adds is embedded; a substitute is subset and embedded like any other. After each edit, `pending_conformance_warnings` lists the claims the edit breaks, as the save planner already does for metadata (ADR 0007). A07 shows them before saving: **Fix it first**, or **Save without the claim**, which applies `RemoveConformanceClaims` (§10, question 2).
- **Earlier revisions.** An incremental save keeps the old content in the previous revision, so `EditContent`'s `Delete` and `EditText`'s deletions set `still_in_earlier_revisions`. Removing text for good is redaction (E12), not editing.

## 4. Crate and package changes

- **`laminapdf-content`**: `ContentOps`, `ContentObjectRef`, hit testing and the encoder (E7c); the inline image's end offset on `Op` (ADR 0022 §1).
- **`laminapdf-text`**: line grouping with glyph ranges, which `PageText::lines()` then builds on (E7e); glyph availability in an embedded subset from its `cmap` or the inverse of its encoding and ToUnicode (E7e); the font chain (E7f).
- **`laminapdf-ops`**: the ops of §2.1 in a `content/` module, each over the IR, and the structure-tree hooks (E7i).
- **The façade and bindings.** `laminapdf::Document::content_objects(page)`, `hit_test(page, point | rect)` and `text_runs(page)`, and each op, with docs, Rust and TypeScript examples and frozen samples in `fixtures/ops/v1/`; in TypeScript, `doc.content.objects`, `hitTest`, `textRuns`, `edit`, `editText`, `setTextStyle` and `replaceText`.
- **`@laminapdf/viewer`**: worker methods for the same, and outlines for selected objects.
- **`@laminapdf/ui`**: A05's selection, handles, context menu and inspectors for existing objects (E7k); A04's inline text editor and Paragraph inspector (E7l); A06's Replace row and report (E7g); A07's strip and save check (E7i).
- **Size.** The IR and the content ops join the extract module, which the viewer opens; E7c measures the base and extract modules and asks the owner before raising a budget (ADR 0016 §6). The reader's startup scripts gain nothing: every E7 screen loads with the editor's lazy chunk.

## 5. Saving and the save constraint

Every op but `RemoveConformanceClaims` is class `Content`. DocMDP permits no content change at any level: P1 allows none, P2 form filling and signing, P3 also annotations (ISO 32000-2 §12.8.2.2). `RemoveConformanceClaims` is class `Metadata`, which DocMDP forbids too.

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update | Nothing; after a delete, the note that earlier versions stay in the file |
| `AppendOnly`: approval signatures | Signed, not certified | An incremental update after the signature, which stays valid | The status-bar note "Signed file. Saving adds your changes after the signature" |
| `AppendOnly` with a FieldMDP lock | Locked form fields | An incremental update: no E7 op touches a field or widget | The note |
| `AppendOnly` with DocMDP P1, P2 or P3 | Certified | Every op violates, so the session becomes `Conflict` | The banner naming the certifier from the first edit; on P1, from entering Edit mode |
| `RequiresRewrite` | A repaired file | A full rewrite | A neutral strip saying why |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11 | The warning strip |
| `Conflict` | Ops the constraint forbids | Save is refused; Apply anyway… invalidates the named signatures after a confirmation (`with_invalidate_signatures`) | The danger strip listing the changes, with Save a copy |

A signature field's appearance is an annotation, not page content, so no E7 op reaches it; A05 shows such an object read-only and names the signature.

## 6. Differences between the shells

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Full fonts for a new subset | `systemFonts` (E6h) | Fonts embedded whole in the file, and font files the user adds | Also fonts on the computer, found by name |
| Substitutes | | Bundled fonts (E6c's set) | Bundled fonts, then metric-compatible fonts on the computer |
| Replace shortcut | | none | ⌥⌘F on macOS, Ctrl H on Windows and Linux |

So substitution is more common on the web, and A04's notice says when a font "isn't available in the browser". No server feature: everything runs on the device.

## 7. Todos

E7d and E7e each lose their reader UI to a todo of their own, so the core can land while E6's Edit-mode tools (E6e) are still in progress, and each PR stays under ~1,500 lines.

| Todo | What | Depends on |
| --- | --- | --- |
| [E7c](../../todos/ready/E7c-content-ir.md) | `ContentOps`, identity, hit testing, the encoder, `content-object-stale`, `content-not-editable`, fuzzing | E7b |
| [E7d](../../todos/ready/E7d-select-transform-existing.md) | `EditContent` (`Transform`, `Delete`, `ReplaceImage`, `Crop`, `Reorder`), copy on write, resources dropped, the façade and SDK | E7c |
| [E7e](../../todos/ready/E7e-text-runs-and-inline-edit.md) | Line grouping, run detection, glyph availability, `EditText` with the embedded subset, the façade and SDK | E7c |
| [E7f](../../todos/ready/E7f-font-handling-and-styling.md) | The font chain, `SetTextStyle`, line-level alignment and spacing | E6c, E7e |
| [E7g](../../todos/ready/E7g-search-and-replace.md) | `ReplaceText`, A06's Replace row, Results and report | C5b, E7f, E7l |
| [E7h](../../todos/ready/E7h-vector-clipping-masks.md) | `EditPath`, `SetPaint`, `SetClip`, `RemoveMask`, A05's Edit points | E7d, E7k |
| [E7i](../../todos/ready/E7i-structure-and-conformance-maintenance.md) | The tagging hooks, `/ActualText` and `/Alt`, `RemoveConformanceClaims`, A07 | B8b, E7d, E7e |
| [E7j](../../todos/ready/E7j-edit-corpus.md) | The edit corpus, masked render diffs, the read-only list, the round's close | E7g, E7h, E7i |
| [E7k](../../todos/ready/E7k-select-transform-ui.md) | A05 in the reader: selection, handles, context menu and inspectors for existing objects | E6e, E7d |
| [E7l](../../todos/ready/E7l-inline-text-editor.md) | A04 in the reader: the inline editor, the Paragraph inspector, the font notices | E6e, E7f |

## 8. Fixtures

`tools/fixtures/content_edit.py` (no dependencies) writes `fixtures/content/edit/`, each file with what it tests in its name:

- **Objects:** a vector chart (bars, axis, labels); one content stream shared by two pages; a Form XObject drawn on two pages and twice on one; an image clipped by a path; an image with a soft mask; an inline image; a page turned 90° with a MediaBox off the origin.
- **Text:** a paragraph in an embedded TrueType subset that lacks some letters; the same with the font embedded whole; a standard 14 font; a Type 3 font; vertical text (`Identity-V`); text with `/ActualText`; a line with `TJ` kerning; text whose glyphs are drawn out of order.
- **Tagging and claims:** a tagged PDF/UA-1 page with a figure and paragraphs; a PDF/A-2b file. The signed files in `fixtures/signed/` cover approval and P1–P3 certification.

Fonts come from `fixtures/fonts/` (OFL Noto subsets). Reopened outputs of each op become byte goldens through `golden --update`.

## 9. Tests

- **Unit:** each op's apply and undo; a stale reference refused; copy on write leaving the other page unchanged; each reason a run is read-only; glyph availability on every fixture font.
- **Golden:** byte goldens of each op on the fixtures; render goldens in PDFium and hayro.
- **Round trip:** `cargo xtask roundtrip` gains the content ops; qpdf, pdf.js and PDFium open every output, and pdf.js and pdftotext read edited text back.
- **Differential:** after `EditText`, Lamina PDF's extracted text equals pdf.js's and pdftotext's on the edited pages (the B9 rule).
- **Conformance:** veraPDF on the PDF/A-2b and PDF/UA-1 fixtures after each op, and after `RemoveConformanceClaims`.
- **Fuzz:** a `content_edit` target (random edits on corpus pages, re-read every time), and the ops join `ops_json`.
- **Edit corpus (E7j):** §10, question 3.
- **End to end:** Playwright drives A04, A05, A06 and A07 in the reader; WebdriverIO checks system-font subsets on each desktop OS.

## 10. Decisions on the open questions

1. **Line-level editing, or paragraph reflow?** ([full-editor §9](../full-editor.md#9-open-questions), question 4.) Line-level for this round (§2.2): a later glyph on the same line moves, and nothing wraps. Paragraph reflow means finding a paragraph's lines, its measure and its alignment from positioned glyphs, which E7e's line grouping makes possible later. **Decided by the owner on 2026-11-13.**
2. **Saving a claim the edits broke** ([the area's open question 1](../../design/paper/add-edit-content.md#open-questions)). The reader never saves a claim it has just found false: Fix it first, or Save without the claim. No "keep the claim" option in the reader; the library keeps ADR 0007's rule that conformance warnings never block, so a caller can still save. A removed claim doesn't come back on its own: a file that passes again can be claimed by the Accessibility tool's checks (X06/X07). **Decided by the owner on 2026-11-13.**
3. **The edit corpus and "identical except edited regions".** The corpus is a list of files from the pinned external corpus (pdf.js, PDFBox, PDFium and qpdf; their licences in `fixtures/corpus-manifest.toml`), never committed, with an edit script per file as an ADR 0006 op log, committed. Each page renders before and after the script in PDFium and hayro at 96 dpi. Outside the old and new boxes of the edited objects, grown by 2 px for anti-aliasing, the two renders must match pixel for pixel. Files Lamina PDF refuses to edit are listed with their `content-not-editable` reasons. E7j owns the list, in the nightly lane.
4. **Synthetic bold and italic** ([the area's open question 2](../../design/paper/add-edit-content.md#open-questions)): whether the page shows a badge is a design question for E7l, with the design owner.
