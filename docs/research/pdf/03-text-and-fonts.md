# 03 — Text, Fonts, Extraction & Accessibility

> Part of the lamina PDF research series. Primary reference: ISO 32000-2:2020 (PDF 2.0), chiefly §9 (Text), §14.7–14.9 (Logical structure, Tagged PDF, accessibility support). Object syntax, streams, filters and the xref live in **[Chapter 01 (File syntax)](01-file-structure.md)**; the graphics state, CTM, colour spaces and painting operators live in **[Chapter 02 (Graphics & colour)](02-imaging-model.md)**. This chapter assumes both. Normative claims were checked against ISO 32000-1:2008 (PDF 1.7) and Adobe TN #5411; clause numbers below are the same in both editions unless a 1.7 number is given — see the **Verification log** at the end.

---

## 0. TL;DR for implementers

1. **A PDF page contains positioned glyphs, not text.** A content stream says "select font F2 at 18 pt, move to (72, 640), paint glyph IDs 0x16, 0x1C, 0x1C…". Any notion of characters, words, lines, paragraphs, reading order or language is either (a) recovered heuristically by an extractor, or (b) declared explicitly by the producer via `ToUnicode` CMaps, `ActualText`, and the structure tree (Tagged PDF).
2. **Writing text is a pipeline PDF does none of:** Unicode text → itemisation (script, bidi, language) → font selection & fallback → shaping (HarfBuzz) → line breaking → justification → positioned glyph runs → PDF operators. lamina must own this pipeline or delegate it to proven libraries.
3. **Embedding fonts is a subsetting problem:** collect used glyph IDs, compute glyph closure (composites, `seac`, GSUB if you keep layout tables), rewrite `glyf`/`loca`/`hmtx`/`CFF`, tag the name `ABCDEF+Name`, and emit a `/W` widths array and a `ToUnicode` CMap that maps every glyph you painted back to the Unicode it represents.
4. **Default to Type 0 / `Identity-H` / CIDFontType2 (TrueType outlines) or CIDFontType0 (CFF outlines)** for anything beyond WinAnsi Latin. Simple fonts (≤ 256 codes) are fine for small Latin subsets but are a trap for general text.
5. **Extraction quality is determined at write time.** Always emit a correct `ToUnicode`, real space glyphs between words, `ActualText` where shaping breaks the 1:1 char↔glyph assumption, and a structure tree. Then lamina-generated PDFs can be PDF/UA-2 / WTPDF conformant and extract perfectly.

---

## 1. The core insight: PDF stores positioned glyphs, not text

PDF descends from PostScript, a page-description language whose job was to put marks on paper. A text-showing operator such as `Tj` takes a **string of bytes**, interprets those bytes as **character codes** through the current font's encoding, maps each code to a **glyph** (an outline program), paints it at the current text position, and advances the position by the glyph's width. That is all.

```
 bytes in content stream ──(font encoding / CMap)──► character codes / CIDs
                                                        │
                       ┌────────────────────────────────┴───────────────────┐
                       ▼                                                    ▼
          (CIDToGIDMap / cmap / charset)                              (ToUnicode CMap)
                       │                                                    │
                       ▼                                                    ▼
              glyph ID → outline → pixels                       Unicode (for extraction ONLY)
```

The left branch is required for rendering. The right branch is **optional** and used only for extraction, search, copy/paste and accessibility. A PDF can render perfectly while carrying no recoverable text at all.

Consequences:

| Concern | Why it is hard in PDF |
|---|---|
| **Extraction** | Codes are often arbitrary (subset fonts with `Identity-H` encode glyph IDs). Without `ToUnicode`, Unicode must be guessed from glyph names, from the embedded font's `cmap`, or by OCR. |
| **Word boundaries** | Producers frequently omit space characters and instead *move* the pen (`Td`, `TJ` offsets). Extractors infer spaces from gaps. |
| **Reading order** | Content stream order is painting order, not reading order. A two-column page may be painted in any order; headers, footers and page numbers can be anywhere. |
| **Search / copy-paste** | Ligatures (`ﬁ`), contextual forms (Arabic), reordered glyphs (Indic reph, bidi visual order) and decomposed accents break 1:1 char↔glyph mapping. |
| **Reflow** | There are no paragraphs or lines, only absolutely positioned glyph runs. Reflow (mobile "liquid mode", screen readers) needs a structure tree or ML layout analysis. |
| **Editing** | Changing a word means re-shaping and re-laying-out the line or paragraph, possibly needing glyphs missing from the embedded subset, and rewriting the content stream with no paragraph model to guide it. "Edit text in PDF" features are heuristic reconstructions. |

Tagged PDF (§9 of this chapter) is PDF's answer to all of these: a parallel, logical description of the content that is authored at generation time.

---

## 2. Text objects and operators (ISO 32000-2 §9.2–9.4)

### 2.1 Text objects

Text is shown inside a **text object** delimited by `BT` … `ET`. `BT` resets the text matrix `Tm` and text line matrix `Tlm` to identity. Text objects cannot nest, and only certain operators are permitted inside (text state, positioning, showing, colour, general graphics state such as `w` and `gs`, marked content, `cs`/`sc`). `q`/`Q` and path construction/painting are **not** permitted inside `BT`/`ET` (ISO 32000-1 §8.2, Figure 9 / Annex L: a text object allows general graphics state, colour, text state, text-showing, text-positioning and marked-content operators only). (Some real-world files violate this; renderers tolerate it, writers must not.)

The CTM and graphics state (Chapter 02) still apply: text is transformed by `Tm × CTM`.

### 2.2 Text state parameters

Text state is part of the graphics state (saved/restored by `q`/`Q`), **not** reset by `BT`. Defaults in parentheses.

| Op | Parameter | Meaning |
|---|---|---|
| `Tc` | char spacing `Tc` (0) | Extra space added after **every** glyph, in unscaled text space units. |
| `Tw` | word spacing `Tw` (0) | Extra space added after every **single-byte code 32**. Does **not** apply to multi-byte codes — so with `Identity-H` (2-byte codes), `Tw` has no effect. Justification in CID fonts must use `TJ` or `Tc`. (§9.3.3: "shall not apply to occurrences of the byte value 32 in multiple-byte codes"; it does apply to a composite font whose CMap defines code 32 as a single-byte code.) |
| `Tz` | horizontal scaling `Th` (100 → 1.0) | Percentage stretch of glyphs and spacing horizontally. |
| `TL` | leading `Tl` (0) | Line distance used by `T*`, `'`, `"`. |
| `Tf` | font `Tf` and size `Tfs` | `/F1 12 Tf`. **No default — font and size "shall be specified explicitly by using Tf before any text is shown" (§9.3.1, Table 105).** |
| `Tr` | render mode `Tmode` (0) | See §2.5. |
| `Ts` | rise `Trise` (0) | Baseline shift (super/subscript), unscaled text space units. |

Knockout (`TK`) is set via ExtGState, not an operator (§9.3.8; it applies to whole text objects and must not be changed between `BT` and `ET`). Text state values persist across text objects and are reset to defaults at the start of each page (§9.3.1).

### 2.3 Positioning operators

| Op | Effect |
|---|---|
| `tx ty Td` | `Tm = Tlm = [1 0 0 1 tx ty] × Tlm` — move to start of next line, offset from start of current line. |
| `tx ty TD` | `-ty TL` then `tx ty Td` (also sets leading). |
| `a b c d e f Tm` | Set `Tm = Tlm = [a b c d e f]` directly (not concatenated). |
| `T*` | `0 -Tl Td`. |

### 2.4 Showing operators

| Op | Effect |
|---|---|
| `string Tj` | Show glyphs for the string. |
| `[ … ] TJ` | Array of strings and numbers. A number `n` moves the pen by `−n/1000 × Tfs × Th` horizontally (in horizontal writing) — **positive values move left** (tighten, i.e. kerning), negative values move right. |
| `string '` | `T*` then `string Tj`. |
| `aw ac string "` | `aw Tw`, `ac Tc`, then `string '`. |

### 2.5 Text rendering modes (`Tr`)

| Mode | Fill | Stroke | Add to clip | Typical use |
|---|---|---|---|---|
| 0 | ✔ | | | Normal text |
| 1 | | ✔ | | Outlined text |
| 2 | ✔ | ✔ | | Faux bold (stroke with line width) |
| 3 | | | | **Invisible** — OCR text layers, hidden searchable text |
| 4 | ✔ | | ✔ | Text-as-clip + fill |
| 5 | | ✔ | ✔ | |
| 6 | ✔ | ✔ | ✔ | |
| 7 | | | ✔ | Text used purely as a clipping path (e.g. image showing through letters) |

Clipping modes accumulate glyph outlines across the whole text object; the clip is applied at `ET`. Mode 3 glyphs are still "shown" — they advance the pen and are extracted — which is exactly why OCR tools (OCRmyPDF, Tesseract's PDF renderer) use it to lay a searchable text layer over a scanned image. Type 3 glyphs are painted by their own content-stream operators, so the fill/stroke distinction does not apply: per §9.3.6, "only a value of 3 for text rendering mode shall have any effect on text displayed in a Type 3 font" — modes 1, 2 and 4–7 are effectively mode 0 for Type 3 text. Clipping modes produce no clip if the only glyphs shown have no outlines (e.g. spaces).

### 2.6 Matrices and glyph displacement math

Text space → user space is controlled by the **text rendering matrix**:

```
          ┌ Tfs·Th   0      0 ┐
  Trm  =  │ 0        Tfs    0 │ × Tm × CTM
          └ 0        Trise  1 ┘
```

Glyph outlines are defined in **glyph space** and mapped to text space by the font matrix (`[0.001 0 0 0.001 0 0]` for everything except Type 3, which supplies its own `/FontMatrix`). Hence widths in PDF are expressed in **thousandths of text-space units** (1/1000 em).

After each glyph, `Tm` is updated by `[1 0 0 1 tx ty] × Tm` where (§9.4.4, verbatim from ISO 32000-1), for **horizontal** writing:

```
tx = ((w0 − Tj/1000) · Tfs + Tc + Tw) · Th
ty = 0
```

and for **vertical** writing (`Identity-V` etc.):

```
tx = 0
ty = (w1 − Tj/1000) · Tfs + Tc + Tw
```

where `w0`/`w1` are the glyph's horizontal/vertical displacement in glyph-space/1000 (from `/Widths`, `/W`, `/W2`, `/DW`, `/DW2`, or the font program if missing), `Tj` is the TJ adjustment preceding that glyph (0 for `Tj`), and `Tw` applies only if the code is single-byte 32.

**Important:** the advance width that drives layout comes from the PDF dictionary (`/Widths`, `/W`), **not** from the embedded font's `hmtx`. ISO 32000-1 requires the two to be consistent (§9.6.2.1 Table 111 `Widths`; §9.7.4.3 for CIDFonts); when they disagree in practice, renderers use the PDF value (and PDF/A validators flag the mismatch). Your writer must take widths from the same source the shaper used.

**Worked check from the example in §10.** `/F1 24 Tf` (Helvetica, W=944, A=V=E=667), `[(W) 120 (AVE) -250 (kerned via TJ)] TJ` starting at x=72:

```
W     : 72 + 0.944·24              = 94.656
kern  : −120/1000·24 = −2.88       → 91.776
AVE   : + (0.667·3)·24 = 48.024    → 139.800   (pdftotext -bbox: "WAVE" xMax=139.800 ✔)
gap   : −(−250)/1000·24 = +6.0     → 145.800   (pdftotext -bbox: "kerned" xMin=145.800 ✔)
```

Poppler's `pdftotext` turned the 6 pt gap into a space; Poppler's structure-tree text dump of the same content did **not** (§10.4) — a live demonstration of why producers should emit real space characters.

### 2.7 Kerning and positioning via TJ

Shaping engines output per-glyph advances and x/y offsets. Map them to PDF as:

- **x-advance difference** (shaped advance − PDF width) → a `TJ` number: `adj = −(shaped_adv − width) × 1000 / Tfs` … in practice compute everything in font units and convert: `adj = (width_1000 − shaped_adv_units × 1000/upem)`.
- **x-offset** (mark positioning, GPOS) → split the run: emit a `TJ` number before the glyph and the opposite after it, or start a new run with `Td`/`Tm`.
- **y-offset** (marks stacked above bases, Arabic cursive attachment) → `Ts` (rise) for that glyph, or a new `Tm`. TJ numbers only move along the writing direction.

---

## 3. Font types (ISO 32000-2 §9.5–9.7)

```
                      ┌──────────── Simple fonts (1-byte codes, ≤256 glyphs/encoding) ────────────┐
 /Type /Font ─┬─ /Subtype /Type1        Type 1 (PFB/PFA) or CFF (FontFile3 /Type1C or /OpenType)
              ├─ /Subtype /MMType1      Multiple Master instance (legacy, essentially extinct)
              ├─ /Subtype /TrueType     TrueType outlines (FontFile2) or OpenType (FontFile3 /OpenType)
              ├─ /Subtype /Type3        glyphs are PDF content streams (CharProcs)
              │
              └─ /Subtype /Type0  ─── composite font: CMap maps 1–4-byte codes → CIDs
                       │
                       └─ /DescendantFonts [ CIDFont ]
                              ├─ /CIDFontType0   CFF outlines (FontFile3 /CIDFontType0C or /OpenType)
                              └─ /CIDFontType2   TrueType outlines (FontFile2 or FontFile3 /OpenType)
```

### 3.1 Type 1 and the standard 14 fonts

Type 1 is Adobe's 1985 PostScript outline format (cubic Béziers, hinting via hints/`OtherSubrs`, `eexec`-encrypted charstrings). Embedded as `/FontFile` with `/Length1` (clear-text part), `/Length2` (encrypted binary), `/Length3` (trailing zeros). Adobe ended Type 1 support in its authoring apps in 2023; for new work, Type 1 is only relevant as a **parsing** target. CFF ("Compact Font Format", `Type1C`) is its compact successor and remains current.

**Standard 14 fonts**: Times (4 styles), Helvetica (4), Courier (4), Symbol, ZapfDingbats. Historically, every viewer was required to have them, so they could be referenced by name without embedding or metrics. The special treatment has been **deprecated since PDF 1.5** (ISO 32000-1 §9.6.2.1 Table 111: "Conforming writers should represent all fonts using a complete font descriptor. For backwards capability, conforming readers shall still provide the special treatment"; §9.6.2.2: the fonts "or their font metrics and suitable substitution fonts, shall be available to the conforming reader"). PDF 2.0 is understood to keep this reader obligation (not re-checked against the 2.0 text). In practice:

- Unembedded rendering uses a substitute (Liberation, Nimbus, URW, Foxit's built-ins, Adobe's own) — metrics usually match, shapes don't.
- PDF/A (all parts) and PDF/UA require **all** fonts used for rendering to be embedded, including the standard 14 — except fonts used only in invisible mode 3 text (PDF/A relaxes embedding for mode 3).
- Their built-in encodings cover only Latin-1-ish repertoires; they are useless for most of the world's text.

**lamina guidance:** support standard 14 by name for small, Latin-only "quick" documents (no font file needed — handy for tests), but ship bundled AFM metrics so widths are always written; default to embedding a real font, and refuse non-embedded fonts when a PDF/A or PDF/UA conformance target is set.

### 3.2 Multiple Master (`MMType1`)

Pre-OpenType variable fonts (axes like weight/width). A PDF can only reference a specific *instance* (`/BaseFont /MinionMM_367_400_12_`). Obsolete; parse, never write. Its modern successor — OpenType Font Variations — has no PDF representation at all (§6.8).

### 3.3 TrueType (simple)

`/Subtype /TrueType`, program in `/FontFile2` (raw `sfnt`). The glyph lookup rules for simple TrueType fonts (§9.6.5.4 in PDF 2.0; §9.6.6.4 in ISO 32000-1) are notoriously messy because TrueType fonts identify glyphs via their internal `cmap`, not PDF glyph names:

- **Non-symbolic** font with `/Encoding` (WinAnsi/MacRoman/Differences): code → glyph name → Unicode via AGL → lookup in the (3,1) Microsoft Unicode `cmap`; else via (1,0) Mac Roman `cmap`; else `post` table names.
- **Symbolic** font: code `c` → lookup in (3,0) Microsoft Symbol `cmap` at `c`, then `0xF000+c`, `0xF100+c`, `0xF200+c`; else (1,0) at `c`.

Real-world renderers (pdf.js, MuPDF, PDFium) layer further heuristics on top. **Writers should avoid all ambiguity** by using Type 0/CIDFontType2 with `CIDToGIDMap`, where the GID is addressed directly. ISO 32000-1 §9.9 says as much for OpenType/`glyf` fonts: because the simple-font process "sometimes produces ambiguous results, conforming writers, instead of using a simple font, shall use a Type 0 font with an Identity-H encoding and use the glyph indices as character codes".

### 3.4 Type 3

Glyphs are arbitrary PDF content streams in `/CharProcs`, selected by name through `/Encoding` (which is **required** for Type 3 and must use `/Differences`). Each glyph starts with `d0` (glyph sets its own colour) or `d1` (uncoloured: shape-only, coloured by current fill like a stencil mask). Type 3 has its own `/FontMatrix` (often `[0.001 0 0 0.001 0 0]`, but TeX's bitmapped fonts use odd matrices), `/FontBBox`, `/Resources`.

Type 3 is how you put **anything** into a font: bitmap fonts (old TeX PK fonts), colour emoji, glyphs containing images or shadings. Downsides: no hinting, poor anti-aliasing in some renderers, often no `ToUnicode` and nonsense glyph names (`/a1`, `/g37`) → extraction garbage. A Type 3 font's `/Widths` are in glyph space (via `/FontMatrix`), and codes outside `FirstChar`–`LastChar` have width 0 (§9.6.5 Table 112). Its `/FontDescriptor` is required in Tagged PDF documents (Table 112), and `FontFamily`/`FontStretch`/`FontWeight` should be given there (Table 122). PDF/A permits Type 3; PDF/UA requires `ToUnicode` or meaningful mapping as for any font.

### 3.5 Type 0 (composite) fonts and CIDFonts

A Type 0 font has a **CMap** (`/Encoding`) mapping multi-byte codes → **CIDs** (Character IDs), and exactly one **descendant CIDFont** mapping CIDs → glyphs. This is the only way to address > 256 glyphs from one font resource.

- **CIDFontType0** — CFF outlines. If the CFF is CID-keyed, its `charset` maps CID → GID. If it is a plain (name-keyed) CFF, CID = GID.
- **CIDFontType2** — TrueType outlines. `/CIDToGIDMap` is either `/Identity` (CID = GID; the default) or a stream of 2-byte big-endian GIDs indexed by CID. It may appear only when the TrueType program is embedded; a non-embedded CIDFontType2 has no `CIDToGIDMap` and may only be used with predefined CMaps (§9.7.4.2, Table 117).
- CIDFont dictionaries carry `/CIDSystemInfo` (`Registry`, `Ordering`, `Supplement` — e.g. `Adobe-Japan1-7`, or `Adobe-Identity-0` for an identity mapping), `/DW` (default width, 1000), `/W` (widths), and for vertical writing `/DW2` (default `[880 −1000]`) and `/W2`.
- The CIDSystemInfo of the CMap and the CIDFont must be compatible (same `Registry` and `Ordering`; `Supplement` is not used for compatibility — §9.7.3); `Identity-H`/`Identity-V` work with any. **`Identity-H`/`Identity-V` shall not be used with a non-embedded font** (§9.7.5.2).

The **`/W` array** has two forms, freely mixed (§9.7.4.3; the example below is the spec's own):

```
/W [ 120 [400 325 500]        % CIDs 120,121,122 have widths 400,325,500
     7080 8032 1000 ]         % CIDs 7080..8032 all have width 1000
```

Emit runs of consecutive GIDs as `c [w…]`, long runs of equal width as `c1 c2 w`, and omit glyphs equal to `/DW`.

### 3.6 OpenType

OpenType = `sfnt` container with either TrueType (`glyf`) or CFF (`CFF `/`CFF2`) outlines plus layout tables (GSUB, GPOS, GDEF…). PDF 1.6+ allows embedding the whole OpenType file as `/FontFile3 /Subtype /OpenType` under any of Type1, TrueType, CIDFontType0 or CIDFontType2 dictionaries (matching outline flavour). Common practice for writers:

| Source font | Recommended embedding |
|---|---|
| TrueType outlines (`.ttf`, `.otf` with `glyf`) | Type 0 → CIDFontType2, `/FontFile2` (a stripped `sfnt`), `/CIDToGIDMap /Identity` |
| CFF outlines (`.otf` with `CFF `) | Type 0 → CIDFontType0, `/FontFile3 /Subtype /CIDFontType0C` (bare CFF, converted to CID-keyed) or `/FontFile3 /Subtype /OpenType` |
| CFF2 (variable CFF) | Instantiate and convert to CFF first; CFF2 is not a PDF-recognised font program format |
| Variable TrueType | Instantiate (`fvar`/`gvar` → static) first |

### 3.7 Font descriptors

Required for every font except Type 0 ("Font descriptors shall not be used with Type 0 fonts", §9.8.1 — its descendant CIDFont has one), Type 3 (optional since PDF 1.5, but required in Tagged PDF), and, historically, standard 14. Key entries (§9.8, Tables 122–124):

| Key | Notes |
|---|---|
| `/FontName` | Must equal `/BaseFont` (including subset tag). |
| `/Flags` | Bit field (1-based): 1 FixedPitch, 2 Serif, 3 **Symbolic**, 4 Script, 6 **Nonsymbolic**, 7 Italic, 17 AllCap, 18 SmallCap, 19 ForceBold. Bits are numbered from 1 (low-order); undefined bits shall be 0. Symbolic and Nonsymbolic "shall not both be set or both be clear" (§9.8.2 Table 123). Symbolic = "contains glyphs outside the Adobe standard Latin character set" — it changes TrueType lookup rules (§3.3) and the implicit base encoding. |
| `/FontBBox`, `/ItalicAngle`, `/Ascent`, `/Descent`, `/CapHeight`, `/StemV` | Used for substitution when the font isn't embedded and for some layout (annotation appearance, highlight boxes). In ISO 32000-1 Table 122 all are **required** (embedded or not): `/ItalicAngle` always; `/FontBBox`, `/Ascent`, `/Descent`, `/StemV` except for Type 3; `/CapHeight` for fonts that have Latin characters, except Type 3. |
| `/MissingWidth`, `/XHeight`, `/Leading`, `/AvgWidth`, `/MaxWidth` | Optional. |
| `/FontFile` / `/FontFile2` / `/FontFile3` | Type 1 / TrueType / CFF-or-OpenType program. At most one. |
| `/CharSet` (Type 1) / `/CIDSet` (CIDFont) | List of glyphs in a subset (§9.8.1 Table 122; §9.8.3 Table 124 — `/CIDSet` is a bit table indexed by CID, MSB of byte 0 = CID 0). `/CIDSet` was required by PDF/A-1/2/3 for subsets but is **deprecated in PDF 2.0** (not verifiable from the 1.7 text) and PDF/A-4 no longer requires it; if present it must be correct. |
| `/FontFamily`, `/FontStretch`, `/FontWeight` | Optional (PDF 1.5); should be used for Type 3 fonts in Tagged PDF. |
| `/Style` (containing `/Panose`), `/Lang`, `/FD` | CIDFont-specific extras (Table 124). `/Panose` is not a top-level key: it is the only defined entry of the `/Style` dictionary (§9.8.3.2). |

### 3.8 `/Widths` (simple fonts)

`/FirstChar`, `/LastChar`, `/Widths [ … ]` with `LastChar − FirstChar + 1` entries in 1/1000 em; `/MissingWidth` in the descriptor (default 0) covers codes outside the range (§9.6.2.1 Table 111). For Type 3, widths are in glyph space (scaled by `/FontMatrix`), not 1/1000, and out-of-range codes have width 0 (Table 112).

---

## 4. Encodings (§9.6.5 in PDF 2.0 / §9.6.6 in ISO 32000-1, §9.7.5, Annex D)

### 4.1 Simple-font encodings

A simple font's `/Encoding` maps byte codes → glyph names.

| Encoding | Where | Notes |
|---|---|---|
| `StandardEncoding` | Built-in default of most Type 1 fonts | Adobe's original Latin set; encodes **no** accented letters at all (Annex D.2 shows "—" for every accented letter in the STD column — they exist only as unencoded glyphs, often built with `seac`). "Shall have no special meaning in PDF" except as the implicit base encoding of a non-embedded nonsymbolic font (§9.6.6.1, Table 114). |
| `WinAnsiEncoding` | Named, predefined | ≈ Windows-1252. The pragmatic choice for Latin text with simple fonts. Code 0x27 is `quotesingle`, 0x60 `grave`; 0x80 is `Euro` (Annex D.2). |
| `MacRomanEncoding` | Named, predefined | Classic Mac OS. |
| `MacExpertEncoding` | Named, predefined | Expert sets (small caps, old-style figures). |
| `PDFDocEncoding` | **Not** a font encoding | Used for *text strings* (metadata, bookmarks, annotation contents) — together with UTF-16BE (BOM `FE FF`) and, in PDF 2.0, UTF-8 (BOM `EF BB BF`). Frequently confused with font encodings. |
| Dictionary with `/BaseEncoding` + `/Differences` | | `/Differences [ 128 /Euro /bullet 150 /endash ]` — a code followed by names assigned to successive codes; runs may be in any order but shall not overlap (§9.6.6.1). `BaseEncoding` may only be `MacRomanEncoding`, `MacExpertEncoding` or `WinAnsiEncoding`; if absent the base is the font's built-in encoding (embedded) or `StandardEncoding`/built-in (non-embedded nonsymbolic/symbolic). `Differences` "should not be used with TrueType fonts" (Table 114). Type 3 fonts must use a `Differences` encoding (§9.6.6.3). |
| Font built-in | `Symbol`, `ZapfDingbats`, Type 1 with custom encodings, symbolic TrueType | |

**Glyph names → Unicode** is via the Adobe Glyph List (AGL) and the AGL Specification's algorithm: look up the name (`eacute` → U+00E9); else parse `uniXXXX`, `uniXXXXYYYY…` (multiple BMP code points), `uXXXX[XX]`; strip suffixes after `.` (`a.sc` → `a`); split on `_` for ligatures (`f_f_i` → "ffi"). Extractors use this when `ToUnicode` is missing; arbitrary names (`g37`, `cid12`) defeat it.

A simple font can address at most 256 glyphs per font resource. Writers that use simple fonts for arbitrary text create multiple resources per real font (e.g. old pdfTeX/`dvipdfmx` behaviour, some report generators) — workable but ugly; prefer Type 0.

### 4.2 CMaps

A CMap maps byte sequences to CIDs. It defines **codespace ranges** (how many bytes each code takes — so parsing a string into codes depends on the CMap) and `cidchar`/`cidrange` mappings.

- **Predefined CMaps** (§9.7.5.2): `Identity-H`/`Identity-V` (2-byte code = CID); and Adobe's CJK CMaps by name, e.g. `UniJIS-UTF16-H`, `90ms-RKSJ-H`, `UniGB-UCS2-H`, `GBK-EUC-H`, `UniCNS-UTF16-H`, `UniKS-UTF16-H`, tied to the Adobe-Japan1 / GB1 / CNS1 / Korea1 character collections. Readers need the CMap resource files (Adobe's `cmap-resources` repo; pdf.js and Poppler ship them).
- **Embedded CMaps**: a stream with `/Type /CMap`, `/CMapName`, `/CIDSystemInfo`, optional `/WMode` and `/UseCMap`, containing PostScript-like CMap syntax (Adobe Tech Note #5014; §9.7.5.3 Table 120). Codes are 1–4 bytes; codespace ranges shall not overlap (§9.7.6.2). `bfchar`/`bfrange` shall not appear in a CMap used as a Type 0 `/Encoding` (only in ToUnicode CMaps).
- `-H` vs `-V` selects horizontal vs vertical writing mode (`/WMode`).

### 4.3 CIDs vs GIDs vs Unicode code points

Three different numbers, constantly confused:

| Number | Defined by | Used for |
|---|---|---|
| **Character code** | Bytes in the content stream, parsed by the CMap codespace | Input to the font |
| **CID** | The character collection (e.g. Adobe-Japan1 CID 843) or, with Identity, the code itself | Index into `/W`, `CIDToGIDMap`, CFF charset |
| **GID** | The font file (index into `glyf`/`loca` or CFF CharStrings) | Rendering |
| **Unicode** | `ToUnicode` CMap (or heuristics) | Extraction only |

With the de facto writer configuration (`Identity-H` + CIDFontType2 + `CIDToGIDMap /Identity`), **code = CID = GID**. Unicode is not involved at all in rendering — which is why `ToUnicode` is mandatory in practice.

### 4.4 `CIDToGIDMap`

Only for CIDFontType2, and only when its TrueType program is embedded (§9.7.4.2 Table 117). `/Identity` (the default) or a stream of `uint16` big-endian GIDs (CID n's GID at bytes 2n, 2n+1). A custom map lets a writer keep stable CIDs while subsetting renumbers GIDs (`retain_gids`-free compaction), or keep original GIDs as CIDs and still emit a compact font. Simplest correct approach: subset with compaction, then encode the **new** GIDs directly (`/Identity`) — what the §10 example does.

---

## 5. ToUnicode CMaps and ActualText (§9.10)

### 5.1 Why

Extractors resolve Unicode in this priority order (§9.10.2): (1) `ToUnicode` CMap; (2) simple fonts that use `MacRomanEncoding`, `MacExpertEncoding` or `WinAnsiEncoding`, or a `Differences` array containing **only** names from the Adobe standard Latin character set and the Symbol font (code → name via Annex D → Unicode via the AGL); (3) composite fonts using a predefined CMap from Table 118 **other than `Identity-H`/`Identity-V`**, or whose descendant uses Adobe-GB1/CNS1/Japan1/Korea1 (via the `Adobe-*-UCS2` CMaps). If these fail, "there is no way to determine what the character code represents": guesswork, the embedded font's own `cmap` reversed, or OCR. **With `Identity-H` and an `Adobe-Identity` (or other non-registered) collection there is no fallback** — no `ToUnicode` means no text. Tagged PDF already requires every character code to be mappable by one of these methods (§14.8.2.4.2); PDF/UA requires the same; PDF/A-2/3 level U and PDF/A-4 likewise.

### 5.2 Syntax

```
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>                      % must match the font's code lengths
endcodespacerange
3 beginbfchar
<0003> <0020>                      % code 3 → U+0020 SPACE
<0024> <00660069>                  % ligature glyph → "fi" (two UTF-16 units)
<0051> <D835DC00>                  % code → U+1D400 (surrogate pair, UTF-16BE)
endbfchar
2 beginbfrange
<0010> <001F> <0041>               % 0x10..0x1F → U+0041..U+0050 (incrementing)
<0030> <0032> [<0066> <00660066> <00660066006C>]   % explicit list form
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
```

Rules writers trip over:

- Destination strings are **UTF-16BE** hex (§9.10.3: mappings "to Unicode character sequences expressed in UTF-16BE encoding"); supplementary-plane characters use surrogate pairs (spec example `<3A51> <D840DC3E>` = U+2003E; TN #5411 §1.5); a destination may contain **several** code points — up to 512 bytes (§9.10.3; the spec's own example maps ligature glyphs to `<00660066>`, `<00660069>`, `<00660066006C>`) — for ligatures, decomposed sequences, Indic conjuncts. (Very old consumers — Acrobat 4/5 — accepted only UCS-2, i.e. no surrogates; TN #5411 §1.2.)
- In the incrementing `bfrange` form, all source bytes except the last must be identical — TN #5411 §1.4.1 calls this not crossing "first-byte boundaries" and names it "the most common problem" (`<10FE> <1101>` is invalid; split as `<10FE> <10FF>` + `<1100> <1101>`). Only the last byte of the destination is incremented, and it must not pass 255: its value "shall be less than or equal to 255 − (srcCode2 − srcCode1)", otherwise the result is undefined (§9.10.3). The array form `<lo> <hi> [dst1 … dstm]` has no such destination constraint.
- Each `begin…` block may contain at most **100** entries: "101 beginbfchar" and "101 beginbfrange" are invalid (TN #5411 §1.4.1, "as with CMap files", i.e. TN #5014). Split larger lists into multiple blocks. (Real bug: OCRmyPDF 17.12 fixed "CMap block splitting" in its new renderer.)
- The codespace must be consistent with the font's encoding (§9.10.3): 2 bytes (`<0000> <FFFF>`) for Identity-H; for a simple font it "shall be one byte long" — use `<00> <FF>`. In the ToUnicode CMap stream dictionary only `UseCMap` is pertinent.
- TN #5411 §1.4.1 recommends mapping a code that has no Unicode equivalent to `<FFFD>` rather than omitting it (written for whole-collection ToUnicode files; for subset fonts, omitting unused codes is fine).
- One code → one Unicode string. If the same glyph is reached from different characters (e.g. U+0041 and U+0391 drawn with one glyph, or a space glyph used for NBSP), the ToUnicode can record only one — use `ActualText` for the other, or duplicate the glyph under a different code/CID (possible with a custom `CIDToGIDMap`: two CIDs → one GID).

### 5.3 Ligatures, reordering and ActualText

Shaping breaks the assumption that glyph order = character order:

| Phenomenon | Glyphs | Characters | ToUnicode alone |
|---|---|---|---|
| Latin ligature | `ﬁ` (1) | f, i (2) | ✔ map to `00660069` |
| Arabic contextual forms | initial/medial/final glyphs | base letters | ✔ map each form to base letter |
| Arabic lam-alef | 1 | 2 | ✔ multi-code-point mapping |
| Devanagari reph / pre-base matra | reordered | logical order differs | ✘ order wrong |
| Bidi (RTL) | visual order in stream | logical order | ✘ extractor must re-run bidi; often wrong |
| One glyph, multiple meanings | 1 | context-dependent | ✘ |
| Hyphenated word broken across lines | "hyphen-" + "ation" | "hyphenation" | ✘ (soft hyphen handling) |

The fix is **`ActualText`** on marked content or a structure element (§14.9.4):

```
/Span << /ActualText (\376\377\011\025\011\077) >> BDC   % BOM + UTF-16BE U+0915 U+093F "कि" (logical order)
  <0456 0123> Tj                                          % glyphs in visual order: i-matra, then KA
EMC
```

`ActualText` on a structure element is PDF 1.4; on a marked-content sequence it is PDF 1.5 and must use the **`Span`** tag (§14.9.4). It is a *character* substitution (consecutive `ActualText` sequences are joined with no word break), unlike `/Alt` and `/E`, which are whole-word substitutions (§14.9.3, §14.9.5). Viewers and extractors that honour it (Acrobat, pdf.js, PDFBox, MuPDF, Poppler partially) substitute the replacement text for everything inside. Related keys: `/Alt` (alternate description, for figures/formulae), `/E` (expansion of abbreviations), `/Lang`. For a hyphen inserted by line-breaking, **Tagged PDF already requires** the incidental hyphen to be represented by a soft-hyphen character that maps to U+00AD, distinct from a hard hyphen U+002D (ISO 32000-1 §14.8.2.2.3) — so map the visible hyphen glyph's code to `<00AD>` (a separate code/CID from a real hyphen). Use `ActualText` when hyphenation changes the spelling (the spec's example: German "Druk-/ker" → `/Span <</ActualText (c)>> BDC (k-) Tj EMC`, §14.9.4).

---

## 6. Font embedding & subsetting from code

### 6.1 The sfnt tables that matter

| Table | Role for a PDF writer |
|---|---|
| `head` | `unitsPerEm` (1000 for CFF, often 2048/1000/1024 for TT), `indexToLocFormat` (0 = short `loca`, 1 = long), global bbox, `checkSumAdjustment`. |
| `hhea` / `hmtx` | Ascender/descender/lineGap; `numberOfHMetrics` and per-glyph advance widths → `/W`/`/Widths`. |
| `vhea` / `vmtx` | Vertical metrics → source for `/W2`/`/DW2`. PDF readers "shall never" use these tables; `DW2`/`W2` are the only way to specify vertical metrics (§9.9). |
| `maxp` | `numGlyphs`; TrueType hinting maxima. |
| `cmap` | Unicode → GID (format 4 BMP, format 12 full Unicode, format 14 variation selectors). Used to map text to glyphs pre-shaping, and to build `ToUnicode` (reverse map; ambiguous for multiply-mapped glyphs). In an embedded CIDFontType2 ISO 32000-1 §9.9 says the `cmap` "is not needed and shall not be present" (required for simple TrueType fonts). |
| `glyf` / `loca` | TrueType outlines; `loca` gives offsets into `glyf`. |
| `CFF ` / `CFF2` | Type 2 charstrings, local/global subroutines, Private DICT, charset (CID-keyed: CID→GID), FDArray/FDSelect. |
| `cvt `, `fpgm`, `prep` | TrueType hinting program; required by the spec for TrueType embedding when present (instructions reference them). |
| `OS/2` | `fsType` (embedding permissions), `sTypoAscender/Descender/LineGap`, `usWinAscent/Descent`, `sxHeight`, `sCapHeight` (v2+), weight/width class, Unicode ranges. |
| `post` | PostScript glyph names (format 2) — useful for simple-font encodings and extraction fallback; italic angle; underline position/thickness; `isFixedPitch`. |
| `name` | Family/subfamily/PostScript name (`nameID 6` → `/BaseFont`), copyright/licence. |
| `GSUB`/`GPOS`/`GDEF`/`morx`/`kerx`/`kern` | Shaping — consumed by the shaper, **dropped** from the embedded subset (PDF renderers never shape). |
| `fvar`/`gvar`/`avar`/`HVAR`/`MVAR`/`CFF2` blend | Variations — must be instantiated away. |
| `COLR`/`CPAL`/`sbix`/`CBDT`/`CBLC`/`SVG ` | Colour glyphs — PDF renderers ignore them; see §7.7. |

The PDF spec (§9.9) lists the TrueType tables that "shall always be present if present in the original": `head`, `hhea`, `loca`, `maxp`, `cvt `, `prep`, `glyf`, `hmtx`, `fpgm`; plus `cmap` for simple TrueType fonts (and, per ISO 32000-1, *no* `cmap` for CIDFonts). For `/FontFile3 /OpenType`, a `glyf` font needs `head`, `hhea`, `hmtx`, `loca`, `maxp` (+ `cvt `/`fpgm`/`prep` if the instructions need them); a CFF-flavoured one needs `CFF ` + `cmap` (Table 126). Everything else may be dropped. Embedding a whole multi-MB CJK font when using 30 glyphs is the most common file-size bug.

### 6.2 Subsetting algorithm

```
input: font, set U of shaped glyph IDs actually painted
1. G := U ∪ {0 /.notdef}
2. closure:
     TrueType glyf: for each composite glyph, add component GIDs (recursively;
                    composite flags MORE_COMPONENTS, ARG_1_AND_2_ARE_WORDS, WE_HAVE_A_SCALE …)
     CFF/Type 1:    `seac`/endchar-with-4-args accent composition → add StandardEncoding base+accent
                    glyphs; subroutine closure (or desubroutinize)
     (if keeping layout tables for some reason: GSUB closure — not needed for PDF)
3. renumber: old GID → new GID (dense) — or keep GIDs ("retain-gids": empty slots, simpler mapping)
4. rewrite: glyf (rewriting component GID references!), loca (pick short/long format),
            hmtx (+ numberOfHMetrics), maxp.numGlyphs, head (bbox, indexToLocFormat, checksum),
            hhea.numberOfHMetrics, post → format 3 (no names), cmap → drop or minimal
   CFF: rebuild CharStrings INDEX, charset, FDSelect, prune/renumber subrs (or inline them),
        convert name-keyed → CID-keyed if embedding as CIDFontType0C
5. emit: /W from new hmtx, ToUnicode from (new GID → Unicode string) recorded during shaping,
         /BaseFont = TAG + "+" + PostScriptName, where TAG = 6 uppercase letters A–Z
```

Real numbers from the §10 example: 32 distinct characters → 37 glyphs in the subset. The extra five are `.notdef` plus four **closure** glyphs: Greek `Epsilon` is a composite referencing Latin `E`; `alphatonos` (ά) composes `alpha` + `tonos`; Cyrillic `а` and `р` are composites referencing Latin `a` and `p`. Omit the closure and those characters render blank or crash renderers. DejaVu Sans went from 6,253 glyphs (~757 KB) to an 8.1 KB subset.

**Subset tags.** `ABCDEF+` — exactly six uppercase letters and a plus, on both `/BaseFont` and the descriptor's `/FontName` (§9.6.4). "Different subsets in the same PDF file shall have different tags" (derive from a hash of the glyph set to be deterministic and reproducible). Extractors and preflight tools use the tag to know a font is partial — so incremental edits that need new glyphs must embed a *new* subset (or, cleverly, append glyphs to an existing subset and rewrite it).

**Retain-GIDs vs compaction.** Keeping original GIDs (with empty glyphs in the gaps) makes the content stream independent of the subset — useful for streaming writers that emit content before the glyph set is known — at a small cost (each empty `loca` entry is 2–4 bytes; `hmtx` still has entries). For CJK fonts with 60k glyphs that's ~250 KB of overhead; compaction plus deferred content-stream emission (or a `CIDToGIDMap` stream) is better.

**Hinting.** Keep `fpgm`/`prep`/`cvt ` (and the per-glyph instructions) for TrueType unless you deliberately strip hints for size; FreeType-based renderers (Poppler, MuPDF, pdf.js in some paths) may use them. Never leave dangling references (e.g. drop `fpgm` while glyphs call its functions).

### 6.3 Licensing: `OS/2.fsType`

| Bits | Meaning | Writer policy |
|---|---|---|
| `0x0000` | Installable embedding | Embed/subset freely. |
| `0x0002` | **Restricted license** — must not be embedded | Refuse (or rasterise/outline only with explicit user opt-in; legal advice says this may still violate the EULA). |
| `0x0004` | Preview & Print | Embed allowed; document should be read-only (some tools refuse editable fields using it). |
| `0x0008` | Editable | Embed allowed. |
| `0x0100` | No subsetting | Embed the **full** font. |
| `0x0200` | Bitmap embedding only | Outlines may not be embedded; effectively unusable for PDF. |

`fsType` is advisory metadata; the actual licence is the EULA. lamina should surface it as an error/warning with an override flag, as Acrobat, LibreOffice and fontTools-based pipelines do.

### 6.4 Libraries (state as of late 2026)

| Library | Lang | What it gives you | Notes |
|---|---|---|---|
| **HarfBuzz** (`hb-shape`, `hb-subset`) | C/C++ | Reference shaper; production subsetter used by Chrome/Google Fonts; `hb-subset` supports retain-gids, desubroutinize, variable-font instancing (`hb_subset_input_pin_axis_location`, partial instancing). | v13.x current. Bindings in nearly every language. |
| **fontTools** (`fontTools.subset`, `ttLib`, `varLib.instancer`, `cffLib`) | Python | Swiss-army knife: read/write every table, subset, instance variable fonts, CFF2→CFF. Used to build the §10 example (4.68.0). | Slow but definitive reference behaviour. |
| **fontations**: `read-fonts`, `write-fonts`, `skrifa` | Rust | Google's zero-copy parser/writer and glyph loader (outlines, hinting, metrics); used in Chrome. | `skrifa` replaces FreeType-ish duties. Subsetting via **`klippa`** (port of hb-subset, in the fontations repo) and **`skera`** (used by HarfRust docs). |
| **HarfRust** | Rust | Pure-Rust port of HarfBuzz shaping on top of `read-fonts`; tracks HarfBuzz 13. | Successor to **rustybuzz**, which is archived. |
| **allsorts** | Rust | Parsing, shaping (OpenType incl. Arabic/Indic), subsetting incl. CFF→CID conversion; built for Prince XML. | Used by `printpdf`. |
| **ttf-parser** / **subsetter** | Rust | Lightweight parser; `subsetter` crate (from Typst) does PDF-oriented TT/CFF subsetting incl. CFF→CID. | Typst's PDF stack (`krilla`) is a good reference design. |
| **fontkit** | JS | Parsing, layout, subsetting (TT and CFF); used by PDFKit and pdf-lib (`@pdf-lib/fontkit`). | Shaping coverage weaker than HarfBuzz; `harfbuzzjs` (WASM) is the robust option in JS. |
| **FreeType** | C | Rasteriser and parser — renderers, not writers. | |

---

## 7. Text layout & shaping — everything PDF does not do for you

A PDF writer that accepts a Unicode string must perform the work a browser does. The canonical pipeline:

```
Unicode text + style runs (font family, size, lang, features)
   │ 1. normalisation (usually none; NFC only if the font lacks precomposed glyphs)
   │ 2. bidi resolution (UAX #9) → embedding levels per char
   │ 3. itemisation: split into runs of (script [UAX #24], bidi level, font, lang, style)
   │ 4. font fallback: for each cluster, choose a font covering it (grapheme-cluster granularity!)
   │ 5. shaping (HarfBuzz): chars → glyph IDs + clusters + advances + offsets
   │ 6. line breaking (UAX #14 opportunities + widths from shaped runs; re-shape at break if needed)
   │ 7. hyphenation (Liang patterns) for candidate words
   │ 8. justification (greedy or Knuth–Plass), letter-/word-spacing, kashida
   │ 9. bidi reordering per line (visual order, L2 rule) 
   │10. emit: per glyph run → Tf, Tm/Td, TJ with adjustments; record GID→Unicode for ToUnicode,
   │         ActualText where clusters aren't 1:1/monotonic; marked content + struct elems
   ▼
PDF content stream + fonts + structure tree
```

### 7.1 Unicode fundamentals

Text is a sequence of code points; user-perceived characters are **grapheme clusters** (UAX #29) — `é` may be 1 or 2 code points; emoji ZWJ sequences may be 7+. Cursor movement, fallback and selection must work on clusters. Normalisation (NFC/NFD) doesn't change rendering in a good shaper (HarfBuzz composes/decomposes using the font's `cmap`), so don't normalise user text destructively — it changes what extraction returns.

### 7.2 Shaping

Shaping maps characters to positioned glyphs using the font's OpenType layout tables (GSUB substitutions: ligatures, contextual alternates, small caps; GPOS positioning: kerning, mark attachment, cursive attachment) and script-specific logic (the "complex shapers"):

- **Arabic/Syriac/N'Ko/Mongolian**: joining forms (isol/init/medi/fina) chosen per character from Unicode Joining_Type; mandatory ligatures (lam-alef); mark positioning; kashida (tatweel) for justification.
- **Indic (Devanagari, Bengali, Tamil, …) and the Universal Shaping Engine (USE)** for many South/Southeast Asian scripts: syllable clustering, reordering (pre-base matras drawn left of consonant, reph moved to end), conjunct formation via `half`/`pres`/`blws` features.
- **Thai/Lao/Khmer/Myanmar**: mark stacking; **Thai/Lao/Khmer/Myanmar need dictionary-based word segmentation for line breaking** (no spaces between words; ICU or libthai-style dictionaries).
- **Hangul**: jamo composition. **CJK**: mostly 1:1 but vertical alternates (`vert`/`vrt2`), `palt` proportional metrics.

HarfBuzz output per glyph: `glyph_id`, `cluster` (index of the first source character), `x_advance`, `y_advance`, `x_offset`, `y_offset` in font units (scale by `size/upem`). **The `cluster` values are your ToUnicode/ActualText source of truth**: a cluster mapping 1 glyph ↔ n chars becomes a multi-code-point ToUnicode entry; n glyphs ↔ 1 cluster (decomposition) map the first glyph to the text and the rest to empty — or better, wrap the cluster in `ActualText`; non-monotonic clusters (reordering) require `ActualText`.

Do **not** shape with features you can't round-trip unless you record the mapping: OCRmyPDF 17.12 had to *disable* optional Latin ligatures in its fpdf2/HarfBuzz text layer because the ligature glyphs broke downstream copy/paste via Ghostscript.

### 7.3 Bidirectional text (UAX #9)

Resolve paragraph direction, compute embedding levels (explicit isolates `U+2066–2069`, numbers in RTL, mirrored brackets `U+0028 → )` via Bidi_Mirroring_Glyph), then after line breaking reorder each line's runs into visual order (rule L2) and shape each run in its direction. PDF has no bidi concept: content streams are painted in visual order left-to-right with absolute positions. (The one hook is Tagged PDF's `/ReversedChars` marked-content tag, which declares that each show string's codes are in reverse of logical order; such strings may have a leading/trailing space but no interior spaces — ISO 32000-1 §14.8.2.3.3. Little software emits or honours it.) For extraction, the producer should declare logical order: the structure tree content order + `ActualText` (or simply emit glyph runs so that an extractor running bidi on its own recovers logical order — this works for simple cases but nested numbers/brackets break). Libraries: ICU, FriBidi, `unicode-bidi` (Rust), `bidi-js`.

### 7.4 Line breaking (UAX #14)

UAX #14 classifies characters (BK, CR, LF, SP, BA, AL, ID, NU, OP, CL, QU, GL, ZW, ZWJ, RI, EB/EM…) and gives pair rules yielding mandatory breaks, opportunities and prohibitions (no break before `!` or closing punctuation; no break inside numbers like `1,000.00`; CJK kinsoku rules depend on strictness). Implementations: ICU `BreakIterator`, `unicode-linebreak` / `icu_segmenter` (Rust; the latter includes dictionary/LSTM models for Thai, Lao, Khmer, Burmese), `linebreak` (JS). Breaking must be done on characters but **measured** with shaped widths; when a break falls inside a shaped run, re-shape both halves (ligatures and kerning across the break change; Arabic joining changes at line ends).

### 7.5 Hyphenation (Knuth–Liang)

Frank Liang's 1983 algorithm (TeX's): a trie of patterns like `.hy3p`, `hen5at`, `4m1p`; for each word, apply all matching patterns, take the max digit between letters, odd = break allowed. Pattern sets for ~80 languages come from the TeX `hyph-utf8` project (various licences — many LPPL/MIT) and are reused by LibreOffice (libhyphen), Typst, and browsers. Respect language (`/Lang`), minimum left/right fragment lengths (e.g. 2/3 for English), soft hyphens (U+00AD) in input as explicit opportunities, and emit the visible hyphen with extraction metadata (§5.3).

### 7.6 Justification

- **Greedy (first-fit):** fill each line until the next word overflows. O(n), streaming-friendly, used by browsers and Word. Produces "rivers" and loose lines.
- **Knuth–Plass (1981) total-fit:** model the paragraph as boxes/glue/penalties; dynamic programming over feasible breakpoints minimising total demerits (badness from glue stretch/shrink ratio, penalties for hyphens, consecutive hyphens, fitness-class changes between adjacent lines). Typical implementation with an active-node list is ~O(n·w) where w is the window of feasible breakpoints. Used by TeX, Typst, InDesign's paragraph composer, Prince. Requires the whole paragraph before output — fine for a document generator.
- Implementing justification in PDF: adjust inter-word gaps — `Tw` only works with single-byte code 32 (simple fonts), so for Type 0 fonts insert per-space `TJ` adjustments; optionally small `Tc` letter-spacing or `Tz` glyph scaling (InDesign-style ±2–3%), and **kashida** elongation for Arabic (insert tatweel glyphs or use `jalt` alternates — never letter-space Arabic).

### 7.7 Font fallback

No font covers all of Unicode. Fallback chooses a font per grapheme cluster from a prioritised list (user stack → language-specific defaults e.g. Noto Sans JP vs SC vs KR for unified Han — **the `lang` determines correct Han glyphs** → last resort font). Check coverage with `cmap` *and* that shaping didn't produce `.notdef`; split runs at font boundaries; keep combining marks with their base's font. System font discovery: fontconfig (Linux), CoreText (macOS), DirectWrite (Windows); `fontdb` (Rust) for a portable in-process index. OCRmyPDF's 2025–26 switch to per-language Noto families and a glyphless fallback font illustrates the trade-offs: a glyphless fallback keeps text searchable but "appears blank when highlighted" and loses characters in PDFium-based viewers.

### 7.8 Emoji and colour fonts

PDF renderers draw font outlines in one colour — they never read `COLR`/`CPAL`/`sbix`/`CBDT`/`SVG ` tables. Options for a writer:

| Colour font format | Mapping to PDF |
|---|---|
| **COLR v0** (layered outlines + palette) | Emit each layer as a separate glyph from the same font with its own fill colour, overprinted at the same position; or a Type 3 font whose CharProcs contain the coloured layers (`d0`). |
| **COLR v1** (gradients, transforms, compositing) | Type 3 glyph procedures with PDF shadings (axial/radial/sweep → function-based shading or mesh approximation), clipping and transparency groups (blend modes map to PDF blend modes). Doable; Typst/krilla do this. |
| **sbix / CBDT** (embedded PNG bitmaps) | Type 3 glyph whose proc paints an image XObject (or inline image); or place an image directly with an `ActualText` span. |
| **SVG-in-OpenType** | Convert SVG to PDF drawing operators (needs an SVG renderer, e.g. resvg/usvg → PDF), wrap in Type 3. |

Always attach `ToUnicode` to such Type 3 fonts (emoji ZWJ sequences → multi-code-point mapping), or wrap each emoji in `ActualText`. Type 3 renders without hinting/anti-aliasing finesse in some viewers; bitmaps don't scale. Monochrome fallback (Noto Emoji outlines) is a valid, simpler option.

### 7.9 Variable fonts

PDF has no notion of variation axes. Before embedding, **instantiate** at the requested axis coordinates: `fontTools.varLib.instancer` (`instantiateVariableFont(font, {"wght": 650})`) or `hb-subset --instance` / `hb_subset_input_pin_axis_location`. Then subset (HarfBuzz can do both in one step). Name the instance distinctly (`Inter-SemiBold_650`, plus subset tag) since several instances of one family may coexist. Shape with the variable font at the same coordinates (HarfBuzz `hb_font_set_variations`) so advances match the instance. CFF2 must be converted to CFF (`fontTools.cffLib.CFF2ToCFF.convertCFF2ToCFF` after instancing, or HarfBuzz instancing to TrueType) — or converted to TrueType outlines (`cu2qu` loses exactness; prefer CFF).

---

## 8. Text extraction

### 8.1 How extractors work

All extractors do roughly the same three steps; quality lies in the heuristics.

```
content stream interpretation ──► glyph list: (unicode?, font, size, Trm, bbox, advance, render mode, colour, MCID)
        │
        ▼
  word/line building: sort/cluster glyphs into lines (baseline/orientation), split words at gaps
        │
        ▼
  block/column building + reading order: group lines into blocks, order blocks (XY-cut, column detection,
  or the structure tree if present and trusted)
```

**Unicode recovery** (in order): ToUnicode → standard encoding + AGL glyph names → predefined CMap + UCS2 CMaps → font `cmap` reverse lookup → glyph-shape hashing against known fonts (some commercial tools) → OCR of glyph images (MuPDF/PyMuPDF and Acrobat can OCR "unmappable" glyphs). Garbage typically comes from Type 3 fonts with names like `/g12`, subset TrueType fonts without ToUnicode, and fonts deliberately obfuscated (shuffled encodings, a known anti-scraping trick).

**Space detection.** A gap between consecutive glyphs larger than a threshold (typically 0.1–0.3 × font size, or relative to the space glyph's width or average char width) becomes a space. Negative gaps or large jumps indicate new lines or out-of-order painting. Duplicate glyphs (fake bold by painting twice with offset — common in older generators) are de-duplicated by position overlap.

**Lines.** Group glyphs whose baselines are within tolerance and whose orientation (from `Trm`) matches; handle rotated text (90°/270°), vertical CJK (`WMode 1`), superscripts (`Ts`) — which should *not* start new lines.

**Hyphen and ligature handling.** De-hyphenate words broken at line end (`-` U+002D at the end followed by lowercase on next line — risky for genuine hyphenated compounds; soft hyphen U+00AD is unambiguous). Decompose presentation-form ligatures (U+FB00–FB06) via NFKC if requested.

**Reading order.** XY-cut (recursive splitting along whitespace gaps), column detection via left-edge histograms, header/footer removal by repetition across pages. If the PDF is tagged (and the tags are trustworthy), the structure tree gives the author's reading order directly.

### 8.2 The main engines

| Engine | Lang / licence | Approach & knobs |
|---|---|---|
| **Poppler `pdftotext`** (`TextOutputDev`) | C++ / GPL | Builds words → lines → blocks → flows; `-layout` keeps physical layout with spaces, `-raw` keeps content-stream order, `-bbox`/`-bbox-layout` emit coordinates; `pdfinfo -struct-text` dumps tagged text. |
| **MuPDF** (`mutool draw -F text/stext.json`, PyMuPDF `page.get_text()`) | C / AGPL (commercial) | "Structured text" device: blocks/lines/spans/chars with fonts, bboxes, flags; options for dehyphenation, ligature preservation, whitespace handling, clipping; `pymupdf4llm` produces Markdown; built-in OCR via Tesseract for pages/unmappable glyphs; table finder (`page.find_tables()`). Fast. |
| **pdf.js** (`getTextContent()`) | JS / Apache-2.0 | Emits text items (string, transform, width, height, fontName, `hasEOL`) for the transparent text layer overlay used for selection/search; joins chars to items, inserts spaces heuristically; supports marked content (`includeMarkedContent`) and struct tree for accessibility. |
| **Apache PDFBox** `PDFTextStripper` | Java / Apache-2.0 | `setSortByPosition`, thresholds for spacing tolerance / average-char tolerance, paragraph start/end detection, bead (article thread) support; `PDFTextStripperByArea`. Basis of Tabula. |
| **pdfminer.six** | Python / MIT | Explicit layout analysis with `LAParams`: `char_margin` (2.0), `word_margin` (0.1), `line_margin` (0.5), `line_overlap` (0.5), `boxes_flow` (0.5; −1 horizontal… +1 vertical ordering, or `None` to disable), `detect_vertical`. Pure Python, slow, very inspectable. |
| **pdfplumber** | Python / MIT | On top of pdfminer: char-level objects with full attributes, visual debugging, `extract_words` tolerances (`x_tolerance`, `y_tolerance`), and a lines/rects-based **table extractor** (strategies: `lines`, `text`, explicit). |
| **pypdf** | Python / BSD | Simple content-stream walker with layout mode; fine for clean PDFs. |
| **PDFium** (`FPDFText_*`) | C++ / BSD | Chrome's engine; char-level API with bounds and generated spaces/newlines. |

Typical failure modes to test against: Type 3 fonts without ToUnicode; text drawn char-by-char with individual `Tm`s (every char is its own "run"); rotated pages (`/Rotate`) vs rotated text; text clipped or covered by images (hidden, but extracted — PyMuPDF can drop clipped text); white text on white; invisible (mode 3) OCR layers duplicated under real text; mis-ordered multi-column pages; tables.

### 8.3 Scanned PDFs and OCR

A scanned page is an image XObject; text extraction returns nothing. OCR adds a **text layer**: for each recognised word, an invisible (`3 Tr`) glyph run positioned and horizontally scaled (`Tz`) to match the word's bounding box, usually with a "glyphless" font (each glyph an empty outline with uniform width) or an actual Unicode font.

- **Tesseract 5.x** (LSTM engine; Apache-2.0): its PDF renderer emits an invisible text layer using "GlyphLessFont": a Type 0 / Identity-H font whose 2-byte codes are simply the UTF-16 code units of the recognised text, a `CIDToGIDMap` that sends every CID to one blank glyph, and an identity `ToUnicode` — the text is encoded directly, and `Tz` stretches each word to its box. hOCR/ALTO/TSV outputs give word boxes for custom renderers.
- **OCRmyPDF** (MPL-2.0; v17.13 as of Sept 2026): orchestrates preprocessing (deskew, rotate, clean), Tesseract, text-layer grafting onto the original page (`--mode`/`--redo-ocr`/`--force-ocr` semantics for already-text pages), PDF/A output via Ghostscript, optimisation. The v17 line renders text layers through an fpdf2-based renderer with HarfBuzz shaping and per-script Noto font discovery, falling back to a glyphless font.
- Other OCR: PaddleOCR (incl. PaddleOCR-VL, a 0.9B document VLM), docTR, EasyOCR, Apple Vision / Windows OCR / cloud APIs (Google Document AI, Azure Document Intelligence, AWS Textract).

### 8.4 Table extraction

Tables in PDF are just text and lines. Approaches:

- **Rule-based "lattice"**: detect ruling lines (paths/rects), compute cell grid, assign chars to cells — Tabula (`lattice` mode), Camelot (`lattice`), pdfplumber (`lines` strategy), PyMuPDF `find_tables`.
- **Whitespace "stream"**: infer columns from alignment gaps — Tabula/Camelot `stream`, pdfplumber `text` strategy.
- **ML**: table detection + structure recognition models — Microsoft Table Transformer (TATR), IBM TableFormer (used in Docling), and end-to-end VLMs that emit HTML/Markdown tables.

### 8.5 Layout analysis and ML document understanding (state of the art, 2026)

The "PDF → structured Markdown/JSON for LLMs/RAG" use case has driven a wave of tools combining a programmatic text layer (when present) with vision models:

| Tool | Approach (2026) |
|---|---|
| **Docling** (IBM → LF AI & Data; MIT) | Pipeline: PDF parsing backend (docling-parse) + layout model (the "Heron" layout model is the default in recent 2.x releases) + TableFormer for tables + optional OCR; or the **VLM pipeline** with **Granite-Docling-258M** emitting DocTags. Outputs DoclingDocument JSON/Markdown/HTML; MCP server. |
| **Marker** (Datalab; GPL code, weights under a revenue-gated licence) | Surya OCR/layout + heuristics + optional LLM post-processing; **Marker 2** (mid-2026) reports 76.0 on olmOCR-bench at ~2.9 pages/s (vendor-run numbers). |
| **MinerU** (OpenDataLab) | Pipeline backend and a VLM backend; strong on Chinese/academic docs and formulas. |
| **olmOCR** (Ai2; Apache-2.0) | Fine-tuned VLM that reads page images *plus* anchor text extracted from the PDF; introduced **olmOCR-bench** (~1,400 PDFs, thousands of pass/fail unit tests), now the de facto open benchmark. |
| **Nougat** (Meta, 2023) | Encoder–decoder transformer turning academic pages into Mathpix-Markdown/LaTeX; historically important, superseded. |
| **PaddleOCR-VL**, **GOT-OCR 2.0**, **dots.ocr**, general VLMs (GPT/Claude/Gemini/Qwen-VL) | End-to-end page image → Markdown/HTML; best on messy layouts, slowest and costliest, can hallucinate. |
| **pymupdf4llm**, **LiteParse**-style fast parsers | Pure text-layer heuristics → Markdown; fast, no GPU, fail on scans and complex layouts. |

Practical consensus: use the native text layer when it exists and is trustworthy (exact characters, free), use vision models for layout, reading order, tables and formulas, and OCR/VLMs only where the text layer is missing or garbage. Benchmarks (olmOCR-bench, OmniDocBench, READoc) vary a lot by document type; evaluate on your own corpus.

**Implication for lamina:** the reader side should expose a rich, lossless **glyph-level model** (Unicode + GID + font + Trm + bbox + render mode + colour + clip + MCID + struct path) from which word/line heuristics, tagged-reading-order extraction, and ML pipelines can all be built — not just a `get_text() -> str`.

---

## 9. Tagged PDF & logical structure (§14.7–14.9)

### 9.1 The model

```
Catalog
 ├─ /MarkInfo << /Marked true >>          (PDF 2.0: Suspects deprecated)
 ├─ /Lang (en-US)                         (default natural language; RFC 3066 tag in 1.7, BCP 47 in 2.0)
 ├─ /ViewerPreferences << /DisplayDocTitle true >>
 └─ /StructTreeRoot
       ├─ /K → StructElem /Document  (exactly one child in Tagged PDF, §14.8.4.2; PDF 2.0 also allows /DocumentFragment)
       │        └─ /K [ StructElem /H1, StructElem /P, StructElem /Figure /Alt (…), StructElem /Table … ]
       │                 └─ /K  MCID integer │ MCR dict << /Type /MCR /Pg … /MCID n >>
       │                                     │ OBJR dict << /Type /OBJR /Obj annot >> │ child StructElems
       ├─ /ParentTree  number tree: page /StructParents key → [ StructElem for MCID 0, 1, 2 … ]
       │                            annotation /StructParent key → StructElem
       ├─ /ParentTreeNextKey
       ├─ /RoleMap   << /MyHeading /H1 >>     (custom → standard types)
       ├─ /Namespaces [ … ]                   (PDF 2.0)
       └─ /IDTree, /ClassMap; /PronunciationLexicon, /AF (PDF 2.0 only)
```

Content is linked to structure through **marked content**: in the page content stream, `/P <</MCID 0>> BDC … EMC` brackets the operators belonging to marked-content ID 0 of that page. The structure element's `/K` references `(page, MCID)`; the page's `/StructParents` key indexes the **ParentTree**, whose array maps each MCID back to its owning structure element (the reverse lookup used for hit-testing and for "what tag is this glyph in"). The MCID is used as a zero-based index into that array, so keep MCIDs small and dense (§14.7.4.4). Each MCID belongs to exactly one structure element; a structure element may own multiple MCIDs across pages (paragraph spanning pages). `ParentTree` is required whenever any structure element has content items (Table 322). A structural marked-content sequence must not contain another structural one (non-structural marked content may nest, §14.7.4.1), and a Tagged PDF needs `/MarkInfo << /Marked true >>` (§14.8.1).

**Artifacts** are content not part of the logical document — page numbers, running headers/footers, decorative rules, background images, the OCR layer when the image is the "real" content (debatable): `/Artifact BMC … EMC` or `/Artifact <</Type /Pagination /Subtype /Header>> BDC … EMC` (§14.8.2.2.2, Table 330: `Type` ∈ `Pagination`/`Layout`/`Page`/`Background` (1.7); `Subtype` (1.7) `Header`/`Footer`/`Watermark`, for Pagination only; `BBox` required for background artifacts; `Attached`). **Every** piece of real content must be either in the structure tree or an artifact (PDF/UA requirement).

### 9.2 Standard structure types

PDF 2.0 defines two standard structure namespaces: the **PDF 1.7 namespace** (`http://iso.org/pdf/ssn`) and the **PDF 2.0 namespace** (`http://iso.org/pdf2/ssn`), plus MathML (`http://www.w3.org/1998/Math/MathML`) for math. Elements without an explicit namespace are in the 1.7 namespace. Custom namespaces must role-map to a standard one.

| Category | PDF 2.0 namespace types |
|---|---|
| Document level | `Document`, `DocumentFragment` |
| Grouping | `Part`, `Sect`, `Div`, `Aside`, `NonStruct` |
| Block (paragraph-like) | `P`, `H1`…`Hn` (unbounded), `H`, `Title`, `FENote`, `Sub` |
| Inline | `Lbl`, `Span`, `Em`, `Strong`, `Link`, `Annot`, `Form`, `Ruby`/`RB`/`RT`/`RP`, `Warichu`/`WT`/`WP` |
| List | `L`, `LI`, `LBody` (+ `Lbl`) |
| Table | `Table`, `TR`, `TH`, `TD`, `THead`, `TBody`, `TFoot` |
| Caption, illustration | `Caption`, `Figure`, `Formula` |
| Other | `Artifact` (as a structure element, new in 2.0) |

(PDF 1.7 itself — §14.8.4 — has a single standard set, with headings limited to `H` and `H1`–`H6`.) PDF 1.7 types not in the 2.0 namespace: `Art`, `BlockQuote`, `TOC`, `TOCI`, `Index`, `Private`, `Quote`, `Note`, `Reference`, `BibEntry`, `Code` (still usable via the 1.7 namespace or role-mapping). Headings: PDF/UA-1 requires either strictly `H` (with `Sect` nesting) or numbered `Hn` without skipping levels; PDF/UA-2 relaxes the no-skipping rule for `Hn`.

Attributes (`/A`) add semantics: table `Scope` (Row/Column/Both), `Headers` (ids of TH), `RowSpan`/`ColSpan`, list `ListNumbering`, layout attributes (`Placement`, `BBox` for figures), `Summary`. Element keys: `/Alt`, `/ActualText`, `/E`, `/Lang`, `/T` (title), `/ID`, `/AF` (associated files, e.g. MathML source attached to a `Formula`), `/Ref` (PDF 2.0, e.g. link annotations ↔ content).

### 9.3 Language, alt text, reading order

- `/Lang` on catalog (default), structure elements, marked content not in the structure tree (`/Span <</Lang (fr)>> BDC` — the `Span` tag is required, §14.9.2.1) and text strings (via escape sequence in UTF-16). Language identifiers are case-insensitive; the empty string means "unknown" (§14.9.2.2). Screen readers switch voices by language; hyphenation and search depend on it.
- `/Alt` is required for `Figure` and `Formula` by PDF/UA (unless the figure is an artifact) — ISO 32000-1 itself only says Tagged PDF "should" use `Lang`/`Alt`/`ActualText`/`E` (§14.8.2.2.2); `ActualText` replaces content; `/E` expands abbreviations. An illustration's content must be whole graphics objects, never inside `BT`/`ET`, and needs a `BBox` attribute to be useful for reflow (§14.8.4.5).
- Reading order = **logical order of the structure tree** (depth-first), *not* content stream order. `/Tabs /S` on pages makes annotation tab order follow structure (required by PDF/UA).
- Links: `Link` structure element containing the link text MCIDs *and* an OBJR to the link annotation, which carries `/Contents` alt text and `/StructParent`.

### 9.4 PDF/UA, WTPDF, Matterhorn

| Spec | Year | Base | Key points |
|---|---|---|---|
| **PDF/UA-1** (ISO 14289-1:2014) | 2012/2014 | PDF 1.7 | Tagged everything-or-artifact; fonts embedded with Unicode mappings; `/Lang`; `dc:title` + `DisplayDocTitle`; `pdfuaid:part=1` in XMP; headings, tables, lists, figures with Alt; no reliance on colour; annotations tagged; no flicker etc. |
| **WTPDF 1.0** (Well-Tagged PDF, PDF Association) | Feb 2024 | PDF 2.0 | Free spec; two conformance levels — **Reuse** and **Accessibility** — for reliable reflow/extraction/conversion as well as AT. |
| **PDF/UA-2** (ISO 14289-2:2024) | Mar 2024 | PDF 2.0 | File-format requirements identical to WTPDF's Accessibility level; uses PDF 2.0 namespaces, MathML for formulae, `pdfuaid:part=2` (+`rev`). Does not replace PDF/UA-1. |
| **Matterhorn Protocol 1.1** (PDF Association, 2021) | | PDF/UA-1 | 31 checkpoints and ~136 failure conditions, split into machine-checkable and human-judgement ones — the checklist validators implement. |

Regulation drives adoption: the EU **European Accessibility Act** applies from 28 June 2025 and the US DOJ ADA Title II rule (April 2024) requires WCAG 2.1 AA for state/local-government web content, including PDFs, with compliance dates originally set for April 2026 / April 2027 — PDF/UA conformance is the most common technical basis for showing that PDFs meet them.

**Validation tools.**
- **veraPDF** (open source, Java): PDF/A all parts; PDF/UA-1 machine-checkable rules; PDF/UA-2 since 1.26 (2024) and WTPDF since 1.28 (2025); CLI + REST + library. The CI tool lamina should use.
- **PAC** (PDF Accessibility Checker, axes4/PDF Association; Windows, free): PAC 2024 and PAC 2026 check PDF/UA-1 + WCAG 2.x; includes a screen-reader preview and logical structure view. Not scriptable in CI in the same way.
- **Acrobat Pro** Accessibility Checker (not a conformance validator), **axesCheck** (online), **callas pdfGoHTML/pdfaPilot**, **CommonLook**.
- Machine checks cover ~half of requirements; alt-text quality, correct reading order and correct semantics require human review.

### 9.5 Generating tagged PDF from code

Rules for a generator:

1. Build the structure tree **from the document model** (the API's paragraphs, headings, lists, tables, images), not after the fact from the page.
2. While emitting content, wrap each real-content run in `BDC … EMC` with the next MCID for that page; wrap everything else as `/Artifact`. Never let content fall outside both. Marked-content sequences must not cross `BT`/`ET` boundaries improperly — they may contain whole text objects or be inside one (§14.6.1: each `BMC/BDC…EMC` and `BT…ET` pair "shall be properly (separately) nested"), must lie within a single content stream (a page's `/Contents` array counts as one), and — as good practice, not stated in 32000-1 — should nest properly with `q`/`Q`.
3. Record `(page, MCID) → StructElem`; at page end write the ParentTree array for that page's `/StructParents` key.
4. Emit `/Lang`, `MarkInfo`, `DisplayDocTitle`, XMP metadata (`dc:title`, `pdfuaid:part`), `/Tabs /S`.
5. Fonts: embedded, with complete `ToUnicode`; real spaces between words (Tagged PDF: "the spacing characters that would be present to separate words in a pure text representation shall be present", §14.8.2.5); `ActualText` for non-1:1 clusters; soft hyphens (§14.8.2.2.3).
6. Images/vector figures: `Figure` with `/Alt` (API must make alt text a first-class required-or-explicitly-decorative parameter), `/BBox` attribute.
7. Tables: `Table/THead/TBody/TR/TH/TD` with `Scope`/`Headers`; lists `L/LI/Lbl/LBody`; links with OBJR.
8. Validate with veraPDF in CI for every example document.

Reference implementations worth studying: Typst/krilla (Rust, PDF/UA-1 output), LaTeX's tagging project (`\DocumentMetadata{testphase=…}`, PDF/UA-2 output from LuaLaTeX), iText 9 (Java/.NET, PDF/UA-2 + WTPDF), WeasyPrint (PDF/UA), Apache FOP, BFO, PDFlib, Prince.

---

## 10. Worked, verified example

A ~150-line Python script (only dependency: `fontTools`) writes a one-page PDF **by hand** (no PDF library) that demonstrates: a standard-14 simple font with `WinAnsiEncoding`; a TJ-kerned line; a subset, embedded **Type 0 / Identity-H / CIDFontType2** font (DejaVu Sans) with `/W` widths and a ToUnicode CMap including a ligature; an invisible mode-3 text line marked as an artifact; and a minimal structure tree. Script: `make_text_example.py` (to be checked into `docs/research/pdf/examples/`). Verified on 2026-10-28 with fontTools 4.68.0 and Poppler 24.02.

### 10.1 Font preparation (essential parts)

```python
src = TTFont("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
assert src["OS/2"].fsType & 0x0002 == 0          # not "Restricted License"

opts = subset.Options()
opts.layout_features = []                        # embedded font is never shaped
opts.name_IDs = []
opts.notdef_outline = True
opts.retain_gids = False                         # compact GIDs
opts.drop_tables += ["GSUB","GPOS","GDEF","kern","MATH","FFTM","post"]
sub = subset.Subsetter(opts); sub.populate(glyphs=[cmap[c] for c in chars])
font = TTFont(FONT); sub.subset(font)            # closure over composite glyphs happens here
gid_of = {name: i for i, name in enumerate(font.getGlyphOrder())}   # NEW GIDs

# /W in 1/1000 em, from the SUBSET's hmtx
W = "[ 0 [" + " ".join(str(round(font["hmtx"][n][0]*1000/upem)) for n in order) + "] ]"
# ToUnicode: one bfchar per used new GID; U+FB01 'ﬁ' glyph maps to "fi" = <00660069>
```

(In a real engine the GIDs come from HarfBuzz and the ligature from GSUB `liga`; here the script cheats by using the U+FB01 presentation form directly so the example has no shaping dependency.)

(Fact-check note: the options above do not drop the subset's `cmap` table. ISO 32000-1 §9.9 says a TrueType program used with a CIDFont "shall not" contain a `cmap`; Poppler doesn't care, but a strict writer should add `"cmap"` to `drop_tables` for CIDFontType2 output. The script was not re-run as part of this check.)

### 10.2 The font dictionaries (actual output)

```
6 0 obj   % the font resource referenced by /F2
<< /Type /Font /Subtype /Type0
   /BaseFont /LAMINA+DejaVuSans          % subset tag: 6 uppercase letters + '+'
   /Encoding /Identity-H                 % 2-byte codes, code = CID, horizontal
   /DescendantFonts [7 0 R]              % exactly one CIDFont
   /ToUnicode 12 0 R >>                  % extraction mapping (CID → UTF-16BE)
endobj

7 0 obj   % the descendant CIDFont
<< /Type /Font /Subtype /CIDFontType2    % TrueType outlines
   /BaseFont /LAMINA+DejaVuSans
   /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>
   /FontDescriptor 8 0 R
   /DW 1000
   /W [ 0 [600 318 361 698 632 752 787 695 613 635 615 278 278 634 635 411 521 392
           592 592 318 500 632 659 659 634 338 589 592 559 710 613 650 639 635 681 630] ]
   /CIDToGIDMap /Identity >>             % CID = GID in the embedded subset
endobj

8 0 obj
<< /Type /FontDescriptor /FontName /LAMINA+DejaVuSans
   /Flags 4                              % Symbolic: glyphs outside Adobe standard Latin
   /FontBBox [23 -208 731 800]           % of the subset, in 1/1000 em
   /ItalicAngle 0 /Ascent 928 /Descent -236
   /CapHeight 729                        % from the 'H' glyph (font's OS/2 v1 lacks sCapHeight)
   /StemV 80
   /FontFile2 11 0 R >>                  % stream: subset sfnt, /Length1 = uncompressed length
endobj

5 0 obj   % simple font, standard 14, not embedded (fine for a demo; fails PDF/A & PDF/UA)
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>
endobj
```

### 10.3 The content stream (decompressed)

```
/P <</MCID 0>> BDC                 % marked content → StructElem P #1
BT
/F1 24 Tf
72 720 Td
(Hello, PDF text!) Tj              % 1 byte per char, WinAnsi
0 -30 Td
[(W) 120 (AVE) -250 (kerned via TJ)] TJ     % +120 tightens W–A; −250 opens a 6 pt gap
ET
EMC
/P <</MCID 1>> BDC
BT
/F2 18 Tf
1 0 0 1 72 640 Tm
<0016001C001C0019001D001A001B0017000100140001001E00200022002000210021002000230
01F0001001400010024000D0008000C> Tj        % "Ελληνικά · Кириллица · ﬁnal" as 2-byte GIDs
ET
EMC
/Artifact BMC                       % not part of the logical document
BT
3 Tr                                % invisible: painted nowhere, still extractable
/F2 12 Tf
72 600 Td
<0005000B00090009000A000D…> Tj      % "Hidden OCR-style text layer"
ET
EMC
```

ToUnicode excerpt (32 entries — one per *character-bearing* GID; the four closure-only component glyphs `E`, `alpha`, `tonos`, `p` that are never painted directly get no entry because no code in the content refers to them):

```
32 beginbfchar
<0001> <0020>
<0016> <0395>      % Ε GREEK CAPITAL EPSILON (composite → references Latin E, GID 4)
<0017> <03AC>      % ά (composite: alpha + tonos)
<001F> <0430>      % Cyrillic а (composite → Latin a)
<0024> <00660069>  % ﬁ ligature glyph → "f" "i"
…
endbfchar
```

### 10.4 Verification output

```
$ pdffonts text-example.pdf
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
Helvetica                            Type 1            WinAnsi          no  no  no       5  0
LAMINA+DejaVuSans                    CID TrueType      Identity-H       yes yes yes      6  0

$ pdftotext -layout text-example.pdf -
Hello, PDF text!
WAVE kerned via TJ

Ελληνικά · Кириллица · final           ← ligature glyph extracted as "fi"

Hidden OCR-style text layer             ← invisible mode-3 text is extracted (that's the point of OCR layers)

$ pdfinfo -struct-text text-example.pdf
Document
  P (block)
    "Hello, PDF text!WAVEkerned via TJ"  ← no space: positional gaps aren't spaces to this code path
  P (block)
    "Ελληνικά · Кириллица · final"

$ pdfinfo text-example.pdf | grep Tagged
Tagged:          yes
```

Rendered (Poppler `pdftoppm`) the page shows the three visible lines with correct Greek/Cyrillic glyphs and the `ﬁ` ligature; the mode-3 line is invisible. Subset: 6,253 → 37 glyphs, 8.1 KB embedded; whole PDF 9.5 KB.

Lessons encoded in this example:
- `pdftotext` coordinates reproduce the §2.6 displacement math to the micropoint.
- Two Poppler code paths disagree about the TJ gap → **emit a real U+0020 glyph between words** (for Tagged PDF this is a requirement, §14.8.2.5 — the first line of this example is therefore non-conforming as tagged content) (Identity-H has no `Tw`, so justify with TJ adjustments *around* the space glyph, not instead of it).
- Glyph closure is not optional: 4 of 32 characters here depend on component glyphs.
- To make it PDF/UA-1 conformant one would still need: embedded Helvetica replacement, `pdfuaid` in XMP, the two first-line "lines" as separate structure if they're semantically separate, etc. — veraPDF would list each failure.

---

## 11. Implications for lamina's design

### 11.1 Layered text architecture

```
┌───────────────────────────────────────────────────────────────────────────┐
│ L4  Document/flow API: paragraphs, headings, lists, tables, images+alt,    │
│     styles, lang; produces structure tree automatically                   │
├───────────────────────────────────────────────────────────────────────────┤
│ L3  Paragraph layout: bidi, itemisation, fallback, line breaking,           │
│     hyphenation, justification (greedy + Knuth–Plass), vertical metrics    │
├───────────────────────────────────────────────────────────────────────────┤
│ L2  Shaped text: shape(text, font, size, features, lang, dir) → GlyphRun     │
│     { glyphs[gid, cluster, advance, offset], source text, font instance }  │
├───────────────────────────────────────────────────────────────────────────┤
│ L1  PDF text emission: GlyphRun → Tf/Tm/TJ; per-font glyph registry;        │
│     subsetting at finalize; /W, ToUnicode, ActualText, MCIDs               │
├───────────────────────────────────────────────────────────────────────────┤
│ L0  Raw operators: BT/ET, Tf, Td, Tm, Tj, TJ, Tr… with typed arguments       │
└───────────────────────────────────────────────────────────────────────────┘
```

Each layer must be usable on its own: power users drive L0/L1 (exact placement, custom shaping); typical users use L3/L4.

### 11.2 Concrete requirements

**Fonts**
- `Font` loaded from bytes (TTF/OTF/TTC index/WOFF2-decompressed), with variation coordinates → an immutable `FontInstance`. Instantiate variable fonts at embed time.
- Default embedding: Type 0 + Identity-H + CIDFontType2 (glyf) or CIDFontType0 (CFF; convert to CID-keyed CFF). Simple-font embedding as an option (smaller for tiny Latin docs, and needed for some legacy consumers).
- Standard 14 by name with bundled AFM metrics; disallowed under PDF/A/UA profiles.
- Deterministic subset tags (hash of font + glyph set); deterministic output overall (byte-reproducible builds).
- Respect `fsType`; error with override.
- Glyph usage registry per font instance; subset + write `/W`, ToUnicode (chunked ≤100/block, multi-code-point, surrogates), `CIDSet` only for PDF/A-1–3 profiles.
- Font discovery (fontconfig/CoreText/DirectWrite or `fontdb`-style), font fallback chains per script/lang, with explicit control and reporting of `.notdef`s ("missing glyph" diagnostics, never silent tofu).
- Type 3 path for colour emoji (COLR v0/v1, bitmaps, SVG) with ToUnicode.

**Shaping & layout**
- Shaper behind a trait/interface; default HarfBuzz (C) or HarfRust (pure Rust) depending on lamina's implementation language; OpenType feature control (`liga`, `kern`, `smcp`, `tnum`, `ss01`…) per run; language/script tags.
- Unicode services: bidi (UAX #9), segmentation (UAX #29), line breaking (UAX #14 + dictionary segmentation for SE-Asian scripts), script itemisation (UAX #24) — ICU4X (`icu_segmenter`, `icu_properties`) is the natural choice in Rust.
- Hyphenation with hyph-utf8 patterns per language, plus soft-hyphen support.
- Knuth–Plass and greedy line breaking; justification via TJ adjustments (never `Tw` for Type 0), optional letter-spacing/scaling limits, kashida for Arabic.
- Vertical metrics model (ascender/descender/line gap choice: `hhea` vs `OS/2 typo` with `USE_TYPO_METRICS` flag vs `win`), baseline grids, first-baseline offset.
- Vertical writing (Identity-V, `vert` feature, `/W2`) as a later milestone.

**Emission**
- Text as glyph runs with clusters retained → automatic ToUnicode; `ActualText` automatically when a cluster is non-monotonic or many-to-many; soft hyphen encoding for auto-hyphenation.
- Always emit real space glyphs between words.
- Minimise operators: one `TJ` per run with numeric adjustments; avoid one-`Tm`-per-glyph output (bloats files and hurts every extractor).
- Text render modes exposed (including invisible text for OCR overlays — a likely lamina feature: "add OCR layer from hOCR/ALTO").

**Tagging & accessibility**
- Structure tree built by default from L4; L0/L1 users get an explicit `begin_tag/end_tag/artifact` API with MCID bookkeeping and ParentTree generation done for them.
- Conformance profiles (`PDF/UA-1`, `PDF/UA-2`, `WTPDF`, `PDF/A-2a/2u/4`…) that turn requirements into API errors at build time (missing alt text, unembedded font, missing `Lang`, unmapped glyph).
- veraPDF in CI against a corpus of lamina-generated documents.

**Reading/extraction**
- A lossless glyph-level page model: for each glyph — Unicode (with provenance: ToUnicode / AGL / cmap / guessed), GID, font, `Trm`, bbox (from font metrics, optionally actual outline bounds), advance, render mode, fill/stroke colour, clip state, visibility, MCID and structure-element path, ActualText scope.
- On top: configurable word/line/block heuristics (pdfminer-like parameters), tag-based reading order when the tree is present and passes sanity checks, de-hyphenation, ligature decomposition options; output to plain text, hOCR/ALTO-like JSON, Markdown.
- Hooks for OCR (render page → external OCR → invisible text layer) and ML layout models, rather than reimplementing them.
- Robustness: broken ToUnicode, missing widths (fall back to font program), Type 3 garbage detection, fonts with invalid `cmap`s, duplicated fake-bold glyphs.

**Editing**
- Treat text editing as: locate the runs (via structure tree/MCID if available, else heuristics) → remove their operators → re-layout new text with the original font if the subset contains the glyphs, otherwise embed a new subset (or substitute font) → re-emit. Be explicit that "edit in place" is best-effort; redaction (true removal of glyphs, not covering them) is a separate, security-relevant feature.

---

## References

**Standards & specifications**
- ISO 32000-2:2020, *Document management — Portable document format — Part 2: PDF 2.0* (available at no cost via the PDF Association sponsorship): https://pdfa.org/sponsored-standards/ · https://www.iso.org/standard/75839.html
- PDF Association, *PDF 2.0 errata / pdf-issues tracker*: https://github.com/pdf-association/pdf-issues
- Adobe, *PDF Reference, 6th ed. (PDF 1.7)* / ISO 32000-1:2008: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf
- Adobe Technical Note #5014, *Adobe CMap and CIDFont Files Specification*; #5411, *ToUnicode Mapping File Tutorial*; #5099, *Developing CMap Resources for CID-Keyed Fonts*: https://github.com/adobe-type-tools/font-tech-notes
- Adobe CMap resources: https://github.com/adobe-type-tools/cmap-resources
- Adobe Glyph List & AGL Specification: https://github.com/adobe-type-tools/agl-specification · https://github.com/adobe-type-tools/agl-aglfn
- Adobe Technical Note #5176, *The Compact Font Format Specification*; #5177, *Type 2 Charstring Format*: https://adobe-type-tools.github.io/font-tech-notes/pdfs/5176.CFF.pdf
- Adobe, *Type 1 Font Format*: https://adobe-type-tools.github.io/font-tech-notes/pdfs/T1_SPEC.pdf
- Microsoft OpenType specification (incl. `OS/2.fsType`, `glyf`, `CFF2`, `COLR`): https://learn.microsoft.com/en-us/typography/opentype/spec/ · https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fstype
- Apple TrueType Reference Manual: https://developer.apple.com/fonts/TrueType-Reference-Manual/
- Unicode UAX #9 (Bidi): https://www.unicode.org/reports/tr9/ · UAX #14 (Line Breaking): https://www.unicode.org/reports/tr14/ · UAX #29 (Text Segmentation): https://www.unicode.org/reports/tr29/ · UAX #24 (Script property): https://www.unicode.org/reports/tr24/
- ISO 14289-1:2014 (PDF/UA-1): https://www.iso.org/standard/64599.html
- ISO 14289-2:2024 (PDF/UA-2): https://pdfa.org/iso-14289-2-pdfua-2/
- PDF Association, *Well-Tagged PDF (WTPDF) 1.0*: https://pdfa.org/wtpdf/ · https://pdfa.org/game-changing-new-free-specification-enables-interoperable-reuse-and-accessibility-for-pdf
- PDF Association, *PDF/UA-2 announcement*: https://pdfa.org/iso-14289-2-pdf-ua-2-the-gold-standard-for-accessibility-in-pdf-2-0-has-arrived/
- PDF Association, *Matterhorn Protocol 1.1*: https://pdfa.org/resource/the-matterhorn-protocol/
- PDF Association, *Tagged PDF Best Practice Guide: Syntax*: https://pdfa.org/resource/tagged-pdf-best-practice-guide-syntax/
- axes4, *PDF/UA-2 vs Well-Tagged PDF*: https://www.axes4.com/en/blog/post/2024/pdf-ua-2-vs-well-tagged-pdf

**Algorithms**
- D. E. Knuth & M. F. Plass, "Breaking Paragraphs into Lines", *Software—Practice and Experience* 11 (1981): https://doi.org/10.1002/spe.4380111102
- F. M. Liang, *Word Hy-phen-a-tion by Com-put-er*, Stanford PhD thesis (1983): https://tug.org/docs/liang/
- hyph-utf8 hyphenation patterns: https://github.com/hyphenation/tex-hyphen
- HarfBuzz manual — clusters: https://harfbuzz.github.io/clusters.html

**Font libraries**
- HarfBuzz (shaping, `hb-subset`): https://github.com/harfbuzz/harfbuzz · https://harfbuzz.github.io/
- HarfRust: https://github.com/harfbuzz/harfrust · https://docs.rs/crate/harfrust/latest ; rustybuzz (archived): https://github.com/harfbuzz/rustybuzz
- fontTools (subset, instancer, cffLib): https://github.com/fonttools/fonttools · https://fonttools.readthedocs.io/en/latest/subset/
- fontations (read-fonts, write-fonts, skrifa, klippa): https://github.com/googlefonts/fontations
- allsorts: https://github.com/yeslogic/allsorts
- Typst `subsetter` / `krilla`: https://github.com/typst/subsetter · https://github.com/LaurenzV/krilla
- fontkit: https://github.com/foliojs/fontkit · harfbuzzjs: https://github.com/harfbuzz/harfbuzzjs
- ICU4X: https://github.com/unicode-org/icu4x

**Extraction, OCR, layout analysis**
- Poppler: https://poppler.freedesktop.org/
- MuPDF / PyMuPDF text extraction: https://mupdf.readthedocs.io/ · https://pymupdf.readthedocs.io/en/latest/app1.html
- pdf.js: https://github.com/mozilla/pdf.js
- Apache PDFBox: https://pdfbox.apache.org/
- pdfminer.six (layout analysis docs): https://pdfminersix.readthedocs.io/en/latest/topic/converting_pdf_to_text.html
- pdfplumber: https://github.com/jsvine/pdfplumber
- Tabula: https://tabula.technology/ · Camelot: https://github.com/camelot-dev/camelot
- Tesseract: https://github.com/tesseract-ocr/tesseract
- OCRmyPDF releases (v17.x; fpdf2 renderer, ligature and CMap fixes): https://github.com/ocrmypdf/OCRmyPDF/releases
- Docling: https://github.com/docling-project/docling · Granite-Docling: https://ibm.com/new/announcements/granite-docling-end-to-end-document-conversion
- Marker: https://github.com/datalab-to/marker · Marker 2 benchmark coverage: https://www.marktechpost.com/2026/07/24/datalab-marker-v2-vs-mineru-docling-and-liteparse-benchmark-breakdown/
- MinerU: https://github.com/opendatalab/MinerU
- olmOCR & olmOCR-bench: https://allenai.org/blog/olmocr · https://github.com/allenai/olmocr
- Nougat: https://github.com/facebookresearch/nougat
- PaddleOCR-VL: https://arxiv.org/abs/2510.14528
- READoc benchmark: https://arxiv.org/abs/2409.05137
- Table Transformer: https://github.com/microsoft/table-transformer

**Accessibility validation**
- veraPDF (release notes incl. PDF/UA-2 and WTPDF support): https://verapdf.org/ · https://github.com/veraPDF/veraPDF-library/blob/integration/RELEASENOTES.md
- PAC (PDF Accessibility Checker): https://pac.pdf-accessibility.org/
- LaTeX tagging project (PDF/UA-2 examples): https://www.latex-project.org/news/2024/03/27/PDFUA2-examples/

---

## Verification log

**Date:** 2026-10-28

**Sources checked against:**
- ISO 32000-1:2008 (PDF 1.7), full text — §8.2 (Figure 9/Annex L), §9.2–9.10, §14.6–14.9, Annex D.
- Adobe Technical Note #5411, *ToUnicode Mapping File Tutorial* (29 May 2003).
- LlamaIndex PDF OCR article and Nowater PDF tools guide (OCR text-layer / mode-3 claims; consistent with the chapter, nothing changed).
- ISO 32000-2 (PDF 2.0) was **not** available; 2.0-only claims are labelled as such and left unverified.

**Confirmed without change (now cited):** text state parameters, defaults and operators (Tables 104–105); `Td`/`TD`/`Tm`/`T*`/`'`/`"`/`TJ` semantics (Tables 108–109); the `Trm` and glyph-displacement formulas (§9.4.4); `Tw` applying only to single-byte code 32 (§9.3.3); rendering modes 0–7 and clip accumulation until `ET` (§9.3.6); font descriptor flag bit positions 1, 2, 3, 4, 6, 7, 17, 18, 19 (Table 123); `/W` syntax and example, `DW` 1000, `DW2` [880 −1000] (§9.7.4.3, Table 117); CID-keyed vs name-keyed CFF CID→GID rule (§9.7.4.2); `CIDToGIDMap` byte layout; subset tag rule (§9.6.4); WinAnsi codes 0x27/0x60/0x80 (Annex D.2); the predefined CMap names cited (Table 118); OpenType embedding in `FontFile3` since PDF 1.6 (Table 126); `Length1/2/3` (Table 127); TrueType required tables (§9.9); ToUnicode UTF-16BE, multi-code-point destinations, surrogates (§9.10.3, TN #5411 §1.5); 100-entry block limit and the `bfrange` byte-boundary rule (TN #5411 §1.4.1); `ParentTree`/MCID/MCR/OBJR model (§14.7.2–14.7.4); artifact syntax and `Pagination`/`Header` (Table 330).

**Corrections made (was → now, clause):**
1. Type 3 + rendering modes: "renderer behaviour varies — don't rely on it" → only mode 3 has any effect on Type 3 text (§9.3.6).
2. Font descriptor required entries: "`/CapHeight` & `/StemV` required only for non-embedded non-symbolic fonts" → `/FontBBox`, `/Ascent`, `/Descent`, `/StemV` required except Type 3; `/CapHeight` required for fonts with Latin characters except Type 3; `/ItalicAngle` always (Table 122).
3. Font descriptor applicability: added that Type 0 fonts "shall not" have one and that Type 3 descriptors are optional since PDF 1.5 but required in Tagged PDF (§9.8.1, Table 112).
4. Symbolic/Nonsymbolic: "exactly one should be set" → "shall not both be set or both be clear" (Table 123).
5. `/Panose` listed as a top-level CIDFont descriptor key → it lives inside `/Style` (§9.8.3.2, Table 124); added `FontFamily/FontStretch/FontWeight`.
6. Unicode-recovery priority (§9.10.2): method 2 narrowed from "AGL-resolvable glyph names" to the three named encodings or `Differences` using only standard-Latin/Symbol names; method 3 now notes Identity-H/V are excluded but a registered descendant collection still allows fallback; "Identity-H has no fallback" qualified to Identity/unregistered collections.
7. ToUnicode `bfrange` rule: reworded to TN #5411's "do not cross first-byte boundaries" (all bytes except the last identical) with its `<10FE> <1101>` example; **added** the destination constraint "last byte ≤ 255 − (srcCode2 − srcCode1)" (§9.10.3), and the 512-byte destination limit.
8. 100-entry limit: source changed from TN #5014 only to TN #5411 §1.4.1 (which cites the CMap-file rule).
9. Codespace rule: now quotes §9.10.3 ("for a simple font, the codespace shall be one byte long"); added `<FFFD>` guidance (TN #5411) and the UCS-2-only note for Acrobat 4/5.
10. Soft hyphens: "PDF/UA-2/WTPDF guidance favours U+00AD" → Tagged PDF itself *requires* incidental hyphens to map to U+00AD (§14.8.2.2.3); `ActualText` kept for spelling-changing hyphenation (spec example, §14.9.4).
11. ActualText example: the UTF-16BE string `\011\025\011\101\011\077` decoded to U+0915 U+0941 U+093F (not the "क्ि" in the comment) → replaced with a correct "कि" (U+0915 U+093F) pre-base-matra example; added that marked-content `ActualText` is PDF 1.5 and needs the `Span` tag.
12. `/Lang` on marked content: `<</Lang (fr)>> BDC` → `/Span <</Lang (fr)>> BDC` (§14.9.2.1); catalog `/Lang` "BCP 47" → RFC 3066 in 1.7 (§14.9.2.2), BCP 47 for 2.0 (unverified).
13. `/Alt` "required for Figure/Formula" → attributed to PDF/UA; ISO 32000-1 only says "should" (§14.8.2.2.2).
14. CIDFontType2 `cmap`: "not needed" → ISO 32000-1 says "not needed and shall not be present" (§9.9); flagged that the §10 example's fontTools options keep `cmap`.
15. `vhea`/`vmtx`: added that readers shall never use them (§9.9).
16. `StandardEncoding`: "no accented characters at ≥128 except via seac" → encodes no accented letters at all (Annex D.2).
17. TrueType lookup clause: §9.6.5.4 → "§9.6.5.4 (PDF 2.0) / §9.6.6.4 (ISO 32000-1)"; chapter §4 heading likewise §9.6.5 → §9.6.6 for 1.7.
18. Structure tree root keys: `/PronunciationLexicon`, `/AF` now labelled PDF 2.0 only (1.7 Table 322 has Type, K, IDTree, ParentTree, ParentTreeNextKey, RoleMap, ClassMap).
19. Marked-content nesting: the `q`/`Q` nesting requirement is not in 32000-1 → relabelled as good practice; added the single-content-stream rule (§14.6.1).
20. Bidi: "PDF has no bidi concept" → added the `/ReversedChars` exception (§14.8.2.3.3).
21. Widths vs font program: "conforming renderers use the PDF value" → spec requires consistency (Table 111, §9.7.4.3); renderer preference marked as practice.

**Additions (sourced):** `Identity-H/V` shall not be used with non-embedded fonts (§9.7.5.2); `CIDToGIDMap` default `/Identity` and only for embedded TrueType (Table 117); CIDSystemInfo compatibility = same Registry+Ordering (§9.7.3); `bfchar`/`bfrange` not allowed in Type 0 encoding CMaps (§9.7.5.4); spec mandate that writers use Type 0/Identity-H for OpenType `glyf` fonts (§9.9); text objects' allowed-operator list (Figure 9); Tagged PDF requires real spaces between words (§14.8.2.5), a single top-level structure element (§14.8.4.2), and `MarkInfo /Marked true` (§14.8.1); illustration content not inside `BT/ET`, `BBox` needed for reflow (§14.8.4.5); Type 3 out-of-range widths are 0.

**Still unverified (outside these sources):**
- Everything specific to PDF 2.0: structure namespaces and the 2.0 type list (`DocumentFragment`, `Aside`, `FENote`, `Sub`, `Title`, `Em`/`Strong`, unbounded `Hn`, `Artifact` structure element), `/Ref`, `/AF`, `/Namespaces`, UTF-8 text strings, `Suspects` deprecation, `CIDSet` deprecation, whether 2.0 keeps the CIDFont `cmap` "shall not be present" wording, and 2.0 clause numbers where they differ from 1.7.
- PDF/A, PDF/UA-1/-2, WTPDF and Matterhorn requirements and counts; EAA/ADA dates; veraPDF/PAC versions.
- `OS/2.fsType` bit semantics (OpenType spec), AGL algorithm details, UAX algorithms, Knuth–Plass/Liang descriptions.
- Library versions and behaviours (HarfBuzz, HarfRust, fontations/klippa/skera, fontTools, OCRmyPDF 17.x, Tesseract GlyphLessFont internals) and ML-tool claims (Docling, Marker 2 benchmark numbers, MinerU, olmOCR-bench).
- The §10 worked example's outputs (script not re-run; arithmetic in §2.6 re-checked and consistent with §9.4.4).
