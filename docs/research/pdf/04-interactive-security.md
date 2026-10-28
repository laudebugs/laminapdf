# 04 — Interactive Features, Metadata, Security & Signatures

> Part of the lamina PDF research series. Primary reference: **ISO 32000-2:2020 (PDF 2.0)**, available at no cost from the PDF Association. Section numbers cited as "§x.y" refer to ISO 32000-2. Claims were fact-checked against ISO 32000-1:2008 (PDF 1.7) and the PDF Association's ISO 32000-2 clause 12 errata (see the Verification log at the end). Where the PDF 1.7 clause number differs, it is given as "1.7: §x.y". File syntax (objects, xref, incremental updates, object streams) is covered in **[01](01-file-structure.md)**. Content streams, XObjects and optional content in **[02](02-imaging-model.md)**. Fonts and text in **[03](03-text-and-fonts.md)**. This chapter covers everything layered *on top of* the page: navigation, annotations, forms, scripting, metadata, attachments, encryption, signatures and redaction. It also treats PDF as an attack surface.

---

## 0. Orientation: where these features live

```
trailer
 ├─ /Root ─────────► Catalog
 │                    ├─ /Outlines ──► outline tree (bookmarks)
 │                    ├─ /Names ─────► /Dests, /EmbeddedFiles, /JavaScript, /AP ... (name trees)
 │                    ├─ /Dests ─────► legacy named-destination dict (PDF 1.1)
 │                    ├─ /OpenAction, /AA ──► actions run on open / doc events
 │                    ├─ /PageLabels ► number tree
 │                    ├─ /PageMode, /PageLayout, /ViewerPreferences
 │                    ├─ /AcroForm ──► field tree, /DR, /DA, /SigFlags, /CO, /XFA
 │                    ├─ /Metadata ──► XMP stream
 │                    ├─ /AF ────────► associated files (PDF/A-3, Factur-X)
 │                    ├─ /Collection ► portfolio
 │                    ├─ /Perms ─────► /DocMDP (certification signature)
 │                    └─ /DSS ───────► Document Security Store (LTV data)
 ├─ /Info ──────────► document information dictionary (mostly deprecated in 2.0)
 ├─ /Encrypt ───────► security handler parameters
 └─ /ID [<permanent> <changing>]
Page
 ├─ /Annots [ ... ] ─► annotation dicts (links, comments, widgets, ...)
 ├─ /AA ────────────► page open/close actions
 ├─ /Thumb ─────────► thumbnail image XObject
 └─ /AF
```

Almost every feature here is **optional to render but mandatory to preserve**. A library that rewrites a PDF and drops `/AcroForm/DR`, `/DSS`, or `/AF` silently breaks forms, long-term signatures, or e-invoices. Lamina's object model must round-trip unknown keys losslessly. See §13.

---

## 1. Navigation

### 1.1 Destinations (§12.3.2)

A destination is a page reference plus a view. Explicit destinations are arrays:

| Form | Meaning |
|---|---|
| `[page /XYZ left top zoom]` | Put (left, top) at the window's upper-left corner at `zoom` (0 or `null` = keep current). Any of the three may be `null` = unchanged. |
| `[page /Fit]` | Fit the whole page in the window. |
| `[page /FitH top]` / `[page /FitV left]` | Fit width (or height); `top`/`left` positions the edge. |
| `[page /FitR left bottom right top]` | Fit the rectangle. |
| `[page /FitB]`, `/FitBH top`, `/FitBV left` | Same, but using the page's content bounding box. |

`page` is an indirect reference to a page object for local destinations. In remote destinations (`GoToR`) it is an **integer page number** in the other document (ISO 32000-1 §12.3.2.2 says only "integer page number". Treating it as a 0-based index is the common convention, but that is not stated in 1.7). PDF 2.0 adds **structure destinations** (`/SD` on GoTo actions, §12.3.2.3). They target a structure element, so a link survives reflow and page re-ordering.

**Named destinations** add a level of indirection, so links can be stable across edits:
- PDF 1.1 style: `Catalog /Dests << /name [dest] >>`, with *name objects* as keys.
- PDF 1.2+: `Catalog /Names << /Dests <name tree> >>`, with *string* keys. The values are either arrays or `<< /D [dest] >>`.

A resolver must handle both forms. It must also handle a value that is a dictionary with `/D`, and a page reference that is stale or points to a non-page object. Real files contain all of these.

### 1.2 Document outline (§12.3.3)

```
Catalog /Outlines ──► << /Type /Outlines /First 10 0 R /Last 12 0 R /Count 5 >>
                              │
       ┌──────────────────────┴──────────────────────┐
  10: << /Title (Ch 1) /Parent .. /Next 11 0 R        12: << /Title (Ch 3) /Prev 11 0 R ...>>
        /First 20 0 R /Last 21 0 R /Count 2
        /Dest [p1 /XYZ 0 792 null]  or  /A <<GoTo...>>
        /C [1 0 0]  /F 2 >>               (color; flags: 1 italic, 2 bold)
```

- Items form doubly linked sibling lists with `/Parent`, `/First`, `/Last`, `/Prev`, `/Next`.
- `/Count` on an item is the number of visible descendants. **A negative value means the item is closed.** Writers must recompute `/Count` after edits. Readers should tolerate wrong counts.
- An item has `/Dest` *or* `/A` (an action), not both. It may also have `/SE` (a structure element).
- `/Title` is a text string (PDFDocEncoding or UTF-16BE with BOM; PDF 2.0 also allows UTF-8 with BOM, see 03).
- **Defensive parsing:** outline cycles (`/Next` pointing backwards) are common in fuzzed and malicious files. Iterate with a visited set and a depth limit.

### 1.3 Actions (§12.6)

An action is a dictionary `<< /Type /Action /S /<ActionType> ... /Next <action or array> >>`. `/Next` builds a chain or tree of actions, and **it can be cyclic**, so detect cycles. Actions are triggered by:
- `Catalog /OpenAction`, which is an action or a destination.
- `/A` on link annotations and outline items.
- **Additional-actions** `/AA` dictionaries:
  - Annotations: `E` enter, `X` exit, `D` down, `U` up, `Fo` focus, `Bl` blur (`Fo`/`Bl` on widgets only), `PO`/`PC` page open/close, `PV`/`PI` page visible/invisible.
  - Pages: `O`/`C`.
  - Form fields: `K` keystroke, `F` format, `V` validate, `C` calculate.
  - Catalog: `WC` will close, `WS` will save, `DS` did save, `WP` will print, `DP` did print.

| Action | Purpose | Security note |
|---|---|---|
| `GoTo` | Local destination (`/D`); PDF 2.0 `/SD` structure destination | — |
| `GoToR` | Destination in another PDF file (`/F` file spec, `/NewWindow`) | Can leak (opens remote/UNC paths, e.g. SMB hash theft on Windows) |
| `GoToE` | Destination in an *embedded* PDF (`/T` target dict path) | Pivots into attachments |
| `GoToDp` (2.0) | Go to a document part | — |
| `URI` | Resolve a URI (`/URI` ASCII string, `/IsMap`); catalog `/URI /Base` | Phishing/tracking ("phone home") |
| `Launch` | Launch an app/open a file (`/F`, legacy `/Win` params) | **Classic malware vector** (CVE-2010-1240-era "escape from PDF") |
| `Thread` | Go to an article bead | — |
| `Sound`, `Movie` | Legacy multimedia; **deprecated in PDF 2.0** | Media-codec attack surface |
| `Hide` | Show/hide annotations or fields (`/T`, `/H`) | Can hide content dynamically → signature/visual tricks |
| `Named` | Viewer command: `NextPage`, `PrevPage`, `FirstPage`, `LastPage` (others are viewer-specific) | — |
| `SubmitForm` | Send field data (`/F` URL, `/Fields`, `/Flags`: HTML/XFDF/PDF/FDF formats) | Data exfiltration channel (used by PDFex) |
| `ResetForm` | Reset fields to `/DV` | — |
| `ImportData` | Import FDF data into fields | — |
| `JavaScript` | Run script `/JS` (string or stream) | Largest attack surface (§4) |
| `SetOCGState` | Toggle optional-content groups (`/State [/ON .. /OFF .. /Toggle ..]`) | Changes visible content without touching content streams |
| `Rendition` | Control media playback for Screen annotations | — |
| `Trans` | Page transition during actions | — |
| `GoTo3DView` | Select a 3D view of a 3D annotation | 3D (U3D/PRC) parsers are an attack surface |
| `RichMediaExecute` (2.0) | Send commands to RichMedia instances | — |

**Lamina guidance:** expose actions as a typed enum with a `Raw(Dictionary)` fallback. Never execute `Launch`, `URI`, `SubmitForm`, `JavaScript` or `GoToR` implicitly. Leave that decision to an embedding application, behind an explicit policy.

### 1.4 Link annotations (§12.5.6.5)

`<< /Type /Annot /Subtype /Link /Rect [...] /A <<action>> | /Dest dest /H /I /QuadPoints [...] /BS <<...>> /PA <<URI action>> >>`
- `/H` sets the highlight mode: `N` none, `I` invert (default), `O` outline, `P` push.
- `/QuadPoints` (PDF 1.6) lets a link follow a multi-line run of text. The spec says to fall back to `/Rect` if the quad points lie outside it.
- Links usually have **no appearance stream**. They are invisible hit areas, so text extraction never "sees" a URL unless it reads the annotations.
- Most links in generated PDFs use `/Border [0 0 0]`, which means no border. They need it because the default `/Border` is `[0 0 1]`, a 1-unit border (ISO 32000-1 Table 164).

### 1.5 Page labels, thumbnails, viewer preferences, page mode/layout

**Page labels** (§12.4.2) live in `Catalog /PageLabels`, a **number tree** keyed by 0-based page index. Each value is `<< /S style /P (prefix) /St start >>`. Styles: `/D` decimal, `/R`/`/r` upper/lower roman, `/A`/`/a` letters (A..Z, AA..ZZ, ...). A label with no `/S` is prefix-only. Example: `[0 <</S /r>> 4 <</S /D>> 120 <</S /D /P (A-)>>]` gives i–iv, 1–116, A-1…

**Thumbnails** (§12.3.4) are an optional per-page `/Thumb` image XObject. Modern viewers render their own thumbnails, so writers should drop stale thumbnails after editing a page.

**Page mode** (`Catalog /PageMode`): `UseNone` (default), `UseOutlines`, `UseThumbs`, `FullScreen`, `UseOC` (layers panel), `UseAttachments`.

**Page layout** (`/PageLayout`): `SinglePage`, `OneColumn`, `TwoColumnLeft`, `TwoColumnRight`, `TwoPageLeft`, `TwoPageRight`.

**Viewer preferences** (`/ViewerPreferences`, §12.2):
- Window chrome: `HideToolbar`, `HideMenubar`, `HideWindowUI`, `FitWindow`, `CenterWindow`, `DisplayDocTitle` (show XMP/Info title instead of filename; PDF/UA requires `true`).
- `NonFullScreenPageMode`, `Direction` (`L2R`/`R2L`).
- `ViewArea`/`ViewClip`/`PrintArea`/`PrintClip` (deprecated in 2.0).
- Printing: `PrintScaling` (`None`/`AppDefault`), `Duplex`, `PickTrayByPDFSize`, `PrintPageRange`, `NumCopies`.
- PDF 2.0 adds `Enforce`, an array naming preferences that must not be overridden (currently `PrintScaling`).

---

## 2. Annotations (§12.5)

### 2.1 The common dictionary

| Key | Meaning |
|---|---|
| `/Type /Annot`, `/Subtype` | Required subtype |
| `/Rect` | Bounding box in default user space (unrotated page coordinates) |
| `/Contents` | Text string: the comment, or alt text for non-text annots |
| `/P` | Parent page (optional, but some tools rely on it) |
| `/NM` | Unique name within the page, used for replies and FDF/XFDF round-tripping |
| `/M` | Modification date |
| `/F` | Flags bitfield (below) |
| `/AP` | Appearance dictionary: `/N` normal, `/R` rollover, `/D` down. Each is a form XObject or a dictionary of **appearance states** → XObject |
| `/AS` | Current appearance state name (required when `/AP /N` is a state dictionary: checkboxes, radios) |
| `/Border` / `/BS` / `/BE` | Legacy border array `[hr vr w [dash]]` / border style dict / border effect (cloudy) |
| `/C` | Color array (0=transparent, 1=gray, 3=RGB, 4=CMYK components) |
| `/StructParent` | Key into the structure parent tree (Tagged PDF/UA) |
| `/OC` | Optional content membership |
| `/AF` (2.0) | Associated files |
| `/ca`, `/CA` (2.0 on all annots), `/BM` (2.0) | Opacity and blend mode for the appearance. In 1.7 only markup annotations have `/CA` (Table 170). In 2.0, `ca` defaults to the value of `CA` (errata §12.5.2) |
| `/Lang` (2.0) | Language |

**Annotation flags `/F`** (bit positions verified against 1.7 Table 165; all other bits shall be 0):

| Bit | Value | Name | Effect |
|---|---|---|---|
| 1 | 1 | Invisible | Hide *unknown* subtypes with no handler |
| 2 | 2 | Hidden | Don't display or print, no interaction |
| 3 | 4 | Print | Print it (**off by default!** Without it, annotations don't print) |
| 4 | 8 | NoZoom | Don't scale with zoom |
| 5 | 16 | NoRotate | Don't rotate with the page |
| 6 | 32 | NoView | Print but don't display |
| 7 | 64 | ReadOnly | No interaction |
| 8 | 128 | Locked | Can't be moved or deleted |
| 9 | 256 | ToggleNoView | Invert NoView on hover/selection |
| 10 | 512 | LockedContents | Contents can't be edited, but the annotation can still be moved |

(Our pyHanko demo signature widget has `/F 132` = Print + Locked.)

### 2.2 The annotation zoo

| Subtype | Markup? | Notes |
|---|---|---|
| `Text` | ✔ | "Sticky note" icon (`/Name`: Comment, Note, Help, ...). Carries `/State` for review workflows |
| `Link` | – | §1.4 |
| `FreeText` | ✔ | Text drawn on the page. `/DA`, `/Q`, `/RC` rich text, `/DS`, `/CL` callout line, `/IT` (FreeTextCallout, FreeTextTypeWriter) |
| `Line` | ✔ | `/L` endpoints, `/LE` line endings, leader lines, `/Measure` |
| `Square`, `Circle` | ✔ | `/IC` interior color, `/RD` rect differences |
| `Polygon`, `PolyLine` | ✔ | `/Vertices`; `/IT` PolygonCloud/PolyLineDimension; PDF 2.0 `/Path` for curves |
| `Highlight`, `Underline`, `Squiggly`, `StrikeOut` | ✔ | "Text markup". `/QuadPoints`, 8 numbers per quad. **The spec says x1y1 x2y2 x3y3 x4y4 in counterclockwise order (1.7 Table 179, Figure 64). Acrobat writes UL, UR, LL, LR, which disagrees.** Readers must not assume an order |
| `Caret` | ✔ | Insertion mark |
| `Stamp` | ✔ | Rubber stamp (`/Name` Approved, Draft, ...); usually a custom AP image. PDF 2.0 `/IT` StampImage/StampSnapshot |
| `Ink` | ✔ | `/InkList` array of point arrays (PDF 2.0 also `/Path`) |
| `Popup` | – | Pop-up window for a markup annotation's text (`/Parent`, `/Open`) |
| `FileAttachment` | ✔ | `/FS` file spec; icon PushPin, Paperclip, ... |
| `Sound` | ✔ | **Deprecated in 2.0** |
| `Movie` | – | **Deprecated in 2.0** (use Screen + Rendition) |
| `Screen` | – | Media region; used with Rendition actions |
| `Widget` | – | Form field visual (§3) |
| `PrinterMark` | – | Registration marks, color bars |
| `TrapNet` | – | Trapping; **deprecated in 2.0** |
| `Watermark` | – | Fixed print/size behavior via `/FixedPrint` |
| `3D` | – | U3D/PRC artwork (`/3DD`, `/3DV`, `/3DA`) |
| `Redact` | ✔ | Marks content for removal (§10) |
| `Projection` (2.0) | ✔ | Associates measurements with 3D/geospatial content |
| `RichMedia` (2.0) | – | Rich media, standardized from Adobe Extension Level 3. PDF 2.0 does not support Flash content in it (per LoC FDD000474) |

All subtypes above except `Projection` and `RichMedia` are in ISO 32000-1 Table 169. `Redact` was added in PDF 1.7.

### 2.3 Markup annotations, replies, review state (§12.5.6.2)

Markup annotations add:
- `/T` author, `/Popup`, `/CA` opacity, `/RC` rich-text contents (XHTML subset), `/CreationDate`, `/Subj`.
- `/IRT` "in reply to", which points at another annotation.
- `/RT`: `/R` (reply, the default) or `/Group` (grouped with the parent).
- `/IT` intent, `/ExData`.

**Review state** is modeled as a *reply* `Text` annotation with `/IRT` to the target annotation:
- `/StateModel (Review)` with `/State`: Accepted, Rejected, Cancelled, Completed, None (the default).
- `/StateModel (Marked)` with Marked/Unmarked (the default).
- In ISO 32000-1 (Table 172) `State` and `StateModel` are **text strings**, not names. Readers should accept both.

These state annotations are usually `Hidden` (that is common practice, not a spec rule). Threads are trees of `IRT` references, so they can be cyclic.

### 2.4 Why appearance streams matter

Since PDF 1.2, an annotation's visual is *defined* by its **appearance stream** (`/AP`). Key facts:
1. **Consistency.** Viewers that honor `/AP` render identically. Viewers that synthesize appearances from `/C`, `/IC`, `/BS`, `/DA` and so on produce visibly different results across Acrobat, pdf.js, PDFium, Preview and Poppler. In PDF 1.7, `/AP` is optional, and "individual annotation handlers may ignore this entry" (Table 164). PDF 2.0 makes `/AP` **required** for every annotation, widgets included, with two exceptions. The first is subtypes `Popup`, `Link` and `Projection`. The second is annotations whose `/Rect` is a single point, where x1 = x2 **and** y1 = y2. The 2020 revision changed this condition from "or" to "and" to match PDF/A (PDF Association clause-12 errata, §12.5.2). PDF 2.0 also says a reader rendering `/AP` shall ignore `C`, `IC`, `Border`, `BS`, `BE`, `BM`, `CA`, `ca`, `H`, `DA`, `Q`, `DS`, `LE`, `LL`, `LLE`, `MK` and `Sy` (same errata). Annotation-related PDF/A and PDF/UA rules also require `/AP`.
2. **Printing and rasterization** in headless pipelines (Ghostscript, Poppler, MuPDF) often use only `/AP`. An annotation without one may simply not print.
3. **States.** Checkbox/radio `/AP /N << /Off 1 0 R /Yes 2 0 R >>` plus `/AS /Yes`. The "on" state name is *not* always `/Yes`. Discover it from the AP keys.
4. **`/NeedAppearances true`** in AcroForm asks the viewer to regenerate field appearances. It is **deprecated in PDF 2.0** and forbidden in PDF/A. Its side effects:
   - It causes "do you want to save changes?" prompts.
   - It is ignored by many non-Acrobat renderers.
   - It **invalidates signatures** if the viewer regenerates and saves.
   It is a crutch, not a solution. Lamina should generate appearances itself (§3.5).

### 2.5 Flattening

Flattening bakes annotations into page content. For each annotation with `/F` not Hidden (and respecting Print/NoView for the target):
1. Pick the appearance: `/AP /N` (resolve `/AS` for state dicts).
2. Compute the transform per §12.5.5 (Algorithm "appearance streams"):
   - Transform the XObject's `/BBox` by its `/Matrix` and take the bounding box of the result.
   - Compute matrix `A` that maps that box onto `/Rect`.
   - The full mapping from form space to the page is `AA = Matrix × A`. `Do` already applies the form's own `/Matrix`, so the `cm` you emit before `Do` is just `A`.
3. Append to the page content: `q <A> cm /FlatN Do Q`. Add the XObject to page `/Resources /XObject`, merging resource names without collisions.
4. Remove the annotation from `/Annots` (and the field from `/AcroForm /Fields` for widgets). Fix the structure tree: the widget's `/StructParent` OBJR becomes marked content, or the struct element is dropped.
5. Honor NoZoom/NoRotate. These cannot be preserved once flattened, so document that.

Pitfalls:
- Hidden/NoView annotations must *not* be flattened.
- Page `/Rotate` must not be double-applied.
- Popup annotations are never flattened.
- For forms, flatten **after** generating fresh appearances.
- Flattening a signed document invalidates the signature. That is expected; warn the caller.

---

## 3. Forms

### 3.1 AcroForm structure (§12.7; interactive form dictionary §12.7.3, 1.7: §12.7.2)

```
Catalog /AcroForm << /Fields [ 10 0 R 20 0 R ]   % root fields only
                     /DR << /Font << /Helv 5 0 R /ZaDb 6 0 R >> >>  % default resources
                     /DA (/Helv 0 Tf 0 g)        % default appearance
                     /Q 0                        % default quadding
                     /SigFlags 3                 % 1=SignaturesExist, 2=AppendOnly
                     /CO [ 30 0 R ]              % calculation order
                     /NeedAppearances false      % deprecated in 2.0
                     /XFA ...  >>                % deprecated in 2.0

10 0 obj << /T (address) /Kids [ 11 0 R 12 0 R ] /FT /Tx >>          % non-terminal field
11 0 obj << /Parent 10 0 R /T (street) /V (Main St 1)                 % terminal field AND widget
            /Type /Annot /Subtype /Widget /Rect [..] /P 3 0 R /AP << /N 40 0 R >> >>
```

- **Field tree.** Fields have partial names `/T`. The *fully qualified name* is the period-joined chain, e.g. `address.street`. A field without `/T` is unnamed. Two fields with the same FQN **are the same field** and share one value, which is common for "name" fields repeated on every page.
- **Terminal fields vs widgets.** A terminal field has one or more widget annotations as `/Kids`. When there is exactly one widget, the field dictionary and the widget annotation **may be merged** into one dictionary, as with 11 0 above. A parser decides "is this kid a field or a widget?" with these rules: a kid with `/T` is a field; a kid with `/Subtype /Widget` and no `/T` is a widget; a merged dict has both. Widgets also appear in the page's `/Annots`. **Orphans are common**: widgets in `/Annots` not reachable from `/Fields`, and fields whose widgets are missing from `/Annots`. Robust tools reconcile both.
- **Inheritable attributes:** `/FT`, `/Ff`, `/V`, `/DV`, and for variable text `/DA` and `/Q` (falling back to AcroForm `/DA`/`/Q`). Always resolve by walking up `/Parent`. A depth limit and cycle check are mandatory.
- **Other field keys:**
  - `/TU` alternate (tooltip, accessibility) name; `/TM` mapping name for export.
  - `/AA` with `K`/`F`/`V`/`C` JavaScript.
  - `/DS` default style and `/RV` rich value (rich text).
  - `/MaxLen` (text).
  - `/Opt`, `/TI`, `/I` (choice).
  - `/MK` widget appearance characteristics: `/BG`, `/BC` colors, `/CA` caption, `/R` rotation, `/I` icon, `/TP` text position.

### 3.2 Field types and flags

Field flags `/Ff` are 1-based bit positions. All positions below were verified against ISO 32000-1 §12.7.3.1 and §12.7.4. Undefined bits are reserved and shall be 0.

| Type | Flag bits |
|---|---|
| All | 1 ReadOnly, 2 Required, 3 NoExport |
| `Btn` | 15 NoToggleToOff (radios), **16 Radio**, **17 Pushbutton** (neither = checkbox), 26 RadiosInUnison |
| `Tx` | 13 Multiline, 14 Password, 21 FileSelect, 23 DoNotSpellCheck, 24 DoNotScroll, 25 Comb (needs `/MaxLen`), 26 RichText |
| `Ch` | 18 Combo (else list box), 19 Edit, 20 Sort, 22 MultiSelect, 23 DoNotSpellCheck, 27 CommitOnSelChange |
| `Sig` | — (uses `/Lock`, `/SV` seed values; §8) |

**Value formats:**
- **Text** `/V` is a text string, or a stream for long or rich text (rich text goes in `/RV`).
- **Checkbox** `/V` is a *name*: the on-state name or `/Off`. It must agree with the widget's `/AS`.
- **Radio**: the parent's `/V` is the selected kid's on-state name. Each kid widget has its own `/AP /N` with one unique on-state.
  - `/Opt` on a button field (PDF 1.5) gives export values indexed by kid order. Then the AP state names may be `/0`, `/1`, ... and the exported value comes from `/Opt`. This trips up many fillers.
- **Choice** `/V` is a text string, or an array for multi-select. `/Opt` entries are strings or `[export display]` pairs. `/I` holds selected indices, which must be kept in sync for multi-select.
- **Signature** `/V` is the signature dictionary (§8).

### 3.3 Calculation order and scripts

`/CO` lists fields whose `C` (calculate) action runs when any value changes, in array order. Format (`F`), keystroke (`K`) and validate (`V`) actions are almost always JavaScript calling Acrobat's built-in `AFNumber_Format`, `AFDate_FormatEx`, `AFSimple_Calculate`, and so on. Consequences:
- **Displayed value ≠ stored value.** `/V` holds `1234.5`, and the appearance shows `$1,234.50`. A filler that writes `/V` without running the format script produces an appearance that disagrees with Acrobat's.
- Lamina should implement a **native, non-JS re-implementation of the AF\* formatting family**, as PDFium and pdf.js effectively do. That covers the overwhelmingly common cases without a JS engine.

### 3.4 XFA

XFA (XML Forms Architecture) is an XML-based form system. The XML packets live in `AcroForm /XFA`, as a stream or as an array of `(name) stream` pairs: `xdp:xdp`, `template`, `datasets`, `config`, `localeSet`, ...
- **Static XFA** ("XFA foreground"): the PDF contains real page content and AcroForm widgets. XFA only adds behavior. Non-XFA viewers render the AcroForm fallback reasonably.
- **Dynamic XFA**: the page layout is computed from the template at runtime. The PDF pages are a placeholder: *"Please wait... If this message is not eventually replaced by the proper contents of the document, your PDF viewer may not be able to display this type of document."* Only Adobe Acrobat/Reader (and a handful of commercial SDKs) render these fully.
- **Status:** XFA 3.3 was the last spec (2012). **ISO 32000-2 deprecates XFA forms.** PDF/A forbids XFA. Prevalence is nonetheless real in government and enterprise workflows: tax, immigration and benefits forms in several countries, and LiveCycle/AEM Forms deployments. Expect maybe a low single-digit percentage of forms in the wild. This figure is an estimate, not a measured statistic.
- **Library support:**
  - pdf.js has partial XFA rendering, enabled in Firefox.
  - PDFium has an optional XFA build (`pdf_enable_xfa`).
  - iText's pdfXFA and Apryse are commercial.
  - Most open-source libraries (pypdf, pdf-lib, PDFBox, qpdf) treat XFA as opaque XML. At most they let you read or write `datasets` and remove `/XFA` to fall back to AcroForm.
- **Lamina:** read and expose the XFA packets. Allow dropping XFA, with a warning that dynamic XFA has no fallback. Allow editing `datasets`. Do **not** implement an XFA layout engine in v1.

### 3.5 Filling forms: generating appearances (the hard part)

Setting `/V` is trivial. Making every viewer *show* it correctly is not. For a text field, the generator must:

1. **Resolve `/DA`.** Walk up to AcroForm `/DA`. Parse it as a content-stream fragment, e.g. `/Helv 12 Tf 0 0 1 rg`, to get the font resource name, size and color operator.
2. **Resolve the font** in the field's `/DR`, then AcroForm `/DR`. Common traps:
   - `/Helv` and `/ZaDb` are *conventional* names. They may be missing from `/DR`.
   - The font may be a non-embedded simple font that can't encode the value, e.g. Cyrillic in WinAnsi Helvetica. Then you must either embed and subset a font (and add it to `/DR`), or fail loudly. See 03 for encodings and subsetting.
   - Each glyph used must be **encodable** in the font's encoding. For Type0/Identity-H fonts you need the CID mapping. For simple fonts, map Unicode → code via the `/Encoding` and `/Differences`.
3. **Auto-size.** A font size of `0` in `/DA` means auto. Single-line text fits the height with Acrobat-ish heuristics (and shrinks to fit the width). Multiline text needs iterative layout with word wrapping.
4. **Layout:**
   - Padding is the border width plus 1–2pt.
   - `/Q` sets alignment (0 left, 1 center, 2 right).
   - `/MK /R` rotation swaps width and height and adds a matrix.
   - **Comb fields** split the box into `/MaxLen` cells, one char per cell.
   - **Password** fields show bullets. **Multiline** fields wrap and leave a top margin.
   - Baseline: Acrobat places single-line text vertically centered using font ascent/descent. Mimic it to look native.
5. **Emit the stream** as a form XObject with `/BBox [0 0 w h]`, `/Resources` with the font, and content:
   ```
   /Tx BMC q <border/background from /MK> BT /F1 12 Tf 0 g 2 6.3 Td (value) Tj ET Q EMC
   ```
   The `/Tx BMC … EMC` marked-content wrapper comes from the spec, not only from convention. When regenerating, a viewer replaces the content between `/Tx BMC` and the matching `EMC`. If the stream has no such section, the new content is appended (1.7 §12.7.3.3).
6. **Checkbox/radio:**
   - Choose the existing on-state.
   - Set `/V` (on the parent for radios) and `/AS` on each widget.
   - Generate an appearance only if none exists. The default checkbox glyph is ZapfDingbats `4` (✓), from `/MK /CA`.
7. **Choice:**
   - Combo boxes render like text.
   - List boxes render visible options and highlight selected rows. Acrobat uses the color `0.6 0.75 0.85`.
   - Respect `/TI` for scroll position.
8. **Don't** set `NeedAppearances`. Optionally also set it as a belt-and-braces measure for Acrobat-only workflows, but never on signed documents.
9. **Signed documents:** fill via **incremental update** only. Check `/Lock` and DocMDP permissions first (§8.6).

### 3.6 FDF and XFDF

- **FDF** (Forms Data Format, §12.7.8; 1.7: §12.7.7) is a PDF-syntax file: `%FDF-1.2`, then `Root << /FDF << /Fields [ << /T (name) /V (val) /Kids [...] >> ] /F (form.pdf) /Annots [...] >> >>`. It is used by `SubmitForm`/`ImportData`, and can carry annotations.
- **XFDF** is the XML equivalent, **ISO 19444-1:2016**: `<xfdf><fields><field name="x"><value>..</value></field></fields><annots>…</annots></xfdf>`. It is the common format for exchanging comments and form data, and is used by pdf.js/Apryse and Acrobat comment export.
- Lamina should support import/export of both for fields and annotations. Match fields by FQN. Match annotations by `/NM` plus page.

---

## 4. JavaScript in PDF

- **Where it lives:**
  - `Catalog /Names /JavaScript` (a name tree of document-level scripts, run on open).
  - `/OpenAction`.
  - `/AA` triggers on pages, annotations and fields.
  - `JavaScript` actions on links and outline items.
- **The API.** The **Acrobat JavaScript API** is ECMAScript plus an object model:
  - Objects: `app`, `this`/`Doc`, `Field` (`this.getField("x").value`), `event` (`event.value`, `event.rc`, `event.change`), `util` (`util.printf`, `util.scand`), `console`.
  - Network and collaboration: `SOAP`, `Net.HTTP`, `Collab`.
  - Built-in `AF*` helper functions.
  - Functions are split into *privileged* and *non-privileged* contexts. Trust is set by folder-level scripts or certified documents.
- **Support matrix:**
  - **Adobe Acrobat/Reader:** full, with the "Enhanced Security" and Protected Mode sandboxes.
  - **pdf.js (Firefox):** supports a forms-oriented subset (field events, AF functions, `app.alert`, ...). The script runs inside a **QuickJS interpreter compiled to WebAssembly**, isolated from the page's JS.
  - **PDFium (Chrome/Edge):** an optional V8-backed "FXJS" implementing much of the forms API. Chrome supports limited form scripting.
  - **Foxit:** extensive support.
  - **Poppler/Okular, macOS Preview, MuPDF:** none or minimal. MuPDF has an optional MuJS for forms.
- **Security implications:**
  - JavaScript has been the #1 Reader exploit primitive historically: `util.printf` (CVE-2008-2992), `Collab.getIcon` (CVE-2009-0927), countless use-after-frees in the Acrobat JS engine and its object bindings.
  - It enables heap grooming for memory-corruption exploits, phone-home tracking, form exfiltration, and UI spoofing (`app.alert` phishing).
  - Signature attacks use it too: dynamic `Hide`/`SetOCGState` changes after signing.
  - **Even pdf.js shipped arbitrary-JS execution (CVE-2024-4367)**. The bug was not in its sandbox. It came from unsanitized font data (`FontMatrix`) being concatenated into a generated glyph-drawing function.
  - The lesson: *any* code generation from document data is a sink.
- **Lamina:** detect and report JavaScript (the inventory tooling must list every location). Offer a "strip all JS" sanitizer. Implement form formatting natively (§3.3). Treat JS execution as out of scope, or as a heavily sandboxed opt-in plugin (e.g. QuickJS-in-WASM with no I/O bindings).

---

## 5. Metadata

### 5.1 The Info dictionary (§14.3.3)

`trailer /Info << /Title /Author /Subject /Keywords /Creator /Producer /CreationDate /ModDate /Trapped >>`.
- `/CreationDate` and `/ModDate` are dates (§7.9.4). `/Trapped` is a name: `True`, `False` or `Unknown` (the default). It must be the name, not a boolean. All other values, including custom keys, are text strings (1.7 Table 317).
- `/Creator` is the authoring application. `/Producer` is the PDF writer.
- Custom keys are allowed.
- Dates use the format `D:YYYYMMDDHHmmSSOHH'mm`. O is `+`, `-` or `Z`.
  - Adobe's PDF Reference (≤1.7) wrote a trailing `'` (`…-08'00'`). The normative grammar in ISO 32000-1 §7.9.4 already has no trailing `'`, although that same document's Info-dictionary example still shows one.
  - Truncated forms are **valid**. Only `D:` and `YYYY` are required, and any later field may be omitted as long as all earlier fields are present. `D:2026` is legal, and MM and DD default to 01.
  - Readers must accept the trailing-`'` form, truncated forms and garbage.
  - In our demo pyHanko wrote `/M (D\07220261028150526Z)`. `\072` is just an octal escape for `:`.

**PDF 2.0 deprecates the Info dictionary except `/CreationDate` and `/ModDate`** (confirmed by LoC FDD000474). Document metadata belongs in XMP. PDF 1.7 has a precedence rule (§14.3.2). If the XMP date stamp is equal to or later than Info `/ModDate`, XMP is authoritative. Otherwise Info overrides, on the assumption that a writer unaware of XMP saved the file. In practice, nearly every reader still shows Info. Writers should keep writing both, synchronized.

### 5.2 XMP (§14.3.2; ISO 16684-1)

`Catalog /Metadata` → `<< /Type /Metadata /Subtype /XML /Length n >> stream … endstream`. The stream is RDF/XML in an **XMP packet**:

```xml
<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
     xmlns:dc="http://purl.org/dc/elements/1.1/"
     xmlns:xmp="http://ns.adobe.com/xap/1.0/"
     xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
     xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/"
     xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
   <dc:title><rdf:Alt><rdf:li xml:lang="x-default">Q3 Report</rdf:li></rdf:Alt></dc:title>
   <dc:creator><rdf:Seq><rdf:li>Ada Lovelace</rdf:li></rdf:Seq></dc:creator>
   <dc:format>application/pdf</dc:format>
   <xmp:CreateDate>2026-10-28T15:05:26Z</xmp:CreateDate>
   <xmp:ModifyDate>2026-10-28T15:05:26Z</xmp:ModifyDate>
   <xmp:CreatorTool>lamina-writer</xmp:CreatorTool>
   <pdf:Producer>lamina 0.1</pdf:Producer>
   <xmpMM:DocumentID>uuid:…</xmpMM:DocumentID>
   <xmpMM:InstanceID>uuid:…</xmpMM:InstanceID>
   <pdfaid:part>3</pdfaid:part><pdfaid:conformance>B</pdfaid:conformance>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<!-- ~2 KB of whitespace padding allows in-place edits -->
<?xpacket end="w"?>
```

- The `begin` attribute holds the UTF-8 BOM (U+FEFF). The `id` is a fixed magic string.
- The padding plus `end="w"` (writable) let tools edit the packet in place without rewriting offsets.
- XMP streams should be **unfiltered and unencrypted**, so non-PDF-aware tools can scan for the packet (1.7 §14.3.2 NOTE 2). PDF/A forbids filters on them in practice.
- Metadata streams can also attach to pages, images, fonts and other objects.
- **PDF/A** needs extension schemas declared in XMP (`pdfaExtension:schemas`) for any non-predefined properties. Factur-X's `fx:` schema is one example.

**Info ↔ XMP mapping** (keep them synchronized; PDF/A validators check equivalence):

| Info | XMP |
|---|---|
| Title | `dc:title` (x-default) |
| Author | `dc:creator` (Seq; Info stores a single string, so split or join with care) |
| Subject | `dc:description` (x-default) |
| Keywords | `pdf:Keywords` |
| Creator | `xmp:CreatorTool` |
| Producer | `pdf:Producer` |
| CreationDate | `xmp:CreateDate` |
| ModDate | `xmp:ModifyDate` |
| Trapped | `pdf:Trapped` |

Also relevant: `xmp:MetadataDate`, `dc:language`, `pdfuaid:part` (PDF/UA), `pdfx:` (PDF/X), `xmpMM:History`.

### 5.3 Document IDs

`trailer /ID [<permanent> <changing>]` holds two byte strings, conventionally 16-byte MD5-ish values:
- The first is set at creation and kept forever.
- The second changes on each save. In our demo the incremental update kept the first and replaced the second.

The ID is **an input to R2–R4 encryption key derivation**. Rewriting `/ID[0]` on an encrypted file makes it undecryptable. It is required when encrypting (1.7 Table 15: "Required if an Encrypt entry is present"), and its two strings must then be direct and unencrypted. PDF 2.0 is reported to make `/ID` required in every file. That is not verified here, so write it always. Also correlate with `xmpMM:DocumentID`/`InstanceID`.

**Privacy:** metadata leaks author names, internal paths, software versions, edit history (`xmpMM:History`) and template GUIDs. Sanitization must scrub Info, every XMP stream (document *and* object-level), `/PieceInfo` (application private data, e.g. Illustrator's full native file), `/ID`, and the incremental-update history (§10).

---

## 6. Embedded files, associated files, portfolios

### 6.1 File specifications (§7.11)

A file spec is a string or a dict:

```
<< /Type /Filespec
   /F (invoice.xml)  /UF (invoice.xml)      % UF = Unicode name (preferred)
   /Desc (Machine-readable invoice)
   /AFRelationship /Alternative             % PDF 2.0 / PDF/A-3
   /EF << /F 50 0 R  /UF 50 0 R >> >>       % embedded file stream(s)

50 0 obj << /Type /EmbeddedFile /Subtype /text#2Fxml   % MIME type, '/' escaped as #2F
           /Params << /Size 4211 /ModDate (D:2026…) /CheckSum <md5> >>
           /Filter /FlateDecode /Length … >> stream … endstream
```

Embedded files are reachable through:
- `Catalog /Names /EmbeddedFiles` (a name tree; this is the "attachments panel"),
- `FileAttachment` annotations (`/FS`),
- `/AF` arrays,
- `GoToE` targets and `/Collection`.

`/RF` (related files, PDF 1.3) and `/Thumb` (PDF 2.0; not in the 1.7 file-spec table) exist but are rare. `/UF` and `/CI` are PDF 1.7. `/Params /CheckSum` is a 16-byte MD5 of the embedded bytes. PDF 2.0 also defines **encrypted payload documents**: an unencrypted wrapper PDF whose real content is an encrypted embedded file. It carries `/AFRelationship /EncryptedPayload` and a `/EncryptedPayload` dict naming the cryptographic filter.

### 6.2 Associated files `/AF` (PDF 2.0, PDF/A-3)

`/AF` arrays of file specs can attach to the catalog, pages, XObjects, annotations, structure elements or marked content. They state a *relationship* to that object via `/AFRelationship`:

| Value | Meaning |
|---|---|
| `Source` | The original source (e.g. the .docx) |
| `Data` | Data used to derive visual content (e.g. a chart's CSV) |
| `Alternative` | An alternative representation of the content |
| `Supplement` | Supplemental representation |
| `EncryptedPayload` | Encrypted payload document |
| `FormData` | Form data |
| `Schema` | A schema |
| `Unspecified` | Default |

**PDF/A-3** (ISO 19005-3) is the archival profile that allows embedding *arbitrary* files, provided they are associated via `/AF` with `/AFRelationship`, `/Subtype` MIME and `/Params /ModDate`. (PDF/A-4f is the PDF 2.0-based successor for files with embedded files.)

**E-invoicing (ZUGFeRD 2.x / Factur-X 1.x):**
- A hybrid invoice is a **PDF/A-3** (or A-4f per newer versions) human-readable invoice with an embedded **`factur-x.xml`** (UN/CEFACT CII syntax, EN 16931 semantic model).
- The XML is attached via catalog `/AF` *and* the EmbeddedFiles name tree, with `/AFRelationship` usually `/Alternative`. Depending on profile and national rules, `/Data` or `/Source` is used instead.
- An XMP extension schema (`fx:DocumentType INVOICE`, `fx:DocumentFileName factur-x.xml`, `fx:Version`, `fx:ConformanceLevel`) declares the profile: MINIMUM, BASIC WL, BASIC, EN 16931, EXTENDED, XRECHNUNG.
- Germany's B2B e-invoicing mandate (receiving obligation since 1 Jan 2025, issuing phased 2027–2028) makes this format commercially important. Lamina should provide a first-class "attach associated file + PDF/A-3 metadata" API.

### 6.3 Portfolios / collections (§12.3.5)

`Catalog /Collection << /Schema <<field defs>> /D (initial doc) /View /D|/T|/H|/C /Sort … /Folders … >>` turns the PDF into a container. ISO 32000-1 defines only `/View` `D` (details), `T` (tile) and `H` (hidden), and has no `/Folders` or `/C`. Folders and navigators came via Adobe Extension Level 3 and PDF 2.0. The "cover sheet" pages are shown only by viewers without portfolio support. Embedded files carry `/CI` collection items, and folders give a hierarchy. `/View /C` (custom navigator) was Flash-based and is dead. Lamina should expose a collection as a list of embedded files plus metadata, and should not try to build a portfolio UI.

---

## 7. Encryption (§7.6)

### 7.1 Model

```
trailer /Encrypt << /Filter /Standard   (security handler)
                    /SubFilter …        (optional, e.g. public-key handlers)
                    /V  algorithm version
                    /R  handler revision
                    /Length key bits
                    /CF << /StdCF << /CFM /AESV3 /AuthEvent /DocOpen /Length 32 >> >>
                    /StmF /StdCF  /StrF /StdCF  /EFF /StdCF
                    /O /U /OE /UE /Perms /P
                    /EncryptMetadata true >>
```

**What gets encrypted:** *strings and streams only*. Dictionary keys, names, numbers, the object structure and the xref are plaintext. An attacker can therefore read the document *structure* (page count, annotation types, field names, object graph) without the key, and can splice in unencrypted objects. This is the root of PDFex (§7.7).

Exempt from encryption:
- strings in the `/Encrypt` dict;
- the trailer `/ID`;
- strings inside streams (content streams, object streams), which are already covered by the stream's encryption (these first three are listed in 1.7 §7.6.1);
- cross-reference streams (1.7 §7.5.8);
- the XMP metadata stream when `/EncryptMetadata false` (V4+);
- **the signature dictionary's `/Contents`**, since otherwise signing would be circular. ISO 32000-1 does not list this exemption. It is PDF 2.0 text and long-standing Acrobat behavior; the exact 2.0 wording is not verified here.

Embedded files can use their own filter (`/EFF`), so an "attachments only" encryption is possible.

**Crypt filters** (V4+; 1.7: §7.6.5) name algorithms:
- `/CFM /V2` (RC4) and `/AESV2` (AES-128-CBC, PDF 1.6) are in ISO 32000-1. `/AESV3` (AES-256-CBC) is **not**: it came from Adobe Extension Level 3 and was standardized in PDF 2.0.
- `/CFM /None` (the default) does *not* mean "unencrypted". It means the application does not decrypt, and the security handler does its own decryption.
- The standard crypt filter `/Identity` passes data through unchanged, so it means no encryption.
- For the standard handler, crypt-filter `/Length` is given in **bytes** (16 = 128-bit), while the public-key handler gives bits (1.7 Table 25). That is why pypdf's AES-256 filter below says `/Length 32`.

Streams can override the default via `/Filter [/Crypt …] /DecodeParms << /Name /X >>`. **`/Identity` and per-object overrides are what make mixed plaintext/ciphertext documents possible.**

AES strings and streams are laid out as `16-byte IV ‖ AES-CBC(PKCS#5-padded plaintext)`.

### 7.2 Standard security handler revisions

| V/R | Cipher | Key | Key derivation | Status |
|---|---|---|---|---|
| V1/R2 | RC4 | 40-bit | MD5 once | Broken: brute-forceable in seconds to minutes |
| V2/R3 | RC4 | 40–128-bit | MD5 + 50 rounds | Deprecated (RC4) |
| V4/R4 | RC4 or AES-128-CBC (crypt filters) | 128-bit | as R3 | Deprecated in PDF 2.0 |
| V5/R5 | AES-256 | 256-bit | single SHA-256 | Adobe Extension Level 3 (Acrobat 9). **Never standardized**. Fast hash → brute-force friendly. Do not write |
| V5/R6 | AES-256-CBC | 256-bit | Iterative SHA-256/384/512 (Alg. 2.B) | Adobe Extension Level 8, then PDF 2.0. **The only non-deprecated option in PDF 2.0** |

ISO 32000-1 defines only V 0–4 and R 2–4 (Tables 20–21), and says V 3 "shall not appear in a conforming PDF file". **V5, R5 and R6 do not exist in PDF 1.7 proper.** In 1.7 the `R` value follows from `V` and `P`. R2 means V < 2 and no R3-only permission cleared. R3 means V 2 or 3, or any R3-only permission cleared. R4 means V 4.
| ISO/TS 32003 | AES-GCM 128/192/256 | — | — | Extension (2023): authenticated encryption per object |
| ISO/TS 32004 | — | — | — | Extension (2024): document-level integrity (MAC) for encrypted PDFs. Addresses the PDFex class |

**R2–R4 key derivation (Algorithm 2, 1.7 §7.6.3.3; verified step by step against the spec):**
```
pw32  = (password ‖ PAD)[0:32]          # PAD = 28 BF 4E 5E 4E 75 8A 41 64 00 4E 56 FF FA 01 08
                                        #       2E 2E 00 B6 D0 68 3E 80 2F 0C A9 FE 64 53 69 7A
h     = MD5(pw32 ‖ O ‖ P_le32 ‖ ID[0] ‖ (R≥4 && !EncryptMetadata ? FF FF FF FF : ""))
if R≥3: repeat 50×: h = MD5(h[0:n])     # n = Length/8 (5 for R2)
key   = h[0:n]
per-object key = MD5(key ‖ objnum_le24 ‖ gen_le16 ‖ (AES ? "sAlT" : ""))[0 : min(n+5,16)]
```
- `/U` (R3/4) = RC4(key, MD5(PAD ‖ ID[0])), then 19 more RC4 passes with key bytes XOR i (i = 1..19), plus 16 bytes of arbitrary padding to make 32 (Algorithm 5). Checking the user password means recomputing this and comparing **only the first 16 bytes** (Algorithm 6). For R2, `/U` = RC4(key, PAD) (Algorithm 4).
- `/O` stores the *padded user* password, RC4-encrypted under a key derived from the owner password. For R3+ that is MD5 iterated 50×, and 19 extra RC4 passes (Algorithm 3). So the owner password unlocks the user password (Algorithm 7).
- Passwords are Latin-1/PDFDocEncoding bytes truncated at 32.

**R6 (Algorithm 2.A/2.B; PDF 2.0 only, not in ISO 32000-1, so not verified against a spec text here):**
- Passwords are UTF-8 after SASLprep (RFC 4013), truncated to 127 bytes.
- `/U` is 48 bytes = 32-byte hash ‖ 8-byte validation salt ‖ 8-byte key salt. `/O` is the same, but its hashes also take the 48-byte `/U` as input.

```
Algorithm 2.B  hash(pw, salt, udata):          # udata = U (48 B) for owner, empty for user
  K = SHA-256(pw ‖ salt ‖ udata)
  for round = 0, 1, 2, …:
     K1 = (pw ‖ K ‖ udata) repeated 64 times
     E  = AES-128-CBC-encrypt(key=K[0:16], iv=K[16:32], K1)   # no padding
     h  = (sum of E[0:16] as big-endian int) mod 3  → SHA-256 / SHA-384 / SHA-512
     K  = h(E)
     stop when round ≥ 63 and E[-1] ≤ round − 32
  return K[0:32]
```
- Validate the user password: `hash(pw, U[32:40], "") == U[0:32]`.
- The file key: `AES-256-CBC-decrypt(key=hash(pw, U[40:48], ""), iv=0, UE)` (owner: `O[40:48]`, `U`, `OE`).
- **`/Perms`** = AES-256-ECB(file key, `P_le32 ‖ FF FF FF FF ‖ ('T'|'F' EncryptMetadata) ‖ "adb" ‖ 4 random bytes`). It lets readers detect tampering with `/P`. Decrypt it and compare.
- The file key is random (32 bytes) and is *the same key for every object*. There is no per-object key derivation in AES-256.

### 7.3 Verified example (pypdf 6.17.0, AES-256, empty user password)

```
5 0 obj
<< /V 5 /R 6 /Length 256 /P 516 /Filter /Standard
   /O <e2f28a2d…d3ebc52e1c296de77fdcd61497735135>     % 48 bytes
   /U <6b908122…8496112fb68894d06ea3f1bc7178aad7>     % 48 bytes
   /CF << /StdCF << /AuthEvent /DocOpen /CFM /AESV3 /Length 32 >> >>
   /StmF /StdCF /StrF /StdCF
   /OE <93fc817d…9695404d>  /UE <c0abccc7…449efbf9>  % 32 bytes each
   /Perms <4949fdbeaaa17aa9c5b879df46fc163a> >>       % 16 bytes
```
```
$ pdfinfo enc.pdf
Title:      Secret
Encrypted:  yes (print:yes copy:no change:no addNotes:no algorithm:AES-256)
```
- `pdfinfo` decrypted the Title **without any password**, because the user password is empty.
- `/P 516` = bit 3 (print, 4) + bit 10 (accessibility extraction, 512). pdfinfo reports `copy:no` because bit 5 is clear.
- Note that pypdf emitted `/P` as a small positive number, with the reserved high bits clear. With reserved bits 7–8 and 13–32 set, the same permissions would be the negative 32-bit value `-3388` (0xFFFFF2C4).
  - ISO 32000-1 (§7.6.3.2) states this only in a NOTE: "all the reserved high-order flag bits … are required to be 1", so `P` "shall be specified as a negative integer". The explicit table rows "bits 1–2 shall be 0; 7–8 and 13–32 shall be 1" are from PDF 2.0 Table 22 and Adobe's PDF Reference. They are not in ISO 32000-1's Table 22.
  - The same 1.7 clause also says "conforming readers shall ignore all flags other than those at bit positions 3, 4, 5, 6, 9, 10, 11, and 12". So Poppler accepting `516` is correct reader behavior, not leniency.
- **Lesson for lamina:** readers must interpret `/P` as a 32-bit value (two's complement, masked with `0xFFFFFFFF`, then read unsigned) and look only at the defined bits. Writers should set the reserved bits, which gives a negative `P`.

### 7.4 Permission bits `/P`

| Bit | Value | Permission (when set) |
|---|---|---|
| 3 | 4 | Print (low-quality if bit 12 clear, R≥3) |
| 4 | 8 | Modify contents (other than 6, 9, 11) |
| 5 | 16 | Copy/extract text & graphics |
| 6 | 32 | Add/modify annotations, fill fields (and with bit 4, create fields) |
| 9 | 256 | Fill existing form fields (even if 6 clear) (R≥3) |
| 10 | 512 | Extract for accessibility. **PDF 2.0: deprecated; readers should treat it as always set** |
| 11 | 1024 | Assemble (insert/rotate/delete pages, bookmarks, thumbnails) |
| 12 | 2048 | High-quality print |

Bits 3–6 apply to all revisions. Bits 9–12 apply to R≥3, and under R2 their operations fall under bits 5/6. Readers ignore every other bit (1.7 Table 22 and §7.6.3.2). The bit-10 deprecation is PDF 2.0 and is not verified here.

### 7.5 User vs owner passwords: why "owner-only protection" isn't security

- The **user password** (the "open password") gates derivation of the file key.
- The **owner password** gates *permissions* in compliant software.
- If the user password is empty, which is the common "protected PDF that opens without a password" case, **anyone can derive the file key**, decrypt everything, and ignore `/P`. Permissions are enforced only by the honesty of the application. `qpdf --decrypt in.pdf out.pdf` removes them in milliseconds, and our pdfinfo run above showed the "encrypted" title in plaintext.
- Lamina must still **honor `/P` by default** (spec compliance, legal expectations). It should offer an explicit override flag, as qpdf, pypdf and PDFBox do, and document clearly that permission bits are a policy marker, not access control.

### 7.6 Public-key security handler (§7.6.5; 1.7: §7.6.4)

- `/Filter /Adobe.PubSec` with `/SubFilter /adbe.pkcs7.s3`, `/adbe.pkcs7.s4` or `/adbe.pkcs7.s5` (s5 uses crypt filters). s3 uses R2 permission semantics and s4 uses R3 semantics.
- The file key is wrapped per recipient in PKCS#7/CMS **EnvelopedData** objects, listed in `/Recipients`. For s5 the list is in the crypt filter dict instead. Each object's enveloped payload is a 20-byte seed plus 4 bytes of permissions. Its `/P` has an extra bit 2: "change encryption, all permissions" (1.7 Table 24).
- Key derivation is SHA-1 over the 20-byte seed plus the recipients' DER (1.7 §7.6.4.3). With AES-256 (`/AESV3`) it is SHA-256, which is PDF 2.0 and not verified here.
- It is used in enterprise and government settings. pyHanko and iText support it. Lamina should support decryption (with a caller-supplied private-key callback, e.g. HSM/PKCS#11) before encryption.

### 7.7 PDFex (Müller et al., ACM CCS 2019)

The researchers evaluated **27 viewers. 23 (85%) were vulnerable to direct exfiltration, and all 27 to CBC gadgets** (pdf-insecurity.org).

1. **Direct exfiltration.** Because only strings and streams are encrypted, and crypt filters allow `/Identity`, an attacker can *add unencrypted objects* to an encrypted PDF without knowing the key:
   - a `SubmitForm` or `URI` action, or a JavaScript action,
   - that references encrypted form values or content,
   - triggered by `/OpenAction`.
   The viewer decrypts the content after the user enters the password and then sends the plaintext to the attacker.
2. **CBC malleability gadgets.**
   - With RC4 or AES-CBC without integrity protection, known plaintext gives a gadget. The `/Perms` value has known structure, and AES-256 uses one key for all objects.
   - The attacker XORs controlled changes into ciphertext blocks to *construct* encrypted objects, e.g. a URL action, that exfiltrate other decrypted content.

**Mitigations:**
- Don't process documents that mix plaintext and encrypted objects. Warn on `/Identity` and on unencrypted strings in encrypted files.
- Disable active content in encrypted docs, or prompt before network actions.
- At the spec level, ISO/TS 32003 (GCM) plus **ISO/TS 32004 (integrity protection)** close the gap.

Lamina's reader should flag "partially encrypted" documents as suspicious.

---

## 8. Digital signatures (§12.8)

### 8.1 Objects involved

```
Catalog /AcroForm << /Fields [... 7 0 R] /SigFlags 3 >>
        /Perms << /DocMDP 11 0 R >>                 % only if certification signature
        /DSS  << /Certs [..] /OCSPs [..] /CRLs [..] /VRI <<..>> >>   % LTV
Page    /Annots [... 7 0 R]
7 0 obj  % signature field merged with its widget
<< /FT /Sig /T (Sig1) /Type /Annot /Subtype /Widget /F 132 /Rect [50 700 250 750]
   /P 4 0 R /AP << /N 10 0 R >> /V 11 0 R
   /Lock 12 0 R   (optional, PDF 1.5, must be indirect: << /Type /SigFieldLock /Action /All >>)
   /SV 13 0 R     (optional, PDF 1.5, must be indirect: seed values: SubFilter, digest, cert constraints) >>
11 0 obj % signature dictionary
<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached
   /ByteRange [a b c d]  /Contents <hex CMS ...000>
   /M (D:...) /Name /Reason /Location /ContactInfo
   /Reference [ << /Type /SigRef /TransformMethod /DocMDP|/FieldMDP
                   /TransformParams << ... >> >> ]   (optional)
   /Prop_Build << /Filter << /Name /... >> /App << /Name /... >> >> >>
```

`/SigFlags` bit 1 is SignaturesExist. Bit 2 is AppendOnly: a non-incremental save may invalidate signatures. 1.7 Table 219 only says readers "may use this flag" to warn the user and require confirmation before a full save. Lamina should treat it as "append only".

### 8.2 The ByteRange / Contents mechanism

The signature covers the **entire file bytes except the hex string value of `/Contents`**:

```
offset 0                       a+b=b            c                         c+d = EOF
 │◄──────── range 1 (signed) ────────►│◄─ gap ─►│◄──── range 2 (signed) ─────►│
 %PDF-1.7 ... 11 0 obj << /Contents  <3082...0000>  /ByteRange [0 b c d] ... %%EOF
                                     ▲            ▲
                                    '<'          after '>'
```

Rules:
- `a = 0`.
- The gap is exactly the `<…>` hex string *including* its delimiters.
- `c + d` = the end of the file (of the signed revision).
- `/ByteRange` itself is inside the signed range, so it cannot be altered.
- `/Contents` holds a DER-encoded CMS object, hex-encoded and **zero-padded** to the reserved length. 1.7 Table 252 says that when `/ByteRange` is present, `/Contents` "shall be a hexadecimal string". It also says space for it "must be allocated before the message digest is computed".
- The spec itself only says the range "should be the entire file, including the signature dictionary but excluding the signature value". Rules like `a = 0` and "the gap is exactly the hex string" are what robust validators enforce (§8.8). The spec does not spell them out.
- When a byte range digest is present, **all values in the signature dictionary shall be direct objects** (1.7 §12.8.1).

**Exact signing procedure:**

1. Build the document, or an incremental update to an existing one, containing:
   - the signature field/widget,
   - `/SigFlags`,
   - the signature dictionary with
     - `/Contents <000…000>`, a hex placeholder of 2×N zeros, where N bounds the expected DER size: certificate chain + timestamp token + revocation info if embedded. 8–16 KB of DER is common for B-T. Placeholders of 16–32 KB are used when OCSP responses go into the CMS.
     - `/ByteRange [0 ********** ********** **********]`, a fixed-width placeholder (spaces or digits).
   - The signature dictionary must be written as an ordinary indirect object, not compressed inside an object stream, because its bytes must be patchable in place. This is a practical requirement; ISO 32000-1 does not state it. Its entries must be direct objects, as above.
2. Serialize the complete file (with xref/trailer) to bytes.
3. Locate the `<` and `>` of the `/Contents` value: `b` = offset of `<`, `c` = offset after `>`, `d = len − c`. Overwrite the ByteRange placeholder in place with `[0 b c d]`, padded with spaces to the same length. **No byte offsets change.**
4. Compute `digest = H(bytes[0:b] ‖ bytes[c:c+d])`.
5. Build a **detached CMS SignedData** (RFC 5652) with `eContentType id-data`, no eContent, and signed attributes:
   - `content-type`,
   - `message-digest = digest`,
   - for PAdES, `signing-certificate-v2` (ESS, RFC 5035), and **no** `signing-time` attribute (PAdES uses `/M` instead).

   Then sign the DER of the signed attributes. Optionally request an RFC 3161 timestamp over the *signature value* and add it as the unsigned attribute `id-aa-signatureTimeStampToken` (that gives B-T).
6. Hex-encode the DER. If it is longer than the placeholder, **fail** and retry with a bigger placeholder. Otherwise write it at `b+1` and leave the trailing zeros.
7. Done. Any later change must be an **incremental update** appended after `c+d`.

**Remote/HSM signing** splits steps 4–5: compute the hash locally, send signed-attribute bytes to the signer, then embed. Lamina's API should expose this "prepare → external sign → finalize" flow (pyHanko and iText call it "deferred/external signing").

### 8.3 Verified example: byte layout of a real signature

Generated with pyHanko 0.38.0 (self-signed RSA-2048, SHA-256, `/SubFilter /ETSI.CAdES.detached`), as an incremental update on top of a one-page pypdf file. The signature object begins at file offset **2636**:

```
offset 2636: "11 0 obj\n"          (9 bytes)
offset 2645: "<<\n"                (3 bytes)
offset 2648: "/Contents "          (10 bytes)
offset 2658: "<3082057D06092A864886F70D010702A082056E3082056A020101310F300D0609608648016503040201...
              ...000000000000>"    (1 + 4226 + 1 = 4228 bytes)
offset 6886: "\n/ByteRange [0 2658 6886 550]                                             \n
              /Type /Sig\n/Filter /Adobe.PPKLite\n/SubFilter /ETSI.CAdES.detached\n
              /M (D\07220261028150526Z)\n>>\nendobj\nxref ... trailer ... /Prev 256 ... startxref\n7063\n%%EOF\n"
```

Arithmetic:

| Quantity | Value | Check |
|---|---|---|
| range 1 | `[0, 2658)` | 2636 + 9 + 3 + 10 = **2658** = offset of `<` |
| gap | `[2658, 6886)` = 4228 bytes | 2 delimiters + **4226** hex chars (2113 reserved bytes) |
| range 2 | `[6886, 7436)` = 550 bytes | 6886 + 550 = **7436** = file size ✔ |
| CMS DER | 1409 bytes = 2818 hex chars | remaining 1408 hex chars are `0` padding |
| digest | SHA-256(range1 ‖ range2) = `c4b3aeef…d993a357` | equals the CMS `messageDigest` attribute (checked with `openssl cms -print`) ✔ |

- The DER begins `30 82 05 7D` (SEQUENCE, length 0x57D = 1405, so 1409 bytes total) and then `06 09 2A864886F70D010702`, the OID 1.2.840.113549.1.7.2 = signedData.
- The signed attributes are exactly `contentType`, `messageDigest` and `signingCertificateV2`, which is PAdES-compliant (no `signingTime`).
- The ByteRange is padded with trailing spaces because it was written as a placeholder before the offsets were known.
- pyHanko also added `/Extensions << /ESIC << /BaseVersion /1.7 /ExtensionLevel 1 >> >>` to the catalog. This is the ETSI developer extension marker for PAdES on PDF 1.7.
- Validation with pyHanko: the original file gave `intact=True valid=True coverage=ENTIRE_FILE modification_level=NONE`. After changing one byte inside range 1 (`612` → `613` in MediaBox) it gave `intact=False`.

### 8.4 SubFilters

| SubFilter | `/Contents` | Hash | Status |
|---|---|---|---|
| `adbe.x509.rsa_sha1` (1.3) | Raw PKCS#1 RSA signature (DER OCTET STRING). Cert chain in `/Cert` (required for this SubFilter) | Not limited to SHA-1 despite the name (1.7 Table 257 note a) | Deprecated in PDF 2.0 (errata, Table 260) |
| `adbe.pkcs7.sha1` (1.4) | CMS SignedData *encapsulating* the SHA-1 digest of the ranges | SHA-1 over the data | Deprecated in PDF 2.0 (errata, Table 260) |
| `adbe.pkcs7.detached` (1.3) | Detached CMS SignedData over the ranges | SHA-1 (1.3), SHA-256 (1.6), SHA-384/512, RIPEMD-160 (1.7) | Widely used ("PDF signature", ISO 32000-1 style; PAdES-compatible-ish "CMS" profile) |
| `ETSI.CAdES.detached` (**PDF 2.0**) | Detached CAdES (CMS + ESS signing-certificate-v2, no signing-time) | SHA-256+ | **PAdES baseline**. Use this. Not in ISO 32000-1; on 1.7 files it is used under the ETSI `ESIC` developer extension |
| `ETSI.RFC3161` (**PDF 2.0**) | An RFC 3161 TimeStampToken (itself CMS SignedData) over the ranges. Dict `/Type /DocTimeStamp` | — | **Document timestamp** (PAdES B-LTA) |

ISO 32000-1 Table 252 lists only the three `adbe.*` values. The PDF 2.0 SubFilter list, with versions, is confirmed by the clause-12 errata (Table 255).

- ISO/TS 32001 adds SHA-3 and SHA-512/… hash identifiers.
- **ISO/TS 32002** adds ECDSA curves and EdDSA (Ed25519/Ed448) for PDF signatures.
- RSA-PSS is already allowed through CMS.

### 8.5 PAdES baseline levels (ETSI EN 319 142-1; current V1.2.1, 2024-01)

```
B-B   CAdES-detached CMS: signing-certificate-v2, message-digest, content-type; /M for claimed time
 │   + RFC 3161 timestamp on the signature value (unsigned attr)  OR  a document timestamp
B-T   proves the signature existed at time T
 │   + validation material in the DSS: full cert chains, OCSP responses and/or CRLs
 │     (for signer AND TSA certs), added via incremental update
B-LT  validatable after the signer's cert expires or the CA goes offline
 │   + a document timestamp (/DocTimeStamp, ETSI.RFC3161) covering the DSS,
 │     its own validation data added to the DSS; repeat before the TSA cert or
 │     algorithms weaken ("archive timestamp chaining")
B-LTA long-term archival
```

- EN 319 142-2 defines extended (non-baseline) PAdES profiles.
- The creation and validation procedures live in **ETSI EN 319 102-1**. Validation reports are in TS 119 102-2.
- EU eIDAS qualified-signature workflows expect PAdES baseline. The EU **DSS** library (Java) is the reference validator.

**Document Security Store** (§12.8.4.3 in PDF 2.0; **not in ISO 32000-1**). The 2.0 errata say "Long term validation (LTV) of signatures was added in PDF 2.0" through DSS and document-timestamp dictionaries, and that all DSS/VRI keys are PDF 2.0. Before 2.0 the DSS came from ETSI TS 102 778-4 (PAdES-LTV) as a developer extension.
- `Catalog /DSS << /Certs [streams] /OCSPs [streams] /CRLs [streams] /VRI << /<SHA1-HEX-of-sig-Contents-DER> << /Cert [] /OCSP [] /CRL [] /TU (date) /TS stream >> >> >>`.
- VRI keys are the uppercase hex SHA-1 of the signature's CMS bytes, i.e. the `/Contents` value without padding.
- VRI is optional and many modern validators ignore it, using the global arrays instead.
- The DSS is always added by an **incremental update after signing**. Validators classify it as an "LTV update". In PDF 2.0, incremental updates that only add DSS data and/or document timestamps "shall be ignored" when evaluating DocMDP restrictions. That holds for every `P` value, **including P=1** (errata, §12.8.5.3).

**LTV** ("long-term validation") in Adobe terms means B-LT/B-LTA-like material is present. Acrobat shows "Signature is LTV enabled".

### 8.6 Certification vs approval signatures; MDP

- **Approval signatures**: any number, one per signature field.
- **Certification signature** (author signature, PDF 1.5): **at most one**, and it must be the **first** signed field in the document (1.7 §12.8.1, §12.8.2.2.1).
  - It is referenced from `Catalog /Perms /DocMDP`.
  - Its `/Reference` holds `<< /TransformMethod /DocMDP /TransformParams << /Type /TransformParams /P n /V /1.2 >> >>`. `/V` is the *name* `/1.2`, not a number.

| DocMDP `/P` (1.7 Table 254) | Allowed after certification (via incremental updates) |
|---|---|
| 1 | No changes at all (any change invalidates the certification) |
| 2 (default) | Fill in forms, instantiate page templates, sign |
| 3 | As 2, plus annotation creation, deletion, and modification |

In PDF 2.0, adding DSS data and document timestamps is ignored for MDP purposes at every level (see §8.5).

**FieldMDP** (`/TransformMethod /FieldMDP`, params `/Action /All | /Include | /Exclude`, `/Fields [names]`) locks specific fields when a signature is applied. The signature field's **`/Lock` dictionary** (`/Type /SigFieldLock`, PDF 1.5, so it is in ISO 32000-1 Table 233) declares which fields to lock *when this field gets signed*. PDF 2.0 adds `/P` to `/Lock`, letting an approval signature also set DocMDP-like restrictions. The errata confirm this as Table 236, "P number (Optional; PDF 2.0)".

**Seed values** `/SV` (PDF 1.5) let a form author constrain signers: `/Filter`, `/SubFilter`, `/DigestMethod` (1.7), `/Cert` constraints (subject, issuer, policy OIDs, key usage), `/Reasons`, `/TimeStamp` URL (1.6), `/MDP` (1.6), `/LegalAttestation`, `/AddRevInfo` (1.7), and `/LockDocument` (2.0, unverified). The `/Ff` bits mark which entries are required constraints and which are only hints.

### 8.7 Incremental updates after signing

Each signature covers the file *as it was* when signed: revision *n*. Later revisions are appended:

```
[ rev0: original ][ rev1: sig A field + dict ][ rev2: form fill ][ rev3: sig B ][ rev4: DSS + DocTimeStamp ]
 ◄──── sig A covers rev0..rev1 ────►
 ◄──────────────── sig B covers rev0..rev3 ─────────────────────────►
 ◄──────────────────────── doc timestamp covers rev0..rev4 ─────────────────────────────────►
```

A validator must therefore answer two questions:
1. Is the signature cryptographically valid over its byte ranges?
2. Are **all changes made after the signed revision permitted**?

The second is the hard one. It needs a semantic diff of the object graph between revisions, using the DocMDP/FieldMDP rules plus "always-allowed" changes: DSS, doc timestamps, new signature dicts, `/V` of unlocked fields, widget appearances matching those values. pyHanko's `DiffPolicy` and the EU DSS library implement this. Most naive validators only check (1). **That gap is exactly what the attacks below exploit.**

### 8.8 Attacks on signatures (Ruhr-Universität Bochum "PDF Insecurity" research)

**"1 Trillion Dollar Refund – How To Spoof PDF Signatures"** (Mladenov, Mainka, Meyer zu Selhausen, Grothe, Schwenk; ACM CCS 2019). They found **21 of 22 desktop viewers** and **5 of 7 online validation services** vulnerable (pdf-insecurity.org; CVE-2018-16042, CVE-2018-18688, CVE-2018-18689).

1. **Universal Signature Forgery (USF).** Malformed signature dicts make the validator skip verification but still show "valid":
   - `/Contents` missing, `null`, or empty;
   - `/ByteRange` missing, `null`, `[]`, or with garbage values;
   - `/Cert` or `/Filter` set to unexpected values.

   *Defense:* any structural anomaly is a hard failure, never a "skip".
2. **Incremental Saving Attack (ISA).** Append a revision that redefines objects (e.g. page content) after the signed range. Naive validators verify the ranges (still valid) and don't flag the update. Variants try to hide the update:
   - remove or alter the new xref/trailer so the validator thinks the signature covers the whole file;
   - put new content in a revision with no xref, relying on viewer repair;
   - make the signature's `d` cover a forged `%%EOF`.

   *Defense:* ensure `c+d` is a genuine revision end, then do the semantic diff of later revisions (§8.7). Report "signed revision ≠ current document" prominently.
3. **Signature Wrapping Attack (SWA).**
   - Manipulate `/ByteRange` so that the original signed `/Contents` value is *moved*. Change `c` so the gap is larger.
   - Use the extra uncovered space to insert malicious objects, along with a new xref.
   - The CMS still verifies over the (redefined) ranges.

   *Defense:*
   - require `a=0`;
   - require `b` = exact offset of the `<` of *this* signature's `/Contents`;
   - require the gap = exactly the hex string;
   - require `c+d` = end of a revision;
   - require no other bytes in the gap;
   - require the `/Contents` value to be ASCII hex only.

**PDFex** (§7.7) appeared at the same venue the same year.

**Shadow Attacks** (Mainka, Mladenov, Rohlmann; disclosed July 2020, NDSS 2021). The pdf-insecurity.org site reports **15 of 28 desktop viewers** vulnerable to at least one variant, including Acrobat and Foxit (CVE-2020-9592/-9596). The earlier figure here, "16 of 29", may be the NDSS paper's count; that is not verified. The attacker prepares the *unsigned* document with hidden "shadow" content. The victim signs it. The attacker then makes a **permitted-looking** incremental update that reveals the shadow:
- **Hide:** a full-page overlay object (image or form XObject) sits in the signed document, *referenced but not displayed*, or the reverse. The later update toggles which one is shown, e.g. by changing an object stream reference or an xref entry so the overlay vanishes.
- **Replace:**
  - Change form-field values or *font descriptors* through allowed updates.
  - Or redefine objects (e.g. a font's `/Widths`/glyphs) so the signed text renders differently.
  - Annotation-level changes count too: the FreeText appearance or a checkbox `/AS` swapping in a prepared appearance.
- **Hide-and-Replace:** the signed document contains a complete second document structure. The update swaps in a different `/Pages` tree, page content, or catalog by redefining objects with the *same* object numbers.

*Defense:* object-level diffing must flag *any* redefinition of objects reachable from rendering (content streams, resources, fonts, XObjects, pages tree), not merely "new annotations". Warn when the signed document contains unreferenced objects or optional content toggled by later updates. Validators should render and compare the signed revision with the current one, which is the robust if expensive approach.

**Certification attacks — "Breaking the Specification: PDF Certification"** (Rohlmann, Mladenov, Mainka, Schwenk; IEEE S&P 2021). Of 26 applications, **15 were vulnerable to EAA, 8 to SSA, and 24 to at least one**. 11 implemented the certification permissions incorrectly. pdf-insecurity.org confirms the 24/26 and 11/26 figures. The EAA and SSA counts come from the paper and were not re-checked.
- **Evil Annotation Attack (EAA):** DocMDP P=3 permits adding annotations. The attacker adds a FreeText, Stamp or other annotation whose appearance stream *draws arbitrary content* over the certified page. That makes it visually indistinguishable from page text, and even P=2 is affected in some viewers via widgets.
- **Sneaky Signature Attack (SSA):** P=2 permits adding signatures. The attacker adds a *signature field* whose widget appearance overlays content, e.g. replaces an IBAN. The signature doesn't even need to be valid in some viewers.
- The paper also showed **code injection** that runs high-privilege JavaScript in Adobe products in certified documents.

*Defense:*
- Treat annotation and signature-widget appearances added after certification as potentially content-altering.
- Restrict permitted annotation types and appearance content: no overlays on text.
- Show added annotations prominently.
- Prefer P=1 or P=2 with strict widget rules for high-value documents.

**Validator checklist for lamina (summarized):**
1. Parse every revision separately and identify the signed revision by `c+d`.
2. Enforce the structural rules above: ByteRange shape, hex-only Contents, gap exactness.
3. Verify the CMS: message-digest, signature, `signing-certificate-v2` binding, and the cert path at the relevant time. Check timestamps and revocation (OCSP/CRL from the CMS or DSS).
4. Diff later revisions under DocMDP/FieldMDP/Lock rules and classify each change (LTV, form fill, annotation, signature, *other*). Any "other" → INVALID or "modified".
5. Report coverage: whole file, or not the latest revision.
6. Never let JavaScript, OCG state or `Hide` actions run during validation rendering.
7. Treat validity as a structured result, not a boolean. pyHanko's `intact/valid/trusted/coverage/modification_level` is a good model.

---

## 9. Signing via incremental update: practical constraints

- A signature *must* be added by an incremental update when other signatures exist. Full rewrites invalidate prior signatures. In general a library must:
  - keep the original bytes verbatim,
  - append new and modified objects, a new xref section/stream with `/Prev`, and a trailer that keeps `/ID[0]`,
  - never "optimize" or renumber.
- Using a **hybrid-reference** or xref-stream style consistent with the original is advisable. Some validators choke on mixing classic xref and xref streams across revisions.
- Encrypted + signed: the signature dict's `/Contents` stays unencrypted (§7.1). Other strings in the sig dict (`/Reason`, `/M`) *are* encrypted. Byte ranges are over the encrypted file bytes.
- Visible signatures: the widget's `/AP /N` is drawn at signing time. Since it is part of the signed bytes, changing it later is a modification.
- Linearized files: appending an update breaks linearization. That is acceptable, and viewers fall back.

---

## 10. Redaction

### 10.1 Drawing a black box is not redaction

A rectangle drawn over text, whether as an annotation or a `re f` in the content stream, leaves every glyph in the content stream. Copy-paste, text extraction, search, screen readers and `pdftotext` all recover it. Famous failures:
- **TSA Screening Management SOP (2009).** A "redacted" airport screening manual was posted online with black boxes over still-present text.
- **US military report on the death of Nicola Calipari (2005).** Classified names and details were recovered by copy-paste.
- **Paul Manafort court filing (Jan 2019).** Lawyers' "redacted" passages about sharing polling data were readable by copying the text.
- Numerous court filings and FOIA releases since.

Research by Bland, Iyer and Levchenko, *"Story Beyond the Eye: Glyph Positions Break PDF Text Redaction"* (PETS 2023), showed something subtler. Even after the glyphs are *removed*, the **positions of surrounding glyphs** (TJ kerning adjustments, `Td` offsets, word-spacing) leak the width of the redacted text, often enough to identify names from a candidate list. They de-redacted real documents this way.

### 10.2 What true redaction must do

For each region (and/or text match) marked for redaction:

| Layer | Action |
|---|---|
| Text in content streams | Remove `Tj`/`TJ`/`'`/`"` glyphs whose bounds intersect. Re-emit the remaining glyphs with **re-computed positions that don't encode the removed widths**: a fixed placeholder advance, or re-positioning each remaining run absolutely. Handle Type 3 fonts, and text in form XObjects, patterns and annotation APs |
| Vector graphics | Clip or remove paths intersecting the area (or remove whole path ops) |
| Images | **Modify pixel data** of each image XObject/inline image intersecting the area (decode, blank pixels, re-encode). Watch out for images reused on several pages and for soft masks (`/SMask`), alternates and JPX layers. Masking with a clip is *not* enough |
| Form XObjects | Shared across pages: copy before editing, or edit globally with care |
| Annotations | Remove or clip annotations in the area, including their `/Contents`, `/RC`, popups, replies |
| Form fields | Remove values (`/V`, `/DV`, appearance) for affected fields |
| Hidden content | Optional content (OCGs off), content outside the CropBox, white-on-white text, text render mode 3 (invisible, typical OCR layer!), tiny fonts, content under images |
| Structure tree | Remove `/ActualText`, `/Alt`, `/E` of affected elements; prune struct elements |
| Metadata | Info, XMP (document + object level), `/PieceInfo`, bookmarks/outline titles, named-destination names, page labels, file names, embedded files, JavaScript strings, thumbnails (`/Thumb` shows the old page!) |
| History | **Incremental updates**: earlier revisions still contain the unredacted objects. Do a **full rewrite**, drop unreferenced objects and free-list remnants, re-encrypt if needed |
| Overlay | Optionally draw the redaction mark (`/IC` color, `/OverlayText`) as *new* content |

**Redact annotations** (§12.5.6.23) express *intent*:
- `/QuadPoints` or `/Rect`, `/IC` interior color, `/RO` overlay form XObject, `/OverlayText`, `/Repeat`, `/DA`/`/Q`.
- Applying them, as Acrobat's "Apply Redactions" does, executes the steps above and removes the annotation.

Lamina should separate "mark" (annotate), "preview" (render with overlays) and "apply" (destructive, full-rewrite). Applying should produce a report of what was removed per layer, plus a post-condition check: extract text, render, and diff.

---

## 11. PDF as an attack surface

### 11.1 Threat catalogue

- **Active content:**
  - JavaScript (§4); the `Launch` action, e.g. Didier Stevens' 2010 "escape from PDF" demo that ran `cmd.exe` with a spoofable dialog;
  - `URI`/`GoToR`/`SubmitForm` phone-home or credential leaks. `GoToR` and `/F` to UNC paths make Windows send NTLM hashes.
  - `/OpenAction` auto-triggers.
- **Embedded payloads:** EXE/script/Office attachments (`/EmbeddedFiles`, FileAttachment), sometimes auto-opened via JS `exportDataObject`. Phishing PDFs with a single big link are the most common malicious PDFs in email today.
- **Memory-corruption in parsers and decoders:**
  - **Fonts:** Adobe CoolType SING-table overflow, CVE-2010-2883; TrueType/Type1 charstring interpreters (Duqu used a Windows kernel TrueType bug, CVE-2011-3402).
  - **Image codecs:** JBIG2, JPX/OpenJPEG, CCITT, DCT.
  - **Xref/object streams, predictors, function evaluators.**
  - **FORCEDENTRY (CVE-2021-30860)**, analyzed by Google Project Zero (Beer & Groß, Dec 2021):
    - It was a zero-click iMessage exploit by NSO Group.
    - The vehicle was a PDF file disguised with a `.gif` extension.
    - The bug was an integer overflow in the **JBIG2** decoder (Xpdf-derived code in Apple's CoreGraphics).
    - It escaped the BlastDoor sandbox.
    - The exploit used JBIG2 *refinement* segments' logical operations (AND, OR, XOR, XNOR) to build **a Turing-complete virtual machine out of 70,000+ segment commands**. That VM did the address arithmetic. It shows that "obscure" decoders are the crown jewels.
- **Resource exhaustion (DoS):**
  - decompression bombs (Flate ratio > 1000:1; chained filters);
  - huge image dimensions (`/Width 1e6 /Height 1e6`);
  - deep nesting of arrays/dicts and recursive `/Parent` or `/Kids` loops;
  - xref `/Prev` cycles; object streams that reference each other;
  - outline and `/Next` action cycles;
  - Type 4 PostScript functions and shading complexity;
  - pathological content streams (millions of ops, `q` nesting);
  - JBIG2 symbol dictionaries and JPX tiles;
  - Type 3 fonts recursively drawing themselves.
- **Logic and spec-level attacks:** signature attacks (§8.8), PDFex (§7.7), polyglots (PDF+ZIP/HTML/JS), and parser-differential attacks. Different viewers interpret the same file differently: duplicate keys, multiple `%%EOF`s, broken xref repair, overlapping objects. The same file can show one content to a scanner and another to a human.

### 11.2 Fuzzing and hardening practice

- **OSS-Fuzz** continuously fuzzes **pdfium, poppler, mupdf, qpdf**, xpdf and others, and has found thousands of bugs in them. pdf.js is fuzzed by Mozilla's tooling.
- The Chrome PDFium sandbox, the Acrobat Protected Mode sandbox, and Apple's BlastDoor are all "assume the parser is compromised" designs.
- **Sanitization / CDR (Content Disarm & Reconstruction):**
  - **Structural CDR:** parse, drop active content (JS, actions except internal GoTo, embedded files, launch, multimedia, XFA), and re-serialize a clean, fully rewritten file. Examples: qpdf-based scripts, commercial CDR (OPSWAT, Glasswall), Didier Stevens' `pdfid`/`pdf-parser` for triage.
  - **Pixel CDR:** render each page to a bitmap in an isolated sandbox and rebuild a PDF from the images (optionally plus OCR). Example: Freedom of the Press Foundation's **Dangerzone**, which renders inside gVisor containers. This is the most robust approach, but it loses text, links and accessibility.

### 11.3 Guidelines for building a safe parser (lamina)

1. **Memory-safe implementation language** for everything touching untrusted bytes: Rust, or a GC'd language. Wrap any C/C++ codecs (OpenJPEG, JBIG2dec, FreeType) in a sandbox or WASM, or replace them over time.
2. **Explicit, configurable limits** with sane defaults:
   - max file size and object count;
   - max nesting depth (~100–256);
   - max decoded stream size and decompression ratio;
   - max image pixels (e.g. 2^28);
   - max xref `/Prev` chain length and revisions;
   - max content-stream operators per page;
   - max recursion for XObjects, Type 3 fonts and patterns;
   - max name/number tree depth;
   - timeouts and fuel counters for functions and JS.
3. **Cycle detection everywhere** a reference is followed (visited sets keyed by object number). Make "lazy object resolution with a recursion guard" a core primitive.
4. **Two parsing modes:**
   - *strict*, for validation and signatures: no repair, report anomalies;
   - *lenient*, for rendering: repair xref, tolerate garbage.

   Expose which repairs happened. Repairs are security-relevant (ISA/SWA rely on them).
5. **No implicit active behavior:** actions and JS are data. Network access and file launching are opt-in callbacks.
6. **Process isolation** for rendering in services (separate process or WASM instance per document, seccomp, no network).
7. **Continuous fuzzing from day one:** cargo-fuzz/libFuzzer harnesses for lexer, xref, each filter, fonts and the content parser. Join OSS-Fuzz once public. Seed with corpora from pdf.js, PDFium and qpdf test suites and the PDF Association's test suites.
8. **Differential testing** against pdf.js/PDFium/MuPDF/qpdf to find parser-differential issues.

---

## 12. Library landscape (reference points)

| Capability | pyHanko | pypdf | qpdf | PDFBox | iText 9 | pdf-lib (JS) | pdf.js / PDFium |
|---|---|---|---|---|---|---|---|
| AcroForm fill + appearance gen | partial | yes (basic AP gen) | – | yes | yes (best-in-class) | yes (basic) | render + interactive fill |
| XFA | – | read XML | – | read XML | pdfXFA (commercial) | – | pdf.js partial render; PDFium optional |
| Encrypt R6 / decrypt | via pypdf-like internal | yes | yes | yes | yes | – (decrypt not supported) | decrypt |
| Public-key handler | yes | – | – | yes | yes | – | partial |
| PAdES B-B…B-LTA sign | **yes** (reference-quality) | – | – | yes (manual DSS) | yes | – | – |
| Signature validation w/ diff analysis | **yes** | – | – | basic | yes | – | basic (display) |
| Redaction apply | – | – | – | – | pdfSweep (commercial) | – | – (MuPDF has `apply_redactions`) |

(This is a snapshot from general knowledge as of 2026. Verify specific features before depending on them.)

---

## 13. Implications for lamina's design

1. **A lossless object model and incremental writer are prerequisites** for forms, signatures, DSS/LTV, and annotation workflows.
   - Lamina needs two write modes: "append revision" (preserves bytes, required for signed docs; auto-selected when `/SigFlags` AppendOnly or any `/Sig` `/V` exists) and "full rewrite" (required for redaction and sanitization).
   - Track per-object provenance by revision.
2. **Revision-aware reader.** Expose `document.revisions()`, each with its own xref view. Allow loading "the document as of revision *n*". This powers signature validation, forensic diffing and "what changed" UIs.
3. **Typed, round-trippable features layer**: Destinations, Actions, Outline, PageLabels, Annotations, Fields, FileSpecs, Metadata. Each is a typed view over the underlying dictionaries with `extra` passthrough. Never drop unknown keys.
4. **An appearance-generation engine** is a core module, not an afterthought. It needs:
   - DA parsing;
   - font resolution with encoding checks and automatic embedding/subsetting of a fallback font (depends on 03's font subsystem);
   - text layout (auto-size, multiline, comb, alignment, rotation);
   - checkbox/radio/choice renderers;
   - native AF\* number/date/percent formatting.

   Test against Acrobat's output visually.
5. **Metadata as one coherent API** that writes Info+XMP in sync, updates `ModDate`/`MetadataDate`/`InstanceID`/`/ID[1]` on save, and supports XMP extension schemas (PDF/A, Factur-X).
6. **Associated files / Factur-X** as a first-class API: `attach(file, relationship, mime, target=catalog|page|…)`, plus helpers for PDF/A-3 conformance metadata.
7. **Crypto module:**
   - Read R2–R6 and the public-key handler. Write **R6 AES-256 only** by default, with an explicit legacy opt-in for R4.
   - Plan for ISO/TS 32003/32004.
   - Honor `/P` by default, with an explicit override.
   - Flag mixed/partially encrypted files.
   - Treat `/Perms` mismatch as a warning.
   - Use audited crypto crates/libraries; never hand-roll AES/SHA.
8. **Signing module** modeled on pyHanko's architecture:
   - placeholder reservation with automatic size estimation and retry;
   - pluggable `Signer` trait (local key, PKCS#11/HSM, remote/cloud signing via prepare→sign→finalize);
   - CAdES attributes;
   - RFC 3161 client;
   - OCSP/CRL fetchers;
   - DSS writer;
   - document timestamps;
   - DocMDP/FieldMDP/Lock/seed values.

   Default output: `ETSI.CAdES.detached`, SHA-256, B-T if a TSA is configured.
9. **Validation module** implementing the §8.8 checklist, with structured results and *modification classification*. Ship a regression corpus of USF/ISA/SWA/shadow/EAA/SSA attack files. The RUB researchers published samples; reproducing them from the papers is also feasible.
10. **Redaction module** with mark/preview/apply. Apply re-positions remaining glyphs to resist glyph-position de-redaction, does pixel-level image redaction, scrubs metadata/OCR layers/thumbnails, and always does a full rewrite. It emits a verification report.
11. **Security posture:** memory-safe core, limits and cycle guards as first-class config, strict vs lenient modes with a repair log, no implicit active content, fuzzing in CI from the first commit, and a `sanitize()` CDR pipeline (structural) with an optional pixel CDR mode.
12. **JavaScript:** inventory and strip by default. Execution only via an optional sandboxed plugin, if ever.

---

## References

**Standards**
- ISO 32000-1:2008, *Document management — Portable document format — Part 1: PDF 1.7* (Adobe-hosted copy; used for the verification pass)
- PDF Association, *Errata for PDF specifications: ISO 32000-2:2020, clause 12 (Interactive features)*: https://pdf-issues.pdfa.org/
- Library of Congress, *PDF 2.0 (ISO 32000-2)*, format description FDD000474: https://www.loc.gov/preservation/digital/formats/fdd/fdd000474.shtml
- ISO 32000-2:2020, *Document management — Portable document format — Part 2: PDF 2.0* (no-cost access): https://pdfa.org/announcing-no-cost-access-to-iso-32000-2-pdf-2-0/ · https://pdfa.org/sponsored-standards/
- ISO/TS 32001 (hash extensions), ISO/TS 32002 (signature extensions: ECC/EdDSA), ISO/TS 32003:2023 (AES-GCM), ISO/TS 32004:2024 (integrity protection in encrypted documents): https://pdfa.org/sponsored-standards/ · https://www.iso.org/standard/45877.html
- ETSI EN 319 142-1 V1.2.1 (2024-01), *PAdES digital signatures; Part 1: Building blocks and PAdES baseline signatures*: https://www.etsi.org/deliver/etsi_en/319100_319199/31914201/ (copy: https://developer.adobe.com/document-services/docs/assets/70fef5ab21077e8ef86384a148080a0d/ETSI_EN_319_142-1.pdf)
- ETSI EN 319 102-1 (creation & validation of AdES): https://www.etsi.org/deliver/etsi_en/319100_319199/31910201/
- RFC 5652 (CMS), RFC 3161 (Time-Stamp Protocol), RFC 5035 (ESS signing-certificate-v2), RFC 4013 (SASLprep), RFC 6960 (OCSP)
- ISO 16684-1 (XMP), ISO 19005-3 (PDF/A-3), ISO 19444-1:2016 (XFDF)
- Factur-X / ZUGFeRD: https://fnfe-mpe.org/factur-x/ · https://www.ferd-net.de/

**Attacks & security research**
- PDF Insecurity project (RUB/Münster): https://pdf-insecurity.org/ (the landing page was checked on 2026-10-28 for the viewer counts in §7.7 and §8.8)
- Mladenov et al., "1 Trillion Dollar Refund – How To Spoof PDF Signatures", ACM CCS 2019: https://nds.ruhr-uni-bochum.de/research/publications/1-trillion-dollar-refund-how-spoof-pdf-signatures · paper: https://pdf-insecurity.org/download/paper-pdf-signatures-ccs2019.pdf
- Müller et al., "Practical Decryption exFiltration: Breaking PDF Encryption", ACM CCS 2019, DOI 10.1145/3319535.3354214: https://pdf-insecurity.org/download/paper-pdf_encryption-ccs2019.pdf
- Mainka, Mladenov, Rohlmann, "Shadow Attacks: Hiding and Replacing Content in Signed PDFs", NDSS 2021: https://www.ndss-symposium.org/ndss-paper/shadow-attacks-hiding-and-replacing-content-in-signed-pdfs/ · https://www.ndss-symposium.org/wp-content/uploads/ndss2021_1B-4_24117_paper.pdf
- Rohlmann, Mladenov, Mainka, Schwenk, "Breaking the Specification: PDF Certification", IEEE S&P 2021: https://www.nds.ruhr-uni-bochum.de/research/publications/breaking-specification-pdf-certification · vulnerability report: https://www.nds.rub.de/media/nds/veroeffentlichungen/2021/05/25/Vulnerability-Report_Attacks-on-PDF-Certification_V2.pdf
- Mainka/Mladenov/Müller, "How to Break PDFs", 36C3: https://media.ccc.de/v/36c3-10832-how_to_break_pdfs
- Beer & Groß, "A deep dive into an NSO zero-click iMessage exploit: Remote Code Execution" (FORCEDENTRY, CVE-2021-30860), Google Project Zero, Dec 2021: https://projectzero.google/2021/12/a-deep-dive-into-nso-zero-click.html
- Citizen Lab, FORCEDENTRY: https://citizenlab.ca/2021/09/forcedentry-nso-group-imessage-zero-click-exploit-captured-in-the-wild/
- CVE-2024-4367 (pdf.js arbitrary JS execution via FontMatrix), Codean Labs: https://codeanlabs.com/blog/research/cve-2024-4367-arbitrary-js-execution-in-pdf-js/
- Bland, Iyer, Levchenko, "Story Beyond the Eye: Glyph Positions Break PDF Text Redaction", PETS 2023: https://petsymposium.org/popets/2023/popets-2023-0069.php
- Didier Stevens, PDF tools (pdfid, pdf-parser) and "Escape from PDF" (2010): https://blog.didierstevens.com/programs/pdf-tools/
- OSS-Fuzz: https://github.com/google/oss-fuzz (projects: pdfium, poppler, mupdf, qpdf)
- Dangerzone (pixel CDR): https://dangerzone.rocks/

**Implementations used / referenced**
- pyHanko (signing/validation; used for the verified example, v0.38.0): https://github.com/MatthiasValvekens/pyHanko · docs: https://pyhanko.readthedocs.io/
- pypdf (encryption example, v6.17.0): https://github.com/py-pdf/pypdf
- qpdf (encryption/inspection): https://qpdf.readthedocs.io/
- EU DSS (reference AdES validator): https://github.com/esig/dss
- pdf.js: https://github.com/mozilla/pdf.js · PDFium: https://pdfium.googlesource.com/pdfium/
- Adobe, *JavaScript for Acrobat API Reference*: https://opensource.adobe.com/dc-acrobat-sdk-docs/library/jsapiref/
- Adobe, *XML Forms Architecture (XFA) Specification 3.3* (2012)

---

### Appendix A: reproducing the verified examples

Scripts and outputs live in [`examples/04-signing-encryption/`](examples/04-signing-encryption/): `demo.py`, `signed.pdf`, `tampered.pdf`, `enc.pdf`, `cms.der`.

```bash
pip install pyhanko pypdf
openssl req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 365 -subj "/CN=Lamina Demo Signer"
python3 demo.py                     # creates in.pdf → signed.pdf (PAdES B-B), prints ByteRange arithmetic
openssl cms -cmsout -inform DER -in cms.der -print | grep -A3 messageDigest
pdfinfo signed.pdf                  # "Form: AcroForm"
pdfinfo enc.pdf                     # "Encrypted: yes (... algorithm:AES-256)"
```

---

## Verification log

**Date:** 2026-10-28

**Sources used**
- ISO 32000-1:2008 (PDF 1.7), full text. Clauses 7.5.5, 7.5.8, 7.6, 7.9.4, 7.11, 12.2–12.8 and 14.3.
- PDF Association errata for ISO 32000-2:2020 clause 12 (interactive features). Used for PDF 2.0 clause numbers and for the PDF 2.0 text of Tables 166, 176, 236, 255, 260 and 263, and §12.8.4–12.8.5.
- pdf-insecurity.org landing page (RUB). Used for the viewer counts in the PDFex, signature-spoofing, Shadow and certification attacks.
- Library of Congress FDD000474 (PDF 2.0). Used for the deprecations: the Info dictionary, Movie/Sound, weak encryption and Flash.

**Corrections (was → now)**
- PDF 2.0 `/AP` requirement (§2.4, 2.0 §12.5.2). Was: required except Popup, Link and "zero-area" `/Rect`. Now: the exemptions are Popup, **Projection**, Link, and a `/Rect` where x1 = x2 **and** y1 = y2. Also added that 1.7 has `/AP` optional, plus the 2.0 list of keys a reader ignores when rendering `/AP`.
- Flattening transform (§2.5, §12.5.5). Was: "draw with `cm = Matrix × A`", which contradicted the `q <A> cm … Do Q` step that followed. Now: AA = Matrix × A is the full mapping, and since `Do` already applies `/Matrix`, the emitted `cm` is `A`.
- Annotation states (§2.3, 1.7 Table 172). Was: `/StateModel /Review` written as names. Now: text strings in 1.7, and readers should accept both. Defaults added. "Usually Hidden" is now labelled as practice, not spec.
- Text-markup QuadPoints (§2.2, 1.7 Table 179). Added the spec's "counterclockwise" wording.
- Info dictionary types (§5.1, 1.7 Table 317). Was: "all values are text strings except /Trapped". Now: CreationDate and ModDate are dates, and Trapped is a name (default Unknown).
- Date format (§5.1, 1.7 §7.9.4). Was: "PDF 1.7 had a trailing `'`; PDF 2.0 drops it". Now: the trailing `'` is Adobe PDF Reference style, and ISO 32000-1's normative grammar already omits it. Truncated forms such as `D:2026` are valid per spec, not garbage.
- `/ID` (§5.3, 1.7 Table 15). Was: "PDF 2.0 requires it in all files with /Encrypt and recommends it always". Now: 1.7 requires it when `/Encrypt` is present, and the PDF 2.0 "always required" claim is marked unverified.
- Encryption exemptions (§7.1, 1.7 §7.6.1). Added "strings inside streams" and the §7.5.8 cite for cross-reference streams. The signature `/Contents` exemption is now labelled as PDF 2.0, because it is not in 1.7.
- Crypt filters (§7.1, 1.7 §7.6.5, Table 25). `/AESV3` is now labelled not-in-1.7 (Extension Level 3 → PDF 2.0). `/CFM /None` was implicitly "no encryption"; it now means "the security handler decrypts", and `/Identity` is the passthrough. Added that the standard handler gives CF `/Length` in bytes.
- V/R table (§7.2, 1.7 Tables 20–21). Added that ISO 32000-1 has only V 0–4 / R 2–4, and V5/R5/R6 are not in 1.7. Added the rule that derives R from V and P. R6 is now attributed to Extension Level 8 → PDF 2.0.
- Algorithms 3–7 (§7.2). The `/U` R3+ check compares only the first 16 bytes. Added R2 `/U` = RC4(key, PAD). `/O` encrypts the *padded* user password.
- Algorithm 2.A/2.B (§7.2). Now labelled PDF 2.0 only and not verified against spec text.
- `/P` reserved bits (§7.3, 1.7 §7.6.3.2). Was: "the spec says reserved bits 7–8 and 13–32 'shall be 1'; Poppler accepted the file anyway". Now: in ISO 32000-1 this is only a NOTE ("required to be 1", so P is negative). The explicit table rows are PDF 2.0 and the Adobe Reference. 1.7 also says readers "shall ignore all flags other than 3, 4, 5, 6, 9, 10, 11, 12", so accepting `/P 516` is conforming behavior. Checked that the value with reserved bits set is `-3388` (0xFFFFF2C4).
- Permission table (§7.4). Added the revision applicability and the reader-ignore rule.
- Public-key handler (§7.6). Clause is now 1.7 §7.6.4 (2.0 §7.6.5). Added s3/s4 permission semantics, the 20-byte seed plus 4-byte permissions payload, `/P` bit 2, and the s5 recipients living in the crypt filter. AESV3/SHA-256 is labelled 2.0.
- Collections (§6.3, 1.7 §12.3.5). Added that 1.7 defines `/View` D/T/H only, and that `/Folders` and `/C` are Extension Level 3 / PDF 2.0.
- File specifications (§6.1, 1.7 §7.11.3). `/Thumb` on a file spec is labelled PDF 2.0. Added the versions for `/UF`, `/CI` and `/RF`.
- FDF clause (§3.6). Added 1.7 §12.7.7 (2.0 §12.7.8 is confirmed by the errata). The AcroForm dictionary clause is now 2.0 §12.7.3 (1.7 §12.7.2).
- `/Tx BMC … EMC` (§3.5, 1.7 §12.7.3.3). Was: "conventional". Now: specified behavior, including the append-if-absent rule.
- `/SigFlags` AppendOnly (§8.1, 1.7 Table 219). Was: "viewers must save incrementally". Now: the spec only says readers *may* warn. Lamina policy is stated separately.
- Signature field example (§8.1, 1.7 Table 232). `/Lock` and `/SV` must be indirect references, and both are PDF 1.5.
- ByteRange/Contents (§8.2, 1.7 §12.8.1, Table 252). Added the rule that "all values in the signature dictionary shall be direct objects". Added the spec's "should be the entire file" wording and the hex-string requirement. The `a = 0` / exact-gap rules are labelled validator practice, and the object-stream point practical.
- SubFilters (§8.4, 1.7 Table 252/257; 2.0 errata Table 255/260). `ETSI.CAdES.detached` is now labelled **PDF 2.0**; it was unlabelled. Added the version for each SubFilter, the digest support in 1.7, and the errata cites for the 2.0 deprecations.
- DSS (§8.5, 2.0 §12.8.4.3). Now labelled not-in-1.7 (LTV "added in PDF 2.0", per the errata). Was: the DSS update is "allowed under DocMDP P≥2". Now: in PDF 2.0, DSS and document-timestamp updates are ignored for MDP at every P, including P=1.
- DocMDP table (§8.6, 1.7 Table 254). Removed "(incl. DSS/doc timestamps)" from the P=2 row because it is not 1.7 text, and noted the 2.0 rule. `/V /1.2` is a name. The "first signed field" rule is now cited.
- `/Lock` (§8.6). Confirmed as PDF 1.5 (in 1.7). `/Lock /P` is confirmed as PDF 2.0 by the errata (Table 236). The seed-value entry list is completed with versions.
- Attack figures (§7.7, §8.8). PDFex: was "27 viewers, all vulnerable"; now 23/27 vulnerable to direct exfiltration and 27/27 to CBC gadgets. Signature spoofing: added 5/7 online validators and the CVEs. Shadow attacks: was "16 of 29"; the site says 15 of 28, and both are now noted. Certification: 24/26 and 11/26 confirmed.
- References. Added ISO 32000-1, the errata site and LoC. Updated the pdf-insecurity.org note.

**Confirmed without change (selection)**
- The action-type list, with GoToDp and RichMediaExecute as 2.0 only. The trigger-event keys.
- Annotation flag bits 1–10. The annotation subtype list and markup column, with Projection and RichMedia as 2.0 only.
- The IRT, RT and IT entries.
- The Ff bits for Btn, Tx and Ch. The inheritable field attributes. `/DA`, `/DR` and `/NeedAppearances` (default false).
- Destination syntax. Outline `/Count` semantics. Page-label indices.
- Link `/H` (default I) and the QuadPoints fallback.
- The Info keys. The padding string, the MD5 ×50 rule and the per-object key with "sAlT" in Algorithms 1–2.
- The meanings of permission bits 3–12.
- DocMDP P values 1–3 (default 2) and the FieldMDP Action values.
- The Redact annotation entries.

**Still unverified (no PDF 2.0 spec text available)**
- PDF 2.0 deprecations: NeedAppearances, XFA (LoC says only that 2.0 "appears to limit" XFA), TrapNet, ViewArea/ViewClip/PrintArea/PrintClip, and permission bit 10.
- PDF 2.0 additions: `Enforce`, `/SD` structure destinations, UTF-8 text strings, `/Path` on Polygon/Ink, the `StampImage`/`StampSnapshot` intents, `/LockDocument`, and the exact wording of the signature-`/Contents` encryption exemption.
- PDF 2.0 file-identifier requirement. The 0-based remote page number.
- R6 Algorithm 2.A/2.B details, the `/Perms` layout and SASLprep. Password encoding for R2–R4.
- VRI key derivation. The PAdES level descriptions (ETSI EN 319 142-1 not consulted). ISO/TS 32001–32004 contents.
- The EAA/SSA counts (15 and 8), and the Shadow-attack paper figure.
- Historical incidents and CVE details (FORCEDENTRY, CVE-2024-4367, redaction failures). The library-landscape table and XFA prevalence.
