# 0022 — Content-stream editing: an IR with object identity and a span-preserving encoder

- Status: Proposed
- Date: 2026-11-13

## Context

The editor's content rounds change what a page draws. E7 moves, deletes and replaces existing objects and edits existing text; E12 removes content for redaction ([full-editor plan §E7](../plans/full-editor.md#e7-editing-existing-content), [§6](../plans/full-editor.md#6-design-summary), step 3). Until now `laminapdf-content` could only read content ([ADR 0023](0023-content-stream-reading.md)) and append to it without parsing what was there (E2e). Editing needs more:

- **Something to select.** A content stream is a postfix program (ISO 32000-2 §7.8.2), not a list of objects. "This paragraph" or "that logo" is a range of operators whose meaning depends on the graphics state that came before it ([02 §2](../research/pdf/02-imaging-model.md#2-content-streams)).
- **Names that last.** An edit names its object in the op log ([ADR 0006](0006-op-log-and-undo.md)), and the reader replays the log into a restarted worker. The name has to find the same object again, or fail plainly.
- **A writer that doesn't disturb the rest.** Whatever an edit leaves alone should stay as it was: for diffs, for signatures on other revisions, and because a reader that tolerates bad content can't promise that writing it again keeps its meaning.
- **Text that can change.** PDF stores positioned glyphs in a font's own encoding, not text ([03 §1](../research/pdf/03-text-and-fonts.md#1-the-core-insight-pdf-stores-positioned-glyphs-not-text)). Editing a word means finding the run, checking that its font can draw the new characters, and positioning what follows ([03 §11.2](../research/pdf/03-text-and-fonts.md#112-concrete-requirements), "Editing"). Editing existing text is rank 7 on the [difficulty map](../research/pdf/06-feature-inventory.md#7-difficulty-map), the hardest editor feature.
- **Removal that leaks nothing.** E12 must delete glyphs and pixels, and must not leave the removed text's widths in the positions of what remains ([04 §10.1](../research/pdf/04-interactive-security.md#101-drawing-a-black-box-is-not-redaction)).

The E7a spike ([`spikes/content-edit/`](../../spikes/content-edit/README.md)) measured the two encoders this ADR chooses between. It ran on the pinned external corpus (pdf.js, PDFBox, PDFium and qpdf files: the 2,058 of 2,161 PDFs that open, 5,967 pages, 26.0 MB of decoded content, 1.84 million operators) on an Apple M1 Pro, in a release build:

| Measure | Full re-serialise | Span-preserving |
| --- | --- | --- |
| Pages whose operators read back identical | 5,499 of 5,967 (92.2 %) | — |
| Pages equal within 10⁻⁶ per number | 5,946 of 5,967 (99.6 %) | — |
| Pages that read back as the edit meant (one object moved) | — | 5,643 of 5,655 (99.8 %) |
| Content bytes copied from the source after that edit | 0 % | 84.3 % |
| Size against the original | 106.5 % | the original, plus the moved object's change |
| Tokenizing and grouping into objects | 67 MB/s; the slowest page 73 ms | the same |

Re-serialising changed 468 pages: 447 only by rounding numbers, and 21 through the spike's own inline-image guess and one case it didn't explain. Lamina PDF's writer rounds reals to six decimals (`0.8509804` becomes `0.85098`) and writes `2.0` as `2`. Neither rounding is visible on the page, but both change the bytes, so a page nobody edited would still differ from its source. The 84.3 % is a lower bound: the spike always moved the page's first text object, which on many pages is most of the page.

## Decision

Edit content through a per-page IR of operators grouped into objects, and encode by copying every span an edit didn't touch.

### 1. `ContentOps`: operators with spans, grouped into objects

`laminapdf-content` gains `ContentOps`, the editable intermediate representation (IR) that ADR 0023 §2 reserved. It is built from `ContentIter` for one content owner: a page (its `/Contents` streams, which §7.8.2 reads as one program) or a Form XObject.

- **Operators keep their source.** Each operator records its operands, its keyword and its span: which `/Contents` part it came from, and the bytes from the end of the previous operator through its own keyword. An inline image's span runs from `BI` through `EI`. `Op` gains that end offset: the spike guessed it from the image data, and the guess caused 30 of its 33 mismatches, so the tokenizer must report where it stopped.
- **Objects are operator ranges.** A text object (`BT … ET`), a path (construction through its painting operator, clipping included), an image (`Do` of an image XObject, or an inline image), a form (`Do` of a Form XObject) and a shading (`sh`). Marked-content sequences (`BMC`/`BDC … EMC`) are containers around objects, not objects. On the corpus, 1.84 million operators grouped into 290,449 objects, 48,202 of them text objects.
- **Each object carries its state.** The interpreter's `GraphicsState` at the object's first operator is recorded with it (ADR 0023's visitor fills it), with its bounding box in page space ([ADR 0027](0027-coordinates-units-and-page-indexing.md)). An object can then be moved, deleted or written again without the `q`, `cm` and colour operators before it.
- **Text objects go down to glyphs.** Each show operator lists its glyphs with their positions, from `laminapdf-text`'s extraction. E7e edits and E12 removes at that level; selection works on whole objects.
- **Built per page, on demand.** The IR exists only for pages being edited, within `max_content_operators` (ADR 0003), and is dropped when the page's edits are saved or undone.

The IR goes no finer than objects. A finer IR would make every encode and hit test slower, and a coarser one (whole streams) would leave E12 nothing to remove. Glyphs inside text objects are the one place where finer detail pays.

### 2. Identity

- **Within a session,** `ContentObjectId` is allocated when the IR is built, and edits update the IR instead of reparsing it, so an ID stays valid across edits to its page.
- **On the wire,** an op names an object by its owner (a `PageId`, or the Form XObject's reference), its ordinal among the owner's objects, and a 64-bit hash of its source bytes. Replaying a log into a reopened document rebuilds the IR, finds the ordinal and checks the hash. A mismatch fails with a new error, `content-object-stale`, rather than editing the wrong object.
- **After a save,** IDs are recomputed from the saved content. An object an edit wrote again gets a new hash, and each undo entry in the op log records the hash its edit wrote, so undo can find the object again.

### 3. Encoder: copy what didn't change

`ContentOps::encode` writes each operator from its source span, byte for byte, unless an edit changed it. Changed and new operators are written canonically by `laminapdf-syntax`'s writer, so its six-decimal rounding touches only them. A moved or rewritten object is wrapped in `q … Q` with its recorded state, so it draws as it did wherever it lands.

- **New streams, never changed ones.** The encoded content goes into new stream objects in the overlay (Flate). Only the `/Contents` parts that hold changed spans are replaced; an object spanning two parts merges them into one new stream.
- **Copy on write for what is shared.** A content stream or Form XObject used by more than one page or form is copied for the owner being edited, and the others keep the original. Editing inside a shared form copies the form and gives the copy a fresh name in this owner's resources, as E2e's `add_xobject` names new ones.
- **The save planner sees one class.** Content edits are `ModificationClass::Content`. They save incrementally on a `Free` document; a certification signature at any DocMDP level forbids them ([ADR 0007](0007-save-planner.md)), which makes the save constraint's kind `Conflict`.

### 4. Finding text runs: marked content first, layout second

- **In a tagged document,** a run is the glyphs of one marked-content sequence (one MCID) within one text object, in content order. The structure tree already says which text belongs together and in what order.
- **Otherwise,** runs and lines come from extraction's own grouping (`PageText::runs` and `lines()`): one font and size on one baseline, joined into lines by gap. One home per concept ([maintainability](../guidelines/maintainability.md)): the editor doesn't keep a second line finder.
- **A run that can't be found** reliably (rotated glyphs within one run, a show operator that writes glyphs out of order) is read-only, and the reason is reported.

### 5. Fonts: reuse, then subset again, then substitute

For each edit, in order:

1. **Reuse the embedded font** when every new character maps to a glyph in it and its encoding can address that glyph (the font's `cmap`, or the inverse of its encoding and ToUnicode).
2. **Embed a new subset** when the full font is available: a system font on the desktop, or a font the user supplies. It goes in as a new font resource; the existing font object, which other text may use, never changes.
3. **Substitute** a bundled font with close metrics, and label the edit as substituted.

A missing glyph is an error (`missing-glyph`, as the builder raises it), never a silent `.notdef`. Type 3 fonts, vertical writing (`Identity-V`), and fonts with neither ToUnicode nor a usable `cmap` make their runs read-only.

### 6. Best effort, labelled

Each content edit reports its fidelity: exact (the same font, the glyphs placed as they were), reflowed (the following glyphs on the line moved), or substituted (another font). The UI shows that label (E7e), and the editor says plainly that editing existing content is best effort ([§E7](../plans/full-editor.md#e7-editing-existing-content)). What can't be edited stays selectable and readable, with its reason: a read-only run, an object inside a form too deep for `max_xobject_depth`, a page whose content fails to tokenize.

### 7. What E12 builds on

E12 removes glyphs from the IR's show operators and writes the remainder canonically with absolute positions, so no removed width survives in a `TJ` adjustment (E12a's rule). It removes or clips paths, and finds images (XObjects, inline images, shared images copied per page) through the IR's image objects. Redaction then saves as a full rewrite with every earlier revision dropped, so span preservation matters there only for the content it keeps.

## Consequences

- An untouched operator never changes, so a one-object edit leaves most of a page's bytes as they were (84 % across the corpus, a lower bound since the spike moved each page's first text object), and a diff of the edited revision shows the edit.
- Numbers an edit writes again are rounded to six decimals. That changes values by at most 5 × 10⁻⁷, below anything a renderer shows; E7j's edit corpus checks that edited pages render the same outside the edited region.
- `Op` gains an inline image's end offset; E7c adds it with the IR.
- On a certified document, content edits make the save constraint's kind `Conflict`, as any content change does today.
- The IR costs a tokenize per edited page (67 MB/s; 73 ms for the slowest corpus page). Pages that aren't edited cost nothing.
- Twelve edited pages in the spike didn't read back as meant: ten are the spike's inline-image guess, which the tokenizer's end offset replaces, and two are text objects that round-trip correctly on their own. Of the 21 pages a re-serialise changed by more than 10⁻⁶, 20 are the same guess and one gained operators for a reason the spike didn't find. E7c's tests and the `content` fuzz target must cover both unexplained cases before the encoder ships.

## Alternatives considered

- **Re-serialise every operator (a display list).** Simpler, since no spans are kept. It changed 7.5 % of the corpus's pages by rounding numbers alone, made content 6.5 % larger, drops comments and odd spacing, and rewrites every stream of an edited page, shared ones included.
- **Patch tokens without an IR.** Keeps bytes like the chosen encoder, but without objects, state or glyph positions nothing can be selected, moved safely or redacted.
- **Cover and add.** Some editors paint a box over the old content and draw the new on top. It isn't editing, extraction still finds the old text, and for redaction it is the failure [04 §10.1](../research/pdf/04-interactive-security.md#101-drawing-a-black-box-is-not-redaction) describes.
- **Glyph-level objects everywhere.** Makes selection and encoding slower on every page for detail only text editing and redaction need; §1 keeps glyphs inside text objects instead.
