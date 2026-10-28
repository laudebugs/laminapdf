# 06 — Feature Inventory: Everything a PDF Platform Can Do

> Part of the [PDF research series](README.md). Chapters 01–05 explain *how PDF works*; this chapter lists *what users and developers want to do with PDFs*, and maps each operation onto the PDF mechanism underneath it, its difficulty, and the chapter that explains it.
>
> Origin: product notes from the project owner (feature inventory modelled on what a full desktop editor such as Adobe Acrobat Pro exposes — Adobe advertises 70+ tools in Acrobat Pro), expanded with the technical mapping. Difficulty ratings and mechanism notes are the research team's assessment, not spec facts.

## Contents

1. [A PDF is more than text and images](#1-a-pdf-is-more-than-text-and-images)
2. [Feature groups at a glance](#2-feature-groups-at-a-glance)
3. [Full operation inventory with mechanisms](#3-full-operation-inventory-with-mechanisms)
4. [What a PDF actually contains (object model recap)](#4-what-a-pdf-actually-contains)
5. [Types of PDF documents](#5-types-of-pdf-documents)
6. [Operations that look alike but are not](#6-operations-that-look-alike-but-are-not)
7. [Difficulty map — where the hard problems are](#7-difficulty-map)
8. [Reference architecture for an Acrobat-class editor](#8-reference-architecture)
9. [Suggested phasing (input for planning)](#9-suggested-phasing)
10. [Implications for lamina](#10-implications-for-lamina)

---

## 1. A PDF is more than text and images

A PDF can carry editable text, vector graphics, raster images, interactive forms, digital signatures, annotations and review threads, optional-content layers, embedded files, JavaScript, rich media and 3D, semantic structure for accessibility, and encryption. ISO 32000-2 (PDF 2.0) standardises all of it; subsets such as PDF/A (archive), PDF/X (print) and PDF/UA (accessibility) constrain it for specific workflows (→ [01 §1](01-file-structure.md)).

A "platform for anything and everything PDF" therefore spans at least four very different kinds of software:

| Kind | Examples | Core competence |
| --- | --- | --- |
| **Generator** | report engines, invoice/label generators, HTML→PDF | layout, fonts, writing valid files |
| **Processor** | merge/split, compress, encrypt, sign, convert, OCR | object-graph surgery, filters, crypto |
| **Reader / extractor** | search indexing, data extraction, accessibility checkers | tolerant parsing, text reconstruction |
| **Viewer / editor** | Acrobat, Preview, pdf.js-based apps | rendering, hit-testing, content editing, UI |

---

## 2. Feature groups at a glance

13 feature groups, ~180 operations.

| # | Group | Ops | Main PDF mechanism | Chapters |
| --- | --- | --- | --- | --- |
| A | Text & typography | 12 | content-stream text operators, fonts, ToUnicode | 03 |
| B | Images & vector graphics | 12 | paths, image/Form XObjects, graphics state | 02 |
| C | Page organisation & geometry | 16 | page tree, page boxes, `/Rotate`, Form XObjects | 01, 05 |
| D | Comments & annotations | 16 | annotation dictionaries + appearance streams, XFDF | 04 |
| E | Interactive forms | 17 | AcroForm field tree, widgets, actions, JS | 04 |
| F | Signatures & approval | 11 | signature fields, CMS/PAdES, DocMDP, incremental updates | 04 |
| G | Security & privacy | 13 | security handlers, crypt filters, redaction | 04 |
| H | Conversion & extraction | 15 | rendering, text reconstruction, OCR, layout analysis | 03, 05 |
| I | Navigation & structure | 14 | outlines, destinations, actions, structure tree, OCGs | 02, 03, 04 |
| J | Metadata & attachments | 12 | Info dict, XMP, EmbeddedFiles, `/AF` | 01, 04 |
| K | Printing & optimisation | 15 | filters, object streams, subsetting, colour management | 01, 02, 05 |
| L | Accessibility | 12 | tagged PDF, structure tree, PDF/UA | 03 |
| M | Advanced interactions | 14 | JS actions, OCGs, rich media, 3D, measurement, revisions | 02, 04 |

---

## 3. Full operation inventory with mechanisms

Difficulty: **1** = straightforward object manipulation · **2** = needs a solid subsystem (fonts, rendering, crypto) · **3** = hard, heuristic or open-ended; even mature products get it wrong.

### A. Text & typography

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Add text anywhere on a page | Append a content stream (or wrap existing in `q … Q`) with `BT /F1 12 Tf x y Td (…) Tj ET`; embed/subset the font; add ToUnicode | 2 |
| Edit existing text | Locate the `Tj`/`TJ` ops that produced the glyphs, re-encode with the *same* font (may lack the needed glyphs — subset!), re-flow manually; there are no paragraphs in PDF | 3 |
| Delete or replace text | Remove/rewrite show-text ops; replacement hits the same subset-glyph problem | 3 |
| Change font family/size/colour/style | Change `Tf`, colour ops, possibly swap font resource; metrics change ⇒ positions of following glyphs change | 3 |
| Bold, italic, underline, super/subscript | Different font face (or synthetic: stroke+fill `Tr 2`, skew in `Tm`); underline is a separate drawn path; super/sub via `Ts` rise | 2 |
| Alignment & line spacing | Pure layout — PDF only stores final positions; requires a layout engine | 2 |
| Move/resize/rotate text blocks | Modify `Tm`/`cm`; requires identifying what a "block" is (heuristic) | 2–3 |
| Search & replace throughout document | Search = extraction (3); replace = edit-existing-text (3) | 3 |
| Add hyperlinks to text | Link annotation with `/Rect` over the glyph bboxes + `/URI` or `/GoTo` action | 1 |
| Extract text with position & formatting | Run the text interpreter, compute glyph quads via text rendering matrix, map via ToUnicode, group into words/lines/blocks | 2–3 |
| Convert outlined or scanned text via OCR | Rasterise → OCR → add invisible text (`3 Tr`) under the image | 2 |
| Insert special characters & symbols | Requires a font containing the glyph + correct ToUnicode; font fallback | 2 |

### B. Images & vector graphics

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Insert, delete, replace images | Image XObject + `cm … Do`; delete = remove the `Do` (and the resource if unused); replace = swap the XObject stream | 1 |
| Resize, rotate, crop, reposition | Change the `cm` matrix before `Do`; crop = clipping path (`re W n`) or re-encode pixels | 1–2 |
| Extract original embedded images | Read XObject stream; DCT/JPX can be passed through as `.jpg`/`.jp2`; others must be decoded and re-encoded, honouring `/Decode`, masks, colour spaces | 2 |
| Adjust transparency/opacity | ExtGState with `/CA` `/ca`, or soft mask | 1 |
| Draw lines, rectangles, circles, polygons | Path ops `m l c re h` + `S`/`f`/`B`; circles via 4 Béziers (κ ≈ 0.5523) | 1 |
| Arrows, paths, freehand drawing | Paths (content) or Line/Polyline/Ink annotations (markup) | 1 |
| Edit vector paths, fills, strokes | Parse content stream into an editable display list, hit-test, re-serialise | 2–3 |
| Change stacking order | Reorder ops in the content stream (painter's model); requires object identification | 2 |
| Add logos, icons, stamps | Form XObject placed via `Do`, or Stamp annotation | 1 |
| Background images & watermarks | Prepend (background) or append (watermark) a Form XObject; optionally in an OCG; Watermark annotation | 1 |
| Clipping paths & masks | `W`/`W*`, image masks, soft masks in ExtGState | 2 |
| Transformations on graphical objects | Matrix concatenation (`cm`); correct ordering math | 1–2 |

### C. Page organisation & geometry

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Insert blank pages | New page dict in the page tree, `/MediaBox`, empty contents | 1 |
| Insert pages from other PDFs | Deep-copy the page and all reachable objects, renumber, dedupe resources; drop/merge annotations' `/P`, struct-tree links | 2 |
| Delete & duplicate pages | Edit `/Kids` + `/Count`; clean dangling refs (outlines, links, struct tree, named dests) | 1–2 |
| Reorder pages | Rearrange `/Kids` | 1 |
| Extract selected pages | Same as insert-from-other-PDF into a new doc | 2 |
| Merge multiple documents | Copy page trees; merging outlines, AcroForm fields (name clashes!), struct trees, OCGs, named dests is the hard part | 2 |
| Split by count, ranges, bookmarks | Extract; bookmark → page mapping via destinations | 2 |
| Rotate pages 90/180/270 | `/Rotate` key (inheritable) | 1 |
| Crop display area | Change `/CropBox` — **content outside remains in the file** | 1 |
| Resize page dimensions | Change `/MediaBox` + scale content via a wrapping `cm` | 1–2 |
| Modify margins & page boxes | MediaBox/CropBox/BleedBox/TrimBox/ArtBox | 1 |
| Portrait ↔ landscape | `/Rotate` or MediaBox swap + transform | 1 |
| Reverse page order | Reverse `/Kids` | 1 |
| Labels, page numbers, headers/footers | `/PageLabels` number tree (logical labels); headers/footers are stamped content, ideally marked as Artifact | 1–2 |
| Booklet / N-up layouts | Each source page → Form XObject; place scaled/rotated on a new sheet (imposition) | 2 |
| Replace pages, preserving the rest | Swap page objects while keeping links/bookmarks pointing at the slot | 2 |

### D. Comments & annotations

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Highlight / underline / strike through | Text-markup annotations with `/QuadPoints` from glyph boxes + appearance stream | 1–2 |
| Sticky notes & text comments | Text annotation (+ Popup) | 1 |
| Text-correction annotations | Caret + StrikeOut, grouped via `/IRT` `/RT /Group` | 1 |
| Freehand ink | Ink annotation, `/InkList` | 1 |
| Shapes, lines, arrows | Square, Circle, Line (`/LE` arrowheads), Polygon, PolyLine | 1 |
| Callouts & text boxes | FreeText annotation with `/CL` callout line, `/DA`, `/RC` rich text | 2 |
| Custom stamps | Stamp annotation with custom appearance stream | 1 |
| Attach files as annotations | FileAttachment annotation with embedded file spec | 1 |
| Audio annotations | Sound annotation (deprecated in PDF 2.0 → RichMedia/Screen) | 1 |
| Replies to comments | Annotation with `/IRT` pointing to parent | 1 |
| Status & review properties | State-model replies (`/StateModel /Review`, `/State /Accepted`) | 1 |
| Filter & search comments | Read annotation dicts across pages | 1 |
| Import & export annotation data | XFDF/FDF serialisation | 1–2 |
| Flatten annotations into content | Draw each appearance stream as a Form XObject into the page content with the annotation's matrix; remove the annot | 2 |
| Delete or summarise annotations | Remove from `/Annots`; summary = render a report | 1 |
| (cross-cutting) Generate appearance streams | Required for consistent rendering across viewers; non-trivial for FreeText/rich text | 2 |

### E. Interactive forms

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Text fields (single/multi-line) | AcroForm `/FT /Tx` field + Widget; `/Ff` multiline flag | 1 |
| Checkboxes & radio buttons | `/FT /Btn` with on/off appearance states (`/AP /N /Yes`, `/Off`); radio = parent with kids | 1–2 |
| Dropdowns & list boxes | `/FT /Ch` with `/Opt`, combo flag | 1 |
| Push buttons | `/FT /Btn` push flag + action | 1 |
| Date/formatted input | `/AA` Format/Keystroke JavaScript (AFDate_FormatEx…) — only works in JS-capable viewers | 2 |
| Required & read-only | `/Ff` bits | 1 |
| Input validation | `/AA /V` JavaScript | 2 |
| Automatic calculations | `/AA /C` scripts + `/CO` calculation order | 2 |
| Tab order | Page `/Tabs` (`/R` `/C` `/S`) | 1 |
| Conditional behaviour | JavaScript show/hide actions | 2 |
| Import & export form data | FDF/XFDF/JSON ↔ `/V` values | 1 |
| Populate fields programmatically | Set `/V` **and regenerate appearance streams** (font, `/DA`, comb, quadding, auto-size) — the hard part | 2–3 |
| Reset form fields | ResetForm action or set `/V` to `/DV` | 1 |
| Flatten forms | Stamp each widget's appearance into page content; remove fields | 2 |
| Signature fields | `/FT /Sig` field + widget, optional `/Lock` and `/SV` | 1 |
| Submit to external endpoints | SubmitForm action (viewer-dependent) | 1 |
| (cross-cutting) XFA forms | Dynamic XFA is XML-rendered, deprecated in PDF 2.0; support = read-only fallback or convert | 3 |

### F. Signatures & approval

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Handwritten signature image | Just an image/stamp — no cryptographic meaning | 1 |
| Draw/type e-signature | Ink/stamp + often an external audit trail; not a PDF digital signature unless combined with one | 1 |
| Certificate-based digital signatures | `/ByteRange` + `/Contents` placeholder, CMS SignedData detached, PAdES profile, written as an incremental update | 2 |
| Verify signatures | Hash byte ranges, verify CMS, build certificate chain, check revocation (OCSP/CRL), **check what changed in later incremental updates** | 3 |
| Certify a document | DocMDP transform with `/P 1–3` on the first signature | 2 |
| Trusted timestamps | RFC 3161 token in the CMS unsigned attrs, or document timestamp (`ETSI.RFC3161`) | 2 |
| Inspect signer identity & certificates | Parse X.509 from CMS | 1 |
| Lock fields after signing | `/Lock` dict / FieldMDP | 2 |
| Allowed changes after certification | DocMDP `/P` semantics, enforced by the validator's diff analysis | 3 |
| Signature status & modification warnings | Revision diffing + the attacks catalogue (USF, ISA, SWA, Shadow) | 3 |
| Signing workflows via external services | API integration; out of PDF scope | — |

### G. Security & privacy

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Document-open password | Standard security handler, user password, AES-256 (R6) | 2 |
| Encrypt a PDF | Encrypt all strings & streams with object keys / crypt filters | 2 |
| Permission passwords | Owner password + `/P` bits — **advisory only**, honoured by compliant viewers | 1 |
| Restrict print/copy/edit | `/P` bits (advisory) | 1 |
| Remove security with authorisation | Decrypt with the password and rewrite unencrypted | 2 |
| Permanent redaction | Remove glyphs/ops intersecting the area, rewrite text runs, burn image pixels, scrub annots/forms/metadata, full rewrite (no incremental history) | 3 |
| Search sensitive content to redact | Extraction + regex/NER, then redaction | 2–3 |
| Remove hidden metadata & comments | Strip Info/XMP, annotations, private app data (`/PieceInfo`), thumbnails | 1–2 |
| Sanitise embedded content | CDR: drop JS, Launch, embedded files, external refs; rebuild the file | 2 |
| Remove attachments & scripts | EmbeddedFiles name tree, `/AF`, FileAttachment annots, `/Names /JavaScript`, `/OpenAction`, `/AA` | 1 |
| Inspect encryption | Read `/Encrypt` dict | 1 |
| Certificate-based security | Public-key security handler (`/Adobe.PubSec`), recipients | 2 |
| Remove hidden layers/information | Drop hidden OCG content, off-page content, incremental history | 2 |

### H. Conversion & extraction

| Operation | How it works | Diff. |
| --- | --- | --- |
| Word / Excel / PowerPoint → PDF | Needs an office layout engine (LibreOffice headless, or vendor APIs) — not a PDF problem | 2 (wrap) / 3 (build) |
| Images → PDF | One page per image, JPEG passthrough, PNG → Flate + SMask | 1 |
| HTML / webpages → PDF | Headless Chromium or WeasyPrint/Prince; CSS Paged Media | 1 (wrap) / 3 (build) |
| PDF → Word | Reconstruct paragraphs, styles, tables from positioned glyphs — **layout analysis** | 3 |
| PDF tables → Excel | Table detection (ruling lines + alignment) or ML models | 3 |
| Pages → PNG/JPEG/TIFF | Full renderer | 2 (wrap) / 3 (build) |
| Extract text & images | Text interpreter + XObject walk | 2 |
| Extract hyperlinks & bookmarks | Link annots + `/Outlines` | 1 |
| Extract attachments | EmbeddedFiles + FileAttachment annots | 1 |
| Export pages/ranges | Page extraction | 1–2 |
| Render pages to images | Renderer | 2–3 |
| Convert to PDF/A, PDF/X, PDF/UA | Embed all fonts, output intent, XMP identification, remove forbidden features (JS, encryption, transparency for PDF/A-1), tags for UA; validate with veraPDF | 2–3 |
| Scanned → searchable PDF | OCR + invisible text layer (OCRmyPDF pattern) | 2 |

### I. Navigation & document structure

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Create/edit bookmarks; nested trees | `/Outlines` doubly linked tree (`/First /Last /Next /Prev /Parent /Count`) | 1 |
| Internal page links | Link annotation + `/Dest` or GoTo action | 1 |
| External hyperlinks | Link annotation + URI action | 1 |
| Named destinations | `/Dests` name tree | 1 |
| Clickable table of contents | Generated content + links (+ outline) | 1–2 |
| Initial view & zoom; page display mode | `/OpenAction`, `/PageMode`, `/PageLayout`, `/ViewerPreferences` | 1 |
| Define reading order; logical structure tags | Structure tree order + MCIDs | 2–3 |
| Optional content groups (layers) | `/OCProperties`, `BDC /OC` marked content | 2 |
| Article threads | `/Threads` beads | 1 |
| Page labels | `/PageLabels` number tree | 1 |
| Outline & navigation preferences | ViewerPreferences | 1 |

### J. Metadata & attachments

| Operation | How it works in PDF | Diff. |
| --- | --- | --- |
| Title, author, subject, keywords | XMP (dc:, pdf:) + Info dict kept in sync | 1 |
| Creation/modification dates | `CreationDate`/`ModDate` + xmp:CreateDate/ModifyDate | 1 |
| Producer & creator | Info `/Producer` `/Creator`, xmp:CreatorTool | 1 |
| Manage XMP | Parse/serialise RDF/XML packet | 1–2 |
| Attach / extract / delete embedded files | EmbeddedFiles name tree, file specs, `/AF` | 1 |
| Portfolios / collections | `/Collection` dict + navigator | 2 |
| Inspect version & conformance | Header, catalog `/Version`, XMP `pdfaid:`/`pdfuaid:` | 1 |
| Identify fonts & colour profiles | Walk resources | 1 |
| Inspect internal objects, streams, references | A COS-level object browser (like `qpdf --json`, `mutool show`) | 1 |
| Remove selected metadata | Edit both Info and XMP | 1 |

### K. Printing & optimisation

| Operation | How it works | Diff. |
| --- | --- | --- |
| Compress / reduce size | Object streams + xref streams, Flate everything, dedupe, image recompression, font subsetting | 2 |
| Downsample & recompress images | Decode → resample → re-encode (JPEG/JPX), keep masks | 2 |
| Embed or subset fonts | Needs the font file; subsetting with glyph closure | 2 |
| Remove unused objects | Garbage-collect from `/Root` on full rewrite | 1 |
| Deduplicate shared resources | Hash streams/dicts, rewrite refs | 2 |
| Linearise for fast web view | Reorder objects + hint tables | 2 |
| Flatten transparency | Plane-map regions and rasterise/split — very hard to do well | 3 |
| Convert colour spaces | ICC-based transforms (LittleCMS), rewrite colour ops and images | 2–3 |
| ICC profiles & output intents | `/OutputIntents` in catalog | 1 |
| Bleed, trim, crop | Page boxes | 1 |
| Print-ready PDFs / preflight | Rule-based checks (fonts embedded, colour spaces, resolution, overprint) | 2 |
| Validate PDF/A or PDF/X | veraPDF (PDF/A, UA); PDF/X needs commercial or custom preflight | 2 (wrap) |
| Imposition & booklet | Form XObjects on sheets | 2 |
| Spot colours & overprint | Separation/DeviceN inspection, overprint preview rendering | 2–3 |

### L. Accessibility

| Operation | How it works | Diff. |
| --- | --- | --- |
| Add structure tags (headings, paragraphs, lists, tables) | Structure tree + MCID marked content + ParentTree | 2 (when generating) / 3 (retrofitting) |
| Logical reading order | Structure element order | 2–3 |
| Alt text for images | `/Alt` on Figure elements | 1 |
| Document language | Catalog `/Lang`, per-element `/Lang` | 1 |
| Mark decorative artifacts | `/Artifact BMC … EMC` | 1–2 |
| Accessible form labels | `/TU` tooltip on fields + Form struct elements | 1 |
| Accessible link descriptions | `/Contents` on Link annots + Link struct elements | 1 |
| Check missing fonts / Unicode mappings | Walk fonts, verify ToUnicode coverage | 1–2 |
| Run accessibility validation | veraPDF (machine-checkable PDF/UA), Matterhorn | 2 (wrap) |
| Fix tagging & structure issues | Auto-tagging needs layout analysis/ML | 3 |
| PDF/UA-compliant output | All of the above + XMP `pdfuaid` | 2 |

### M. Advanced interactions

| Operation | How it works | Diff. |
| --- | --- | --- |
| Add/edit JavaScript actions | `/JS` in actions, `/Names /JavaScript`; execution needs a JS sandbox + Acrobat API shim | 1 (write) / 3 (execute) |
| Interactive buttons | Push-button widgets + actions | 1 |
| Event-triggered actions | `/AA` dictionaries on doc, page, field | 1 |
| Manage/show/hide layers | OCGs, `/OCProperties /D`, SetOCGState | 2 |
| Embed audio & video | RichMedia / Screen + Rendition (viewer support is poor) | 2 |
| Embed 3D models | 3D annotations (U3D/PRC) — very poor viewer support outside Acrobat | 3 |
| Measurement tools & scales | Viewport `/VP` + Measure dictionaries | 2 |
| Geospatial coordinates | GeoPDF / `/Measure /Subtype /GEO` | 2 |
| Associated files & structured metadata | `/AF` arrays (e.g. Factur-X/ZUGFeRD XML invoice) | 1 |
| Document open actions | `/OpenAction` | 1 |
| Rich-media annotations | RichMedia annotations | 2 |
| Inspect incremental updates & revisions | Walk `/Prev` chain; reconstruct each revision | 2 |
| Compare versions visually & structurally | Render-and-diff + text/structure diff | 2–3 |

---

## 4. What a PDF actually contains

Conceptual object model (not byte layout — see [01](01-file-structure.md) for that):

```
PDF document
├── Trailer → Catalog (/Root)              ── cross-reference data locates every object
│   ├── Pages (page tree)
│   │   └── Page ── /Contents (content streams: drawing instructions)
│   │            ── /Resources (fonts, images, Form XObjects, ExtGState, colour spaces, patterns)
│   │            ── /Annots (links, comments, form widgets, …)
│   ├── /AcroForm         interactive form fields
│   ├── /Outlines         bookmarks
│   ├── /Names            named destinations, embedded files, JavaScript
│   ├── /OCProperties     layers
│   ├── /StructTreeRoot   semantic structure (accessibility)
│   ├── /Metadata         XMP stream
│   └── /AF               associated files
├── /Encrypt              security handler configuration
└── /Info                 legacy metadata
(signature dictionaries hang off /Sig fields in /AcroForm)
```

| Object / concept | Purpose | Chapter |
| --- | --- | --- |
| Catalog | Root of the document | 01 |
| Page tree | Hierarchy of pages, inherited attributes | 01 |
| Content streams | Drawing instructions for text and graphics | 02, 03 |
| Font objects | Font programs, encodings, character mappings | 03 |
| Image XObjects | Reusable embedded raster images | 02 |
| Form XObjects | Reusable graphics groups | 02 |
| Annotations | Comments, links, widgets, other page interactions | 04 |
| AcroForm | Form fields and properties | 04 |
| Outlines | Bookmark navigation | 04 |
| Optional content groups | Layers and visibility | 02 |
| Structure tree | Semantic structure for accessibility | 03 |
| EmbeddedFiles / `/AF` | Attachments and associated data | 04 |
| Metadata streams | XMP metadata | 04 |
| Encryption dictionary | Security configuration | 04 |
| Signature dictionaries | Cryptographic signatures | 04 |
| Cross-reference data | Locates objects in the file | 01 |

---

## 5. Types of PDF documents

Not every PDF is equally editable, and a platform should detect the type early because it decides which pipeline applies.

| Type | What's inside | Consequences | How to detect |
| --- | --- | --- | --- |
| **Born-digital** | Real text ops and vector graphics | Precise extraction and search; limited direct editing | Text ops present; fonts with ToUnicode |
| **Scanned / image-only** | A full-page image per page | OCR needed for text; "editing" means reconstructing the page | One large image XObject per page, few/no text ops |
| **Scanned + OCR layer** | Image + invisible text (`3 Tr`) | Searchable; text positions approximate | Text render mode 3 over an image |
| **Interactive** | Form widgets, signature fields, actions | Behaviour depends on the viewer; JS-dependent forms degrade | `/AcroForm`, widgets, `/AA`, XFA |
| **Tagged** | Structure tree | Reflow and accessibility possible; edits must keep tags consistent | `/MarkInfo /Marked true`, `/StructTreeRoot` |
| **Signed / certified** | Signature(s), incremental revisions | Any non-incremental save breaks signatures | `/Sig` fields with `/V`; DocMDP in `/Perms` |
| **Encrypted** | Encrypted strings/streams | Need the password (or empty user password) | `/Encrypt` in trailer |
| **Standards-conformant** | PDF/A, PDF/X, PDF/UA, PDF/E, PDF/VT | Edits can silently break conformance | XMP `pdfaid:`, `pdfxid:`, `pdfuaid:` |
| **Portfolio** | Collection of embedded files | The "document" is a container | `/Collection` in catalog |

---

## 6. Operations that look alike but are not

| What the user does | What actually happens in the file |
| --- | --- |
| Add text | Adds drawing instructions to the page, **or** a FreeText annotation (different editability, printing, flattening) |
| Edit existing text | Rewrites content-stream operators; constrained by the embedded font subset; no reflow |
| Highlight text | Adds a Highlight annotation; the text itself is untouched |
| Cover text with a black rectangle | Draws on top — **the text is still there and extractable** |
| Redact text | Removes the underlying glyphs, image pixels and related data, then rewrites the file (if done correctly) |
| Crop a page | Changes `/CropBox`; off-page content is still in the file |
| Delete a page | Removes it from the page tree; its objects may linger until a full rewrite garbage-collects them |
| Save after editing | Incremental save appends (old content recoverable!); full save rewrites |
| Flatten a form | Turns field appearances into static page content; values no longer machine-readable as fields |
| Sign digitally | Adds cryptographic evidence of integrity over specific byte ranges of one revision |
| Add a password with only an owner password | Encrypts, but anyone can open it — permissions are advisory |
| Compress | Re-encodes or removes resources |
| OCR | Adds recognised text, usually invisible, under the image |
| Convert PDF to Word | Reconstructs structure from fixed-layout glyphs — inherently lossy and heuristic |

**Key limitation:** PDF is a fixed-layout format, not a word-processing format. Editing a paragraph does not reflow surrounding text. Robust editing requires reconstructing the page's logical content.

---

## 7. Difficulty map

Roughly in increasing order of difficulty:

1. **Structural operations** (page tree, metadata, bookmarks, attachments, encryption): mostly object-graph surgery. A good COS (object) layer makes these easy.
2. **Generation** (new documents): hard parts are fonts, shaping, layout and tagging, but the problem is well understood.
3. **Annotations and forms**: easy to write, hard to make *look right everywhere* — appearance-stream generation is the crux.
4. **Rendering**: a large, well-specified but deep subsystem (colour, transparency, fonts, shadings). Build or wrap is a key decision.
5. **Signature validation**: crypto is easy; diff analysis between revisions to detect attacks is hard.
6. **Extraction to structure** (PDF→Word/Excel, auto-tagging): heuristic or ML; never perfect.
7. **Editing existing content in place**: hardest. It needs content-stream parsing into an editable model, font subset limits, glyph-level layout, preserving overlapping graphics, structure tags and marked content, and serialising back without breaking other viewers.

Cross-cutting needs for an editor-class product: undo/redo over an object graph, cross-viewer compatibility, reliable serialisation (incremental vs full), large-document performance, security hardening of the parser, and a regression corpus.

---

## 8. Reference architecture

A full editor breaks into a UI shell over six engines, all sharing one object model.

```
┌──────────────────────────────────────────────────────────────────┐
│           Viewer / Editor UI (page canvas, tools, overlays)      │
├──────────┬──────────┬───────────┬────────────┬──────────┬────────┤
│Rendering │ Editing  │ Document  │ Annotation │ Forms &  │Security│
│ engine   │ engine   │ engine    │ engine     │signatures│& export│
│render,   │modify    │merge,     │comments,   │widgets,  │redact, │
│zoom,     │content,  │split,     │shapes,     │validation│encrypt,│
│select,   │text,     │extract,   │markup,     │signing,  │convert,│
│search    │images    │optimise   │appearances │verify    │OCR     │
├──────────┴──────────┴───────────┴────────────┴──────────┴────────┤
│ Content model: content-stream parser/serialiser, fonts, text      │
├───────────────────────────────────────────────────────────────────┤
│ COS object layer: tolerant parser, object graph, xref, filters,   │
│ crypt, incremental + full writer                                  │
└───────────────────────────────────────────────────────────────────┘
```

Libraries that cover parts of this today (details, licences and status in [05](05-creating-pdfs-from-code.md)):

| Library | Best suited for |
| --- | --- |
| pdf.js | Browser rendering, text selection, annotation display/editing |
| pdf-lib | JavaScript manipulation, form filling, basic drawing |
| MuPDF / PyMuPDF | Fast rendering, extraction, redaction, editing (AGPL/commercial) |
| qpdf | Structure, encryption, merging, repair, linearisation |
| Apache PDFBox | Java processing, extraction, forms, signatures |
| Ghostscript | Conversion, rendering, compression, print (AGPL/commercial) |
| PDFium | Chrome's renderer; forms; permissive licence |

Some advanced editing, forms, accessibility and signing features exist today mainly in commercial SDKs, and licensing matters for a commercial product.

For a web-based product, a practical starting point is pdf.js (or PDFium/MuPDF compiled to WebAssembly) for rendering, with a separate manipulation engine.

---

## 9. Suggested phasing

Input for `docs/plans/`, not a decision. Ordered for an editor product; a library/API product would reorder around generation and processing.

| Phase | Theme | Scope |
| --- | --- | --- |
| 1 | Core viewer | Open, render, zoom, search, text selection, thumbnails, download |
| 2 | Page manipulation | Merge, split, reorder, rotate, crop, delete, duplicate, extract |
| 3 | Annotations | Highlight, draw, add text, stamps, comments, undo/redo |
| 4 | Content editing | Edit text, add images, move objects, typography |
| 5 | Forms & signatures | Create/fill fields, flatten, e-sign, sign and validate |
| 6 | Advanced workflows | OCR, secure redaction, encryption, diffing, batch processing, accessibility |

Rendering and annotations are comparatively straightforward. Reliable editing of existing text and layout is usually the hardest feature.

---

## 10. Implications for lamina

- **The COS layer is the foundation for everything.** Almost every operation in groups C, D, G, I, J and K is object-graph surgery. A tolerant parser, a strict writer, and both incremental and full save modes come first.
- **Pick the product shape before the roadmap.** "Library/API/CLI for developers", "hosted processing service" and "Acrobat-class editor" share the core but diverge sharply above it. The phasing in §9 assumes an editor.
- **Generation and processing give the best value early.** They're well specified, testable without a UI, and most operations are difficulty 1–2.
- **Build-vs-wrap is per engine.** Rendering, OCR, office conversion and HTML layout are each large enough to wrap at first (subject to licence: avoid AGPL in the core unless we choose that model deliberately). The COS layer, writer, forms, annotations and signing are where owning the code pays off.
- **Correctness traps to design for from day one:** redaction that only covers content, crop that only hides it, incremental saves that keep "deleted" content, saves that break signatures, flattening without appearance streams, and edits that break PDF/A or PDF/UA conformance.
- **Track this inventory as a capability matrix.** Each row in §3 can become a test-backed feature flag with a status (unsupported / read / write / round-trip).
