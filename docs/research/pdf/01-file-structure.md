# 01 — File Structure & Syntax

> **Scope.** This chapter covers how a PDF file is physically laid out (bytes, tokens, objects, cross-reference data, trailers, incremental updates, linearization, stream filters) and how it is logically organized (catalog, page tree, resources, name/number trees, metadata, outlines, page labels). It ends with how real parsers read real, often broken, files, plus a byte-exact "Hello, World" PDF that we built and checked.
>
> **Primary reference:** ISO 32000-2:2020 (PDF 2.0, 2nd edition), §7 "Syntax" and §7.7 "Document structure", plus Annexes F (linearization) and C (limits). Where PDF 1.7 / ISO 32000-1:2008 differs, the text says so. Clause numbers are for ISO 32000-2. ISO 32000-1 numbers them almost the same way for this material; where the 32000-1 number differs it is given in parentheses. On 2026-10-28 the PDF 1.7-level claims were checked against the text of ISO 32000-1:2008 (see the Verification log at the end); PDF 2.0-only claims could not be checked against the 2.0 text.
>
> Content streams (operators, graphics state, text) are covered in later chapters, as are fonts, encryption, signatures and security. This chapter only touches them where they affect file structure.

---

## Table of contents

1. History and lineage
2. The ISO family: subset and profile standards
3. Lexical conventions
4. The object model
5. Physical file structure: header, body, xref, trailer
6. Cross-reference streams, object streams, hybrid files
7. Incremental updates
8. Linearization ("Fast Web View")
9. Stream filters
10. Document logical structure
11. How a parser actually reads a PDF
12. A minimal, verified "Hello, World" PDF
13. Security-relevant parsing pitfalls
14. Implementation guidance for lamina
15. References
16. Verification log

---

## 1. History and lineage

PDF comes from **PostScript** (Adobe, 1984). PostScript is a Turing-complete stack language for describing pages. A page description is a program that a printer runs. That power caused problems for document exchange: you cannot find page 500 without running pages 1–499, the output can depend on the interpreter, and a program can loop forever. In 1991 John Warnock's internal "Camelot" memo proposed a simpler form: keep PostScript's imaging model (paths, fills, fonts, transformation matrices) but remove the programming language, and store the result as a random-access structure of objects.

| Version | Year | Acrobat | Structural/syntax-relevant additions (non-exhaustive) |
|---|---|---|---|
| PDF 1.0 | 1993 | 1.0 | Core object model, xref table, LZW, page tree, outlines |
| PDF 1.1 | 1994 | 2.0 | Encryption (RC4-40), links, threads, device-independent color |
| PDF 1.2 | 1996 | 3.0 | **FlateDecode**, `#xx` name escapes, AcroForms, Unicode text strings (UTF-16BE) |
| PDF 1.3 | 2000 | 4.0 | Digital signatures, CIDFonts, DeviceN, smooth shading, JavaScript, page boundaries (Bleed/Trim/Art) |
| PDF 1.4 | 2001 | 5.0 | Transparency, **JBIG2Decode**, XMP metadata streams, Tagged PDF, catalog `/Version`, RC4-128 |
| PDF 1.5 | 2003 | 6.0 | **Cross-reference streams, object streams**, **JPXDecode**, Crypt filters, optional content |
| PDF 1.6 | 2004 | 7.0 | AES-128, `/UserUnit`, OpenType embedding, 3D (U3D), NChannel |
| PDF 1.7 | 2006 | 8.0 | Portfolios/collections, more XFA. Later Adobe "Extension Levels" (e.g., 1.7 ExtensionLevel 3 = AES-256) via catalog `/Extensions` |
| ISO 32000-1 | 2008 | — | PDF 1.7 published as an ISO standard. Adobe gave up change control to ISO TC 171/SC 2 |
| ISO 32000-2 | 2017 | — | **PDF 2.0**: first version developed inside ISO |
| ISO 32000-2 | 2020 | — | 2nd edition of PDF 2.0: clarifications and corrections, no new feature set |

PDF 2.0 changes that matter for this chapter:

- **Header `%PDF-2.0`.** The version number no longer tracks Acrobat releases. There is no "PDF 1.8".
- **Trailer `/ID` is required** in PDF 2.0 files. In 1.7 it was required only when `/Encrypt` was present.
- **The document information dictionary (`/Info`) is deprecated**, except for `/CreationDate` and `/ModDate`. XMP metadata is the normative store.
- **UTF-8 text strings** are allowed, marked with the BOM `EF BB BF`. Before 2.0 a text string was either PDFDocEncoding or UTF-16BE with BOM `FE FF`.
- Deprecations elsewhere in the spec, such as XFA, `NeedsRendering`, RC4 and the older security handler revisions, and relying on non-embedded standard-14 fonts. PostScript XObjects (already discouraged in 1.7) were removed outright rather than deprecated, per the Library of Congress PDF 2.0 format description. PDF 2.0 readers must still handle these. Writers should not emit them.
- New document-level features: associated files (`/AF`), document parts (`/DPartRoot`), the document security store (`/DSS`), page-level output intents, and structure namespaces.

**Governance and availability.** The PDF Association (a non-profit industry association) hosts the ISO working groups. From 2017 to 2023 you had to buy ISO 32000-2 from ISO. On **5 April 2023** the PDF Association made ISO 32000-2:2020 available **at no cost**, sponsored by Adobe, Apryse and Foxit. The download is watermarked with the requester's name. The current free copy includes the PDF Association's errata collections. The related ISO/TS 32001 (SHA-3/SHA-512 hash extensions), ISO/TS 32002 (ECC/EdDSA signature extensions) and ISO/TS 32003 (AES-GCM) are also free. Adobe still hosts a free copy of ISO 32000-1:2008 (PDF 1.7) and the older "PDF Reference" editions. The PDF Association also runs a public corrections tracker for PDF 2.0 (`pdf-issues.pdfa.org`).

**Extensions after 2.0.** New features now arrive as ISO technical specifications or PDF Association extension specs, declared through the catalog `/Extensions` dictionary, rather than as new PDF versions. One example relevant to file structure: the PDF Association has published an extension that adds a **`/BrotliDecode`** stream filter. iText, HexaPDF and pypdf have implementations. Inclusion in ISO 32000 is still pending as of late 2026. lamina should be able to *read* it. It should not *write* it by default until mainstream viewers support it.

## 2. The ISO family: subset and profile standards

These standards add constraints on top of ISO 32000. A conforming file is still an ordinary PDF. These standards add no syntax. They forbid some features and require others. Declaring conformance always means XMP metadata identification (e.g., `pdfaid:part`, `pdfuaid:part`) and often an `/OutputIntents` entry.

| Family | ISO | Base | Purpose | What it constrains (summary) |
|---|---|---|---|---|
| **PDF/A-1** | 19005-1:2005 | PDF 1.4 | Long-term archiving | No encryption, no JavaScript/launch actions, no LZW, no transparency, all fonts embedded, device-independent color or OutputIntent, XMP required. Levels **a** (accessible: tagged) and **b** (basic: visual appearance) |
| **PDF/A-2** | 19005-2:2011 | ISO 32000-1 | Archiving | Adds JPEG 2000, transparency, layers, embedded files only if they are PDF/A themselves. Levels a/b/**u** (Unicode text mapping) |
| **PDF/A-3** | 19005-3:2012 | ISO 32000-1 | Archiving + attachments | Like A-2, but **any** embedded file allowed with an `/AFRelationship` (basis of ZUGFeRD/Factur-X e-invoices) |
| **PDF/A-4** | 19005-4:2020 | ISO 32000-2 | Archiving on PDF 2.0 | Drops a/b/u. Conformance levels base, **F** (embedded files) and **E** (engineering, replacing PDF/E). Requires no `/Info` dictionary except where needed for dates |
| **PDF/X** | 15930-x (X-1a, X-3, X-4, X-5, X-6:2020) | PDF 1.3 → 2.0 | Print/prepress exchange | Fonts embedded, `/OutputIntents` mandatory, `/TrimBox` or `/ArtBox` required, color space limits (X-1a: CMYK/spot only), no encryption. X-4/X-6 allow live transparency and layers |
| **PDF/UA-1** | 14289-1:2012 (rev. 2014) | ISO 32000-1 | Accessibility | Fully tagged, logical reading order, alt text, language, no content conveyed by appearance alone. Machine-checkable rules defined by the Matterhorn Protocol |
| **PDF/UA-2** | 14289-2:2024 | ISO 32000-2 | Accessibility on PDF 2.0 | Uses PDF 2.0 structure namespaces. Aligned with the freely available **WTPDF** ("Well-Tagged PDF") spec published by the PDF Association |
| **PDF/E-1** | 24517-1:2008 | PDF 1.6 | Engineering docs (3D, geospatial) | Little adoption. Now covered by PDF/A-4e |
| **PDF/VT** | 16612-2:2010 (VT-1/2), 16612-3:2020 (VT-3) | PDF/X-4 / X-6 | Variable & transactional printing | Document Part Metadata (DPM) hierarchy, recipient record boundaries, XObject reuse for high-speed RIP caching |
| **PDF/R** | 23504-1:2020 | PDF subset | Raster scanning interchange | Restricted to raster image pages, for scanners |

The engineering impact is that a writer which wants to produce PDF/A or PDF/X needs a **policy layer**, a set of feature gates and required-entry checks, over the core writer. Validation should be done against **veraPDF** (the industry-supported open-source PDF/A and PDF/UA validator).

## 3. Lexical conventions

A PDF file is a sequence of **8-bit bytes**. The "text" parts are ASCII-compatible, but the format is binary. Never open a PDF in text mode or run it through newline conversion.

### 3.1 Character classes (§7.2.3; 32000-1 §7.2.2)

**White-space characters** (Table 1) are all equivalent outside strings and streams, except that EOL markers carry meaning in a few places.

| Dec | Hex | Name |
|---|---|---|
| 0 | 00 | NUL |
| 9 | 09 | HT (tab) |
| 10 | 0A | LF |
| 12 | 0C | FF |
| 13 | 0D | CR |
| 32 | 20 | SP |

**Delimiters** (Table 2): `(` `)` `<` `>` `[` `]` `{` `}` `/` `%`. `{` and `}` appear only in PostScript calculator functions (Type 4). A tokenizer must treat them as delimiters anyway. `<<` and `>>` are two-character tokens.

**Regular characters**: every other byte, including all bytes ≥ 0x80. A token made of regular characters runs until the next white-space or delimiter. So `/Type/Page` is two names, and `[1 2 3]` needs no space before `]`.

**End-of-line (EOL) markers**: CR, LF, or the pair CR LF (counted as one EOL). Places where the EOL type matters:

- After the `stream` keyword: CR LF or LF. **A lone CR is not allowed**, because a reader could not tell whether a following LF belongs to the data.
- Inside cross-reference table entries (exactly 2 bytes, see §5.3).
- Inside literal strings, where a raw CR or CR LF is normalized to LF (§4.3).

Keywords are **case-sensitive**: `obj`, `endobj`, `stream`, `endstream`, `xref`, `trailer`, `startxref`, `true`, `false`, `null`, `R`, `f`, `n`.

### 3.2 Comments (§7.2.4; 32000-1 §7.2.3)

A `%` outside a string or stream starts a comment that runs to the end of the line. The comment counts as a single white-space. Comments have no meaning to the object model, with two exceptions that are **file markers**:

- `%PDF-n.m`: the header (must be the first line).
- `%%EOF`: the end-of-file marker.

Comments are also ignored inside content streams, so `%` in content data cannot be used to smuggle anything structural. A writer should not rely on any comment surviving a rewrite.

### 3.3 Header and binary marker (§7.5.2)

```
%PDF-1.7\n
%âãÏÓ\n          ← bytes 25 E2 E3 CF D3 0A
```

- The header is `%PDF-` followed by the version: `1.0`…`1.7` or `2.0`. The spec says it is at byte 0.
- **Real-world tolerance:** Acrobat accepts the header anywhere within the **first 1024 bytes** (an implementation note in the old Adobe PDF Reference; ISO 32000-1 §7.5.2 has no such allowance and simply says the header is the first line). Other major readers do the same. This tolerance is the root of many polyglot tricks (§13).
- If the file contains binary data, which nearly every file does, the header line **shall** be followed immediately by a comment line containing **at least four bytes ≥ 128** (32000-1 §7.5.2). The purpose is historical: FTP clients, mail gateways and VCS tools decide "binary vs text" by sniffing the first bytes, and the marker stops them from rewriting EOLs. The sequence `%âãÏÓ` (E2 E3 CF D3) is the one Acrobat writes. Any four high bytes are fine.
- **Version override (PDF 1.4+):** the catalog's `/Version` name (e.g. `/1.7`) replaces the header's version **if it is later** (32000-1 §7.5.2 and Table 28). This lets an incremental update raise the version without rewriting byte 0.

## 4. The object model (§7.3)

PDF has **eight basic object types** plus the **null** object:

| Type | Example | Notes |
|---|---|---|
| Boolean | `true` `false` | |
| Integer | `42` `-17` `+0` | No exponent, no hex |
| Real | `3.14` `-.5` `4.` | Decimal point required. **No exponent notation** (`1e5` is invalid) |
| String (literal) | `(Hello \(world\))` | Byte sequence |
| String (hex) | `<48656C6C6F>` | Byte sequence |
| Name | `/Type` `/A#20B` | Atomic symbol |
| Array | `[1 (two) /Three [4]]` | Heterogeneous, 1-D, can nest |
| Dictionary | `<< /Key value >>` | Keys are names |
| Stream | dictionary + `stream`…`endstream` | Always indirect |
| Null | `null` | |

### 4.1 Booleans and null

`true`, `false` and `null` are keywords. A dictionary entry whose value is `null` is **equivalent to the entry being absent** (§7.3.7). A writer must not use `null` to mean anything else. A reader must treat `/Foo null` exactly as a missing key.

### 4.2 Numbers

- **Integers**: optional sign, decimal digits. **Reals**: optional sign, digits with exactly one `.`. Leading or trailing digits may be omitted (`.5`, `5.`).
- Wherever a real is expected, an integer is accepted, and the reverse holds where the spec allows it. In practice readers convert freely, and an integer value in a real slot is common and valid.
- **Limits.** ISO 32000-1 Annex C (Table C.1) gave the minimum limits a typical conforming reader on a 32-bit machine should accommodate, which writers should stay within:
  - integers −2³¹ … 2³¹−1
  - reals ±3.403×10³⁸, smallest non-zero ±1.175×10⁻³⁸, about 5 significant decimal digits of precision in the fractional part
  - string length 32,767 bytes, a limit that applies **only to strings in content streams**. Annex C says there is no effective limit on other strings
  - name length 127 bytes
  - 8,388,607 indirect objects
  - `q` nesting depth 28
  - 32 DeviceN colorants
  - CID values up to 65,535
  - page size 3–14,400 units (a recommendation in C.2, not part of Table C.1)
  - file size 10¹⁰ bytes when a classic xref table is used, because offsets have ten digits (the one limit C.2 calls an architectural limit of PDF itself; it does not apply to files using xref streams)
- ISO 32000-2 reportedly turned most of these from limits into portability advice (PDF 2.0 claim, not checked against the 2.0 text). Real files break them: there are 64-bit offsets in huge files, and some writers emit `1.0e-05` (invalid, though many readers accept it).
- **Writer rule:** use integers where possible. Print reals in fixed-point with bounded precision (e.g., up to 6 fractional digits, trailing zeros stripped). Never emit exponents, `NaN` or `Infinity`.
- **Reader rule:** accept `--5`, `5-`, `1e5` and `.` leniently. pdf.js, for example, treats `--` as a single `-` and logs a warning. Clamp values to sensible ranges.

### 4.3 Strings (§7.3.4)

A string is a **sequence of bytes**. Its interpretation (text, binary, date) depends on context, not on syntax.

**Literal strings** `( … )`:

- Parentheses must be balanced or escaped. `(a (b) c)` is valid and contains `a (b) c`.
- Escapes:

| Escape | Meaning |
|---|---|
| `\n` `\r` `\t` `\b` `\f` | LF, CR, HT, BS, FF |
| `\(` `\)` `\\` | literal `(`, `)`, `\` |
| `\ddd` | 1–3 **octal** digits; high-order overflow is ignored. At most three digits are read, so `\0053` is `\005` followed by `3` |
| `\` + EOL | line continuation: both are removed |
| `\` + any other char | the backslash is **ignored**, and the char is kept |

- A raw EOL inside a literal string (CR, LF or CR LF) is read as a **single LF**. To keep a CR, write `\r`. This bites writers that put binary data in literal strings. Use hex strings for binary, or escape CR.

**Hexadecimal strings** `< … >`:

- Hex digits in either case. White-space is ignored. If the number of digits is odd, a final `0` is assumed (`<901FA>` = `90 1F A0`).
- Hex strings are about twice the size of the raw bytes. Use them for binary data such as `/ID`, and inside signatures, where `/Contents` must be a fixed-width hex placeholder.

**Kinds of strings by usage** (§7.9.2):

- *Byte strings*: arbitrary binary.
- *Text strings*: human-readable. Encoded as **PDFDocEncoding**, or **UTF-16BE with BOM `FE FF`**, or (PDF 2.0) **UTF-8 with BOM `EF BB BF`**. Readers must handle all three, and a missing or incorrect BOM in the wild. Writers should use PDFDocEncoding for pure ASCII and UTF-16BE otherwise. UTF-8 is unsafe for files that pre-2.0 readers will open.
- *ASCII strings*, and *date strings* (§7.9.4): `D:YYYYMMDDHHmmSSOHH'mm`. The Adobe PDF Reference (up to 1.7) put a trailing `'` after the offset minutes. ISO 32000-1 §7.9.4 already dropped it, and PDF 2.0 keeps the form without it. Readers must accept both forms and partial dates like `D:2026` (only `D:` and the year are required; MM and DD default to 01).

**Encryption note:** when a document is encrypted, every string and stream in the body is encrypted, *except* the strings in the `/Encrypt` dictionary, the `/ID`, cross-reference streams, and strings inside streams such as content streams and object streams (the stream as a whole is encrypted instead) (32000-1 §7.6.1, §7.5.8.2). A parser must therefore know which object a string belongs to (object number and generation) to decrypt it. That is one reason decryption belongs in the object loader, not the tokenizer.

### 4.4 Names (§7.3.5)

- `/` followed by regular characters. `/` alone is a valid **empty name**.
- Since PDF 1.2, any byte can be written as `#xx` (two hex digits). Regular characters outside `!`…`~` (0x21–0x7E) **should** be written this way. `#` itself, delimiters and white-space **must** be. `/A#20B` is the name `A B`. NUL (`#00`) is not allowed.
- A name is an atomic **byte sequence**. Comparison is byte-wise after decoding `#xx`, so `/Type` and `/Typ#65` are the same name. The spec recommends interpreting names as UTF-8 when they must be shown to users. The only common case is font and separation names.
- Max length is 127 bytes per the 1.7 limits. Readers should be lenient.

### 4.5 Arrays and dictionaries (§7.3.6–7.3.7)

- **Array**: `[` objects `]`. Elements can be of any type, including other arrays.
- **Dictionary**: `<<` (name value)* `>>`. Keys **must be names**. Values can be any object, including indirect references.
- **Duplicate keys** are not allowed ("shall not", §7.3.7). The spec does not say what a reader should do with them. Most readers keep the last one, some keep the first, and parser differentials like this are an attack surface (§13).
- **Conventions:** `/Type` names the object kind (`/Page`, `/Font`, …). `/Subtype` refines it. Many `/Type` entries are formally optional, but readers rely on them for repair, so writers should always emit them.
- **Unknown keys are ignored.** This is not in §7.3.7. In 32000-1 it is Annex I.3: readers behave as if entries they don't know were not there, which is what makes forward compatibility and private extensions possible. Private keys should use a registered **second-class name prefix** (ISO 32000-2 Annex E; 32000-1 Annex E, "PDF Name Registry") to avoid collisions.

### 4.6 Streams (§7.3.8)

```
<< /Length 43 /Filter /FlateDecode >>
stream⏎                ← EOL here must be CR LF or LF (not CR alone)
…exactly /Length bytes of (encoded) data…
⏎endstream              ← an EOL before endstream SHOULD be present, NOT counted in /Length
```

- A stream is a **dictionary followed by a byte sequence**. Streams **must be indirect objects** (inside `n g obj … endobj`). A direct stream would make the object model ambiguous.
- `/Length` (required) is the number of bytes between the EOL after `stream` and the EOL before `endstream`. It **may be an indirect reference** (`/Length 6 0 R`). This lets a single-pass writer stream compressed output without knowing its size in advance, then write `6 0 obj 12345 endobj` afterwards.
- Other common stream dictionary keys:

| Key | Meaning |
|---|---|
| `/Filter` | name or array of filter names (§9) |
| `/DecodeParms` | dict or array of dicts/nulls, one per filter |
| `/F`, `/FFilter`, `/FDecodeParms` | (PDF 1.2) data stored in an **external file**. The bytes between `stream` and `endstream` are then ignored, and `/Length` should still count them (usually 0). Rare, and a security concern |
| `/DL` | (PDF 1.5) decoded length hint. Use it for bomb limits, but don't trust it |

- Content streams, images, fonts, ICC profiles, XMP metadata, embedded files, object streams and cross-reference streams are all streams.

### 4.7 Indirect objects and references (§7.3.10)

```
12 0 obj          ← object number 12, generation 0
  << /Type /Page … >>
endobj

/Parent 12 0 R    ← an indirect reference to it
```

- **Object number**: positive integer. Object 0 is reserved as the head of the free list.
- **Generation number**: non-negative integer up to 65,535. All objects in a newly written file have generation 0. The generation only grows when an object number is freed and reused in an incremental update (§5.3).
- The pair (number, generation) **identifies** the object. A reference to an undefined object **is a reference to the null object** and is not an error (§7.3.9, §7.3.10). The spec states this for nonexistent objects; readers apply the same rule to references whose generation doesn't match the xref entry or that point to a free entry.
- **Direct versus indirect:** any object may be indirect, but some **must** be:
  - streams
  - objects shared from several places (pages referenced from `/Kids` and from destinations)
  - many structural objects (pages, page tree nodes, annotations)
  - `/Length` values that are not known up front

  Some **must be direct**, for example stream dictionaries themselves (§7.3.8.1), the Table 17 entries of cross-reference stream dictionaries (§7.5.8.2), the values in the linearization dictionary (Annex F.3.3), and the trailer `/ID` array and its strings when `/Encrypt` is present (Table 15).
- **The tokenizer has to look ahead:** `1 0 R` and `1 0 obj` both start with two integers. When a parser sees an integer it must look ahead up to two more tokens to tell whether this is a number, a reference, or an object header. Every PDF lexer has this quirk.

## 5. Physical file structure (§7.5)

A conventional ("classic") PDF has four parts:

```
+-------------------------------+  offset 0
| Header        %PDF-1.7        |
|               %âãÏÓ           |
+-------------------------------+
| Body          1 0 obj … endobj|
|               2 0 obj … endobj|
|               …               |
+-------------------------------+  ← byte offset X
| Cross-reference table         |
|   xref                        |
|   0 6                         |
|   0000000000 65535 f␍␊        |
|   0000000015 00000 n␍␊        |
|   …                           |
+-------------------------------+
| Trailer                       |
|   trailer                     |
|   << /Size 6 /Root 1 0 R … >> |
|   startxref                   |
|   X                           |
|   %%EOF                       |
+-------------------------------+
```

All offsets are **byte offsets from the start of the file** (byte 0 is the `%` of `%PDF`). Some readers measure offsets relative to the header when junk comes before it, so a reader has to guess which convention a broken file used (§11).

### 5.1 Body

The body holds the indirect objects in any order. Object numbers do not have to be contiguous or ascending, though writers usually emit them that way. Nothing between objects matters except white-space and comments. Files with junk between objects are common, and lenient readers skip it.

### 5.2 The `startxref` / `%%EOF` tail

```
startxref
416
%%EOF
```

- `startxref` is followed by the byte offset of the **last** cross-reference section. That is either the `xref` keyword of a table, or the `n g obj` of a cross-reference stream.
- `%%EOF` ends the file, and each trailer of an incremental update ends with its own `%%EOF` (§7.5.6). Acrobat only searches the **last 1024 bytes** for `%%EOF` (an old Adobe implementation note, not part of ISO 32000-1). Garbage after `%%EOF` (padding added by mail gateways or web servers) is common, so readers search backwards for `startxref` rather than assuming exact positions.

### 5.3 The cross-reference table (§7.5.4)

The table maps object numbers to byte offsets. It is made of one or more **subsections**, each starting with a line `first count`:

```
xref
0 6                      ← subsection: objects 0..5
0000000000 65535 f␍␊
0000000015 00000 n␍␊
0000000064 00000 n␍␊
0000000121 00000 n␍␊
0000000253 00000 n␍␊
0000000323 00000 n␍␊
```

**Each entry is exactly 20 bytes**, so entry *k* of a subsection can be found by arithmetic:

```
nnnnnnnnnn ggggg n eol
└──10────┘ └─5─┘ │ └2┘
 offset    gen   type   2-byte EOL: SP CR, SP LF, or CR LF
  (or next free obj#)  ('n' in use, 'f' free)
```

- For **in-use (`n`)** entries, `nnnnnnnnnn` is the 10-digit, zero-padded byte offset of the `n g obj` line, and `ggggg` is the 5-digit generation.
- For **free (`f`)** entries, `nnnnnnnnnn` is the **object number of the next free object**, and `ggggg` is the generation number to use **if this number is reused**.
- **The free list is a linked list.** It starts at object 0, which is always free with generation 65535, and the last free entry links back to 0. When an object is deleted, its generation is incremented and it is linked into the list. An entry whose generation reaches 65535 is never reused.
- In practice, many writers emit free entries with garbage links. Readers **must not depend on the free list**. It only matters for incremental-update writers that want to reuse numbers, and they don't need to: allocating a new number past `/Size` is always valid.
- **Subsections** let incremental updates list only the objects that changed:

```
xref
0 1
0000000000 65535 f␍␊
3 1
0000025325 00000 n␍␊
23 2
0000025518 00002 n␍␊
0000025635 00000 n␍␊
```

- **Common malformations:**
  - entries of 19 bytes (single-byte EOL) or 21 bytes (CR CR LF)
  - a first subsection header `1 N` when the entries actually start at object 0. This "off-by-one" is common enough that pdf.js, MuPDF and others detect it by checking whether the first entry looks like `0000000000 65535 f`
  - offsets that are wrong by a constant (prepended junk, or EOL conversion during transfer)

### 5.4 The trailer dictionary (§7.5.5, Table 15)

```
trailer
<< /Size 6 /Root 1 0 R /Info 7 0 R /ID [<a1…> <a1…>] >>
```

| Key | Type | Meaning |
|---|---|---|
| `/Size` | integer | **One greater than the highest object number** used in this file across all sections. It is not a count of objects. Required. Must not be a reference |
| `/Prev` | integer | Byte offset of the **previous** cross-reference section. Present only when the file has more than one section (incremental update, or linearized). Must be direct (32000-1 Table 15 literally says "shall be an indirect reference", a known typo) |
| `/Root` | dict ref | The **document catalog**. Required, must be indirect |
| `/Encrypt` | dict | Encryption dictionary. Present if and only if the document is encrypted |
| `/Info` | dict ref | Document information dictionary (§14.3.3). Must be indirect. Deprecated in 2.0 except for the dates |
| `/ID` | array of 2 byte strings | File identifier (§14.4). **Required in PDF 2.0** and whenever `/Encrypt` is present; with `/Encrypt` the array and strings must be direct and unencrypted. First element: permanent identifier from creation. Second element: changes on each update. Used as input to encryption key derivation and for matching file references. 32000-1 §14.4 suggests an MD5 digest of the current time, file path, file size and the `/Info` values (16 bytes); it need not be reproducible |
| `/XRefStm` | integer | Hybrid files only: offset of a cross-reference stream (§6.3) |

In an incremental update, each trailer **repeats all keys** of the previous trailer except `/Prev`, whether their values changed or not (changed keys carry their new values), and adds a new `/Prev` (§7.5.6). A reader uses the newest trailer's `/Root`, `/Info`, `/Encrypt` and `/ID`.

## 6. Cross-reference streams, object streams, hybrid files (§7.5.7–7.5.8)

PDF 1.5 added two features that work together. Together they typically shrink structure-heavy files (many small dictionaries: tagged PDF, forms) by 20–60%.

### 6.1 Object streams (`/Type /ObjStm`)

An object stream packs many non-stream objects into one (usually Flate-compressed) stream:

```
6 0 obj
<< /Type /ObjStm /N 4 /First 20 /Filter /FlateDecode /Length 163 >>
stream
…compressed bytes…
endstream
endobj
```

Decoded content (taken from our verified example in §12.4):

```
1 0 2 30 3 66 4 163 <</Type/Catalog/Pages 2 0 R>> <</Type/Pages/Kids[3 0 R]/Count 1>> <</Type/Page …>> <</Type/Font …>>
└────── header ────┘└── object data starts at /First = 20 ──────────────────────────────────────────────────────────────
pairs: (objnum, offset relative to /First)
```

- `/N`: number of objects. `/First`: byte offset of the first object within the decoded data. `/Extends`: optional reference to another object stream that this one logically extends (rarely used).
- Objects inside are written **without** `n g obj` / `endobj`. Their generation is implicitly 0.
- **What cannot go in an object stream:**
  - stream objects
  - objects with generation ≠ 0
  - the document's encryption dictionary
  - an object that is the `/Length` of an object stream
  - in **linearized** files: the document catalog, the linearization dictionary, and page objects
  - the cross-reference stream itself (it is a stream anyway)

  Also, an object in an object stream must not consist solely of an object reference, and a freed object-stream or compressed-object number may be reused only for an ordinary uncompressed object (§7.5.7).
- A reader resolving an object inside an object stream must decode the whole stream. Cache decoded object streams (LRU), because random access into them is expensive.

### 6.2 Cross-reference streams (`/Type /XRef`)

A cross-reference stream replaces both the `xref` table **and** the `trailer`. Its dictionary carries the trailer keys (`/Size /Root /Info /ID /Encrypt /Prev`), and `startxref` points at the stream object:

```
7 0 obj
<< /Type /XRef /Size 8 /W [1 2 1] /Root 1 0 R /Filter /FlateDecode /Length 35 >>
stream
…binary rows…
endstream
endobj
startxref
362
%%EOF
```

| Key | Meaning |
|---|---|
| `/W` | Array of 3 integers: **byte widths** of the three fields of each row. A width of 0 means the field is absent and takes its default (type → **1**, field 3 → 0). Required |
| `/Index` | `[first1 count1 first2 count2 …]`: the subsections. Default `[0 Size]` |
| `/Size` | As in the trailer |
| `/Prev` | Offset of the previous xref section (table or stream) |

Rows are fixed-width, **big-endian** binary integers. After decoding (often with a PNG Up predictor, `/DecodeParms << /Predictor 12 /Columns 5 >>`, to make the delta-like data compress better), each row means:

| Type (field 1) | Field 2 | Field 3 | Meaning |
|---|---|---|---|
| **0** | next free object number | generation if reused | free entry |
| **1** | byte offset of object | generation | ordinary in-use object |
| **2** | object number **of the object stream** containing it | **index** of the object within that stream | compressed object (generation implicitly 0) |
| other | — | — | reserved: treat as a reference to null |

Decoded rows of the verified example (W = [1 2 1]):

```
00 0000 ff   obj 0: free (gen 255: the largest value one byte can hold)
02 0006 00   obj 1: in object stream 6, index 0
02 0006 01   obj 2: in object stream 6, index 1
02 0006 02   obj 3: in object stream 6, index 2
02 0006 03   obj 4: in object stream 6, index 3
01 000f 00   obj 5: at offset 15, gen 0
01 006a 00   obj 6: at offset 106, gen 0   (the object stream)
01 016a 00   obj 7: at offset 362, gen 0   (the xref stream itself)
```

Rules worth knowing:

- The cross-reference stream needs an xref entry like any indirect object, usually in itself (in hybrid files it can be in the classic table) (§7.5.8.3).
- It is **never encrypted**, even in encrypted documents, because the reader needs it before it has the key. It must not use a `Crypt` filter (§7.5.8.2).
- The Table 17 entries (`/Type /Size /Index /Prev /W`), and `/Filter` and `/DecodeParms`, must be direct objects, because they are needed before any other object can be resolved. Other entries may be indirect, and `/Root` must be (§7.5.8.2).
- The field widths are chosen by the writer. Use the minimum widths that fit (offset width = ceil(log256(file size))). Choosing `/W [1 8 2]` everywhere wastes space but is valid.

### 6.3 Hybrid-reference files (§7.5.8.4)

Hybrid files were a transition mechanism, so that pre-1.5 readers could still open files that use object streams:

- The file has a **classic xref table and trailer** that list only the objects a 1.4 reader can use, i.e., everything *not* in object streams.
- The trailer has `/XRefStm offset` pointing to a cross-reference stream that lists the compressed objects. (32000-1 §7.5.8.4 words this as allowed only in an update section's trailer, not the main section's; files in the wild don't always follow that.) The objects hidden from old readers appear as free entries with generation 65535 in the classic table.
- **Lookup order for 1.5+ readers within one section:** (1) the classic table, (2) the stream named by `/XRefStm`, (3) then follow `/Prev`. Pre-1.5 readers ignore `/XRefStm` and never see the compressed objects. The writer must make sure the content they can see is still a usable document.

Hybrid files are now mostly legacy (Word "Save as PDF" still produces some). lamina should read them correctly but has no reason to write them.

## 7. Incremental updates (§7.5.6)

A PDF can be modified **by appending**, without touching existing bytes:

```
+------------------+
| original header  |
| original body    |
| original xref    |  ← xref section 1
| original trailer |
| %%EOF            |
+------------------+  ← everything above is byte-for-byte unchanged
| new/changed objs |  (whole replacement objects, same obj# and gen)
| new xref section |  ← lists ONLY changed objects (+ obj 0)
| trailer          |
|  << … /Prev X1 >>|  ← points back at section 1
| startxref X2     |
| %%EOF            |
+------------------+
```

Resolution rule: the reader starts at the **last** `startxref`, loads that section, then follows `/Prev` to older sections. **For each object number, the first (newest) entry seen wins.** A deleted object is written as a free entry with an incremented generation in the new section, which hides the old one.

We checked this with poppler and pypdf. `hello-incremental.pdf` appends a new object 5 (content stream reading "Hello, Update") plus this update section to the 599-byte `hello.pdf`. Both `pdftotext` and pypdf (strict mode) extract "Hello, Update":

```
xref
0 1
0000000000 65535 f␍␊
5 1
0000000599 00000 n␍␊
trailer
<< /Size 6 /Root 1 0 R /Prev 416 >>
startxref
693
%%EOF
```

**Why updates exist and why they matter:**

1. **Speed and safety of saving.** Appending is O(changes) instead of O(file). Acrobat's plain "Save" appends, and "Save As" rewrites. The old version can be recovered by truncating after the earlier `%%EOF`. This is a known source of **information leaks**: "redacted" content that is still present in the earlier revision.
2. **Digital signatures depend on them.** A signature's `/ByteRange [0 a b c]` covers the whole file as it was at signing time, except the `/Contents` hex placeholder. Any later change (a second signature, filled form fields, annotations) **must** be appended as an incremental update, or the first signature's digest breaks. Validators check that the signed byte range ends at a `%%EOF` and then decide whether the appended changes are allowed under the document's **DocMDP/FieldMDP** permissions. Details are in the Signatures chapter.
3. **Editors that preserve signatures** must therefore support *append mode*: serialize only dirty objects, keep the original bytes, write an update section using the same kind of xref as the previous section (a stream if it used streams), carry forward the trailer keys, and point `/Prev` at the previous `startxref` value. Whole-file rewriters (qpdf/pikepdf, `mutool clean`) invalidate existing signatures. Apache PDFBox (`PDDocument.saveIncremental`) and iText (`StampingProperties.useAppendMode()`) provide explicit append modes.

Writer checklist for an update:

- New object numbers start at the old `/Size`. Reused numbers must use the incremented generation from the free list.
- The new `/Size` = max(old `/Size`, highest new number + 1).
- Copy `/Root`, `/Info`, `/ID[0]` and `/Encrypt` forward. Update `/ID[1]`.
- If the file is encrypted, encrypt new objects with the existing key.
- A linearized file stops being linearized after an update. Readers detect this because the linearization dictionary's `/L` no longer matches the file length.

## 8. Linearization ("Fast Web View", Annex F)

A **linearized** PDF is arranged so that a viewer can show the first page before the rest of the file has downloaded, and then fetch other pages with HTTP byte-range requests. It is fully backward compatible: a non-linearization-aware reader sees a valid PDF with an extra xref section.

### 8.1 Layout

```
 1  Header
 2  Linearization parameter dictionary   (first object, within the first 1024 bytes)
 3  First-page cross-reference section + trailer
 4  Document catalog and other document-level objects
 5  Primary hint stream (may be here or after the first page)
 6  First page section: page object of page 1 + everything it needs
 7  Remaining pages (each page's private objects, page by page)
 8  Shared objects for all other pages
 9  Other objects (not page-related)
10  Overflow hint stream (optional)
11  Main cross-reference section + trailer
```

The linearization dictionary:

```
43 0 obj
<< /Linearized 1        % version of linearization (1.0)
   /L 54321             % total file length in bytes — if this ≠ actual length, the file is NOT linearized any more
   /H [ 1025 512 ]      % offset & length of primary hint stream (optionally 4 numbers incl. overflow)
   /O 45                % object number of first page's page object
   /E 9876              % offset of end of first page section
   /N 12                % number of pages
   /T 53000             % offset of the white-space before the first entry of the main xref table; with xref streams only, offset of the main xref stream
   /P 0                 % (optional) page number of the first page, 0-based
>>
endobj
```

**Hint tables** (in the hint stream) are bit-packed tables. They give, per page, the object count, the byte length, and the shared objects it references (**page offset hint table**), plus the locations of shared object groups (**shared object hint table**). Optional tables (32000-1 Table F.2 and F.4.6) cover thumbnails, outlines, article threads, named destinations, the info dictionary, interactive forms, logical structure, page labels and embedded file streams. Their format is fiddly: deltas from minimum values and bit widths stored in headers. The old Adobe PDF Reference also listed implementation notes where Acrobat deviates from the hint-table spec; ISO 32000-1 Annex F does not carry them.

### 8.2 Why it is rarely worth implementing for writing

- **Networks and readers changed.** Range requests work on any PDF: a reader can fetch the tail (`startxref`), then the xref, then the objects it needs. pdf.js does exactly this with range requests and does not need linearization to stream large documents. Linearization only saves the first round trip or two.
- **It is fragile.** Any incremental update, including a signature, breaks it. Many "linearized" files in the wild have wrong hint tables. Readers must check `/L` against the real length and fall back silently.
- **It constrains the writer.** Object ordering is forced, the catalog and page objects cannot go in object streams, page attributes may not be inherited from the page tree (§7.7.3.4), and the xref has to be written twice. Getting hint tables exactly right is hard. qpdf is the reference open-source implementation (`qpdf --linearize`, `--check-linearization`).

**Recommendation for lamina:** the reader must *tolerate* linearized files, which needs nothing special because the file is valid without the hints. It can optionally use `/Linearized` + `/O` + `/E` to render page 1 early over a slow transport. Writing linearized output should come late, probably as an optional post-processing step. Note also that `/Linearized` must be a number in the first object; a stray `/Linearized` dictionary later in the file means nothing.

## 9. Stream filters (§7.4)

`/Filter` names one or more **decoding** filters. With an array, they are applied **in order** when decoding. `/Filter [/ASCII85Decode /FlateDecode]` means: ASCII85-decode first, then inflate. The data was encoded in the reverse order. `/DecodeParms` is a dict, or an array parallel to `/Filter` with `null` where a filter has no parameters.

| Filter | Since | Kind | Key parameters | Notes |
|---|---|---|---|---|
| `ASCIIHexDecode` | 1.0 | text ↔ binary | — | Hex pairs, white-space ignored, `>` = EOD, odd final digit padded with 0 |
| `ASCII85Decode` | 1.0 | text ↔ binary | — | Base-85, 4 bytes → 5 chars `!`..`u`, `z` = four zero bytes, `~>` = EOD, partial final group |
| `LZWDecode` | 1.0 | general | `Predictor`, `Colors`, `BitsPerComponent`, `Columns`, `EarlyChange` (default 1) | 9–12-bit codes, codes 256 = clear, 257 = EOD. Forbidden in PDF/A. Don't write it |
| `FlateDecode` | 1.2 | general | `Predictor`, `Colors`, `BitsPerComponent`, `Columns` | **zlib** stream (RFC 1950 header + RFC 1951 deflate + Adler-32). The workhorse |
| `RunLengthDecode` | 1.0 | general | — | Length byte L: 0–127 → copy next L+1 bytes. 129–255 → repeat next byte 257−L times. 128 = EOD |
| `CCITTFaxDecode` | 1.0 | bilevel image | `K` (<0 G4, 0 G3-1D, >0 G3-2D), `Columns` (1728), `Rows`, `BlackIs1`, `EncodedByteAlign`, `EndOfLine`, `EndOfBlock`, `DamagedRowsBeforeError` | Scanned documents. Note: `BlackIs1` default false means 0 bits are black |
| `JBIG2Decode` | 1.4 | bilevel image | `JBIG2Globals` (stream with shared segments) | Embedded JBIG2 stream organization (no file header). Its complexity has led to famous exploits (§13) |
| `DCTDecode` | 1.0 | image | `ColorTransform` | Baseline/progressive **JPEG**. Adobe APP14 marker overrides `ColorTransform`. Embed JPEG files unchanged |
| `JPXDecode` | 1.5 | image | — | **JPEG 2000** (ISO 15444-1) codestream or JP2. Color space and `SMaskInData` can come from the codestream |
| `Crypt` | 1.5 | security | `Type /CryptFilterDecodeParms`, `Name` | Selects a named crypt filter for this stream. **Must be first** in the array. `/Identity` = unencrypted (e.g., to leave XMP metadata readable) |
| `BrotliDecode` | ext. | general | (per extension) | PDF Association extension, not yet in ISO 32000 (§1). Read it. Don't write it yet |

The image-only filters (CCITT, JBIG2, DCT, JPX) are defined for image data. A decoder for them returns pixels, or can pass compressed data straight through to a renderer or exporter. Inline images in content streams use abbreviated names (`AHx`, `A85`, `LZW`, `Fl`, `RL`, `CCF`, `DCT`). These are covered in the content-stream chapter.

### 9.1 Predictors (Flate and LZW)

Predictors make image-like data compress better by storing differences between neighboring bytes:

| `/Predictor` | Meaning |
|---|---|
| 1 (default) | none |
| 2 | **TIFF Predictor 2**: horizontal differencing per component, at `BitsPerComponent` granularity (1, 2, 4, 8, 16 bits) |
| 10–15 | **PNG predictors**: *each row starts with a filter-type byte* (0 None, 1 Sub, 2 Up, 3 Average, 4 Paeth). The value 10–15 is only a hint about what the encoder used. **The per-row byte decides**, so a decoder treats 10–15 identically |

Parameters: `Colors` (default 1), `BitsPerComponent` (default 8), `Columns` (default 1).

- Bytes per pixel = ceil(Colors × BPC / 8). Paeth and Sub use this distance.
- Row length = ceil(Colors × BPC × Columns / 8), plus 1 tag byte for PNG predictors.
- The typical cross-reference stream setting is `/Predictor 12 /Columns <sum of W>` (PNG Up).

### 9.2 Practical decoding robustness

- **Flate:**
  - Accept raw deflate with no zlib header (some writers emit it).
  - Ignore Adler-32 mismatches.
  - On a truncated or corrupt stream, **return what was decoded so far**. Acrobat, pdf.js and MuPDF all do this, and many damaged files still render.
  - Some streams are compressed twice or labeled wrongly. Sniff the `78 xx` zlib header when the declared filter fails.
- **Filter names:** accept the abbreviations in stream dictionaries as well, as Acrobat does (the spec allows them only in inline images, §8.9.7), and accept unknown filters by passing the data through raw (or failing only that object).
- **`/Length`:** treat it as a hint. If the bytes at `start + Length` are not (EOL+)`endstream`, search forward for `endstream` and use that. This is very common in the wild.
- **Limits:** enforce a maximum decoded size per stream and per document (§13).

## 10. Document logical structure (§7.7)

```
trailer
  └─ /Root ──► Catalog (/Type /Catalog)
                ├─ /Pages ─────► Page tree root (/Type /Pages)
                │                  ├─ /Kids [ … ]
                │                  │    ├─ Pages node ─► /Kids [Page, Page, …]
                │                  │    └─ Page (/Type /Page)
                │                  │         ├─ /Resources ─► /Font /XObject /ExtGState …
                │                  │         ├─ /Contents ──► content stream(s)
                │                  │         └─ /Annots ────► annotations
                │                  └─ /Count N
                ├─ /Outlines ──► outline tree (bookmarks)
                ├─ /Names ─────► name trees (/Dests /EmbeddedFiles /JavaScript /AP …)
                ├─ /Dests ─────► (1.1-style named destinations dict)
                ├─ /PageLabels ► number tree
                ├─ /Metadata ──► XMP stream
                ├─ /AcroForm ──► interactive form
                ├─ /StructTreeRoot, /MarkInfo, /Lang  (tagged PDF)
                ├─ /OCProperties (layers), /OutputIntents, /Extensions, /Version
                └─ /OpenAction, /AA, /ViewerPreferences, /PageMode, /PageLayout, /AF, /DSS, /DPartRoot …
```

### 10.1 The catalog

`/Type /Catalog` and `/Pages` are required. Everything else is optional. Entries most relevant to a parser:

- `/Version`: overrides the header if later.
- `/Extensions`: developer extensions, e.g., `<< /ADBE << /BaseVersion /1.7 /ExtensionLevel 3 >> >>`. ISO-defined extensions use the `/ISO_` prefix and, in PDF 2.0, an array of extension dictionaries per prefix.
- `/Names`: the name dictionary (§10.5).
- `/Metadata`: XMP metadata (§10.6).
- `/StructTreeRoot`, `/MarkInfo << /Marked true >>`, `/Lang`: tagged PDF, the basis of accessibility and reliable text extraction.
- `/AcroForm`, `/OCProperties`, `/OpenAction`, `/AA`: forms, layers and actions. The last two are a security surface: lamina should parse them without executing anything.

### 10.2 The page tree (§7.7.3)

Pages are **not** stored as a flat array. They are a tree of **page tree nodes** (`/Type /Pages`) whose leaves are **page objects** (`/Type /Page`):

| Key (Pages node) | Meaning |
|---|---|
| `/Kids` | array of indirect references to child Pages nodes or Page objects |
| `/Count` | number of **leaf pages** under this node (not the number of kids) |
| `/Parent` | parent node (required everywhere except the root) |

Why a tree: insertion and deletion are cheap, and a reader can find page *n* in O(log n) by going down and using `/Count` to skip subtrees without loading them. Writers commonly use fan-outs of around 8–32 kids per node. Writing every page as a direct child of one root is valid and very common, but forces readers to load a huge `/Kids` array.

**Inheritable attributes** are looked up through the `/Parent` chain if the page itself lacks them:

| Attribute | Notes |
|---|---|
| `/Resources` | required (via inheritance if needed) |
| `/MediaBox` | required (via inheritance if needed) |
| `/CropBox` | defaults to MediaBox |
| `/Rotate` | multiple of 90, clockwise rotation for display. Readers should normalize negatives and values ≥ 360 and treat non-multiples of 90 as 0 |

Nothing else is inheritable. In particular, `/Contents` and `/Annots` are not.

**Reader robustness for page trees:**

- `/Count` is often wrong. Never allocate based on it without a cap.
- Cycles in `/Kids` are a classic DoS. Track visited nodes.
- A missing or wrong `/Type`: guess from shape (has `/Kids` → node, has `/Contents` or `/MediaBox` → page).
- `/Parent` links may be wrong. Inheritance should follow the path actually taken *down* the tree, not `/Parent` pointers.
- If the tree is damaged beyond repair, a last resort is to collect all `/Type /Page` objects in object-number order (MuPDF and pdf.js both have fallbacks of this kind).

### 10.3 Page objects

Required keys: `/Type /Page`, `/Parent`, `/MediaBox` and `/Resources` (the last two may be inherited). `/Contents` is optional: a page without it is empty. Other optional keys:

- `/CropBox`, `/BleedBox`, `/TrimBox`, `/ArtBox`, `/BoxColorInfo`
- `/Rotate`, `/UserUnit` (1.6: default user space unit in multiples of 1/72 inch, to go beyond the 200-inch practical limit)
- `/Annots`, `/Group` (transparency group), `/Thumb`, `/B` (beads), `/Dur`, `/Trans`, `/AA`
- `/Metadata`, `/StructParents`, `/Tabs`, `/VP` (viewports, used for geospatial), `/AF`, `/OutputIntents` (2.0), `/DPart` (2.0)

**`/Contents`** is a stream or an **array of streams**. An array is equivalent to the concatenation of the streams with white-space between them. Splits may occur only at lexical token boundaries, although readers should tolerate splits inside tokens produced by sloppy writers. Writers must not write an empty `/Contents` array (Table 30).

### 10.4 Page boundaries (§14.11.2)

All five are rectangles `[llx lly urx ury]` in default user space (1 unit = 1/72 in × `UserUnit`). A rectangle may be given by any two opposite corners, so readers normalize to (min, max).

```
 ┌───────────────────── MediaBox ── physical medium (required)
 │ ┌─────────────────── CropBox ─── visible region in viewers; default = MediaBox
 │ │ ┌───────────────── BleedBox ── clip region in production (incl. bleed); default = CropBox
 │ │ │ ┌─────────────── TrimBox ─── finished page after trimming; default = CropBox
 │ │ │ │ ┌───────────── ArtBox ──── meaningful content (for imposition/placement); default = CropBox
 │ │ │ │ │
```

All the other boxes are **clipped to the MediaBox**. Viewers display and print the CropBox. PDF/X requires a TrimBox or an ArtBox. Note that the MediaBox need not start at (0,0). Offsets like `[0 0 612 792]` vs `[-306 -396 306 396]` both occur, and text extraction and coordinate mapping must apply the box origin and `/Rotate`.

### 10.5 Resources, name trees, number trees

**Resource dictionary** (§7.8.3): maps names used by content streams to objects:

```
/Resources <<
  /Font      << /F1 4 0 R >>
  /XObject   << /Im1 9 0 R /Fm1 10 0 R >>
  /ExtGState << /GS1 11 0 R >>
  /ColorSpace, /Pattern, /Shading, /Properties
  /ProcSet [/PDF /Text /ImageB /ImageC /ImageI]   % obsolete since 1.4; ignore on read (§14.2)
>>
```

ISO 32000-1 §14.2 still says writers *should* keep emitting `/ProcSet` (all five names unless fewer are known to suffice) for old readers, while readers must not depend on it. Omitting it is harmless in practice.

Form XObjects, patterns, Type 3 fonts and annotation appearance streams have their **own** `/Resources`. Files written to earlier PDF versions could omit them from forms and Type 3 fonts and inherit the page's resources. 32000-1 §7.8.3 calls this obsolete, but readers should still fall back to the page's resources.

**Name trees** (§7.9.6): sorted maps from string keys to objects, stored as a B-tree-like structure:

```
root: << /Kids [ 20 0 R 21 0 R ] >>
20:   << /Limits [(Apple) (Mango)] /Names [ (Apple) 30 0 R (Banana) 31 0 R … ] >>
21:   << /Limits [(Orange) (Zebra)] /Kids [ … ] >>
```

- Keys are **byte strings sorted in byte order** (not locale order, not decoded text). Shorter keys come before longer keys that start with the same bytes, and equality is byte-by-byte (§7.9.6).
- `/Limits [least greatest]` lets a reader binary-search without loading every node.
- Uses (in the catalog `/Names` dict): `/Dests`, `/AP`, `/JavaScript`, `/Pages`, `/Templates`, `/IDS`, `/URLS`, `/EmbeddedFiles`, `/AlternatePresentations`, `/Renditions`.
- **Number trees** (§7.9.7) are the same but with integer keys and `/Nums`. They are used for `/PageLabels` and the structure tree's `/ParentTree`.
- **Robustness:** unsorted keys and wrong `/Limits` are common. When a lookup fails, fall back to a full scan. Guard against cycles.

### 10.6 `/Info` dictionary vs XMP metadata

| | Document Info dictionary | XMP metadata stream |
|---|---|---|
| Location | trailer `/Info` | catalog `/Metadata` (also on pages, images, fonts…) |
| Format | PDF dict of text strings: `/Title /Author /Subject /Keywords /Creator /Producer /CreationDate /ModDate /Trapped` | `/Type /Metadata /Subtype /XML` stream of RDF/XML (ISO 16684-1) |
| Status | **Deprecated in PDF 2.0** (except the dates) | Normative in 2.0. Required by PDF/A and PDF/UA |
| Extensible | custom keys, untyped | namespaced schemas (`dc:`, `xmp:`, `pdf:`, `xmpMM:`, `pdfaid:`, …) |

Writers targeting broad compatibility should write both and keep them in sync. The PDF/A mapping fixes the pairs: `Title`↔`dc:title`, `Author`↔`dc:creator`, `Producer`↔`pdf:Producer`, `CreationDate`↔`xmp:CreateDate`, and so on. Leave XMP streams **uncompressed** (or use `/Crypt /Identity` in encrypted files) so that file-level metadata scanners can find them. On read, PDF 2.0 readers should prefer XMP when the two disagree. In practice, `/Info` is often the more up-to-date one, because many tools update only `/Info`. Compare `ModDate` with `xmp:ModifyDate`.

### 10.7 Document outline (bookmarks, §12.3.3)

The catalog `/Outlines` points to the outline dictionary (`/First`, `/Last`, `/Count`). Each item is a node in a **doubly-linked list of siblings** with child lists:

| Key | Meaning |
|---|---|
| `/Title` | text string |
| `/Parent`, `/Prev`, `/Next`, `/First`, `/Last` | tree links |
| `/Count` | if > 0: open, with this many visible descendants. If < 0: closed |
| `/Dest` or `/A` | target (a destination, or an action such as GoTo/URI) |
| `/C`, `/F` | color and style flags (1.4) |

**Destinations** are arrays like `[pageRef /XYZ left top zoom]`, `/Fit`, `/FitH top`, `/FitV left`, `/FitR l b r t`, `/FitB`, `/FitBH`, `/FitBV`. Named destinations are strings looked up in the `/Dests` name tree, or names looked up in the 1.1-style catalog `/Dests` dictionary. Readers must guard against `/Next` cycles, which are very common in broken outlines, and treat `/Count` as advisory.

### 10.8 Page labels (§12.4.2)

`/PageLabels` is a **number tree** keyed by **0-based page index**. Each value starts a labeling range:

```
/PageLabels << /Nums [
   0 << /S /r >>                 % i, ii, iii, …
   4 << /S /D >>                 % 1, 2, 3, … (restarts at 1)
  20 << /S /D /P (A-) /St 1 >>   % A-1, A-2, …
] >>
```

`/S` is the style: `/D` decimal, `/R` and `/r` Roman upper and lower, `/A` and `/a` letters (A..Z, then AA..ZZ, then AAA.., i.e. the letter repeated). The tree must have an entry for page index 0, and `/St` must be ≥ 1. `/P` is a prefix, and `/St` is the starting number. If `/S` is omitted, the label is the prefix only. Page *labels* are what viewers display and what users type. Page *indices* are what the API uses. Keep them separate.

## 11. How a parser actually reads a PDF

### 11.1 The normal path

```
open file
 │
 ├─1. find header: scan first 1024 bytes for "%PDF-"   → version, header offset H (normally 0)
 │
 ├─2. find tail: read the last ~1–2 KB, search BACKWARDS for "startxref"
 │      parse the integer after it → X
 │
 ├─3. at offset X:
 │      "xref"            → parse classic table + "trailer" dict
 │      "n g obj" /XRef   → parse xref stream (its dict = trailer)
 │
 ├─4. merge: for each object number, the newest entry wins
 │      if trailer has /XRefStm → load that stream (hybrid) before following /Prev
 │      follow /Prev → older section; repeat (detect loops: remember visited offsets)
 │
 ├─5. trailer: /Root, /Encrypt (set up decryption BEFORE resolving any string), /ID, /Info
 │
 └─6. lazy object loading on demand:
        resolve(num, gen):
          entry = xref[num]
          type 1 → seek(offset), parse "num gen obj", parse object;
                   if stream: resolve /Length (may be indirect → recursion guard!),
                              verify "endstream" after it, else scan for it
          type 2 → load object stream (cached), decode, index header, parse k-th object
          free/missing → null
        cache results by (num, gen)
```

Production parsers build a **lazy object graph**. Opening a 2,000-page document should touch only the xref, the trailer, the catalog, and the path down the page tree to the pages actually requested. A good object loader:

- caches parsed objects (and decoded object streams) with a bounded cache, because huge files must not keep every object in memory
- keeps **references as references**. It does not inline them on parse, because cycles are normal (`/Parent` ↔ `/Kids`)
- separates *syntax* errors (recoverable) from *semantic* errors (missing required key) and reports both as diagnostics, not exceptions

### 11.2 When the normal path fails: reconstruction

A large share of real-world PDFs have **some** structural defect. Corpus studies (e.g., the DARPA SafeDocs program, and the issue trackers of pdf.js and PDFium) consistently find that a strict ISO 32000 parser would reject a significant minority of files that every mainstream viewer opens. Users judge a PDF library by whether it opens what Acrobat opens, so **tolerant parsing is not optional**.

The universal fallback is **xref reconstruction**: ignore the xref and scan the whole file.

```
for each match of  /(\d+)\s+(\d+)\s+obj\b/  in the file:
    record xref[num] = (offset, gen)      # later occurrences overwrite earlier ones
    (skip over stream data using /Length or by finding "endstream")
for each "trailer" keyword: parse the dict; keep the one with /Root (prefer the last)
if no trailer: look for an object with /Type /Catalog (or an /XRef stream dict carrying /Root)
also index every /Type /ObjStm found, and add its contained objects (type-2 entries)
```

How the major implementations do it:

| Implementation | Where | Behavior |
|---|---|---|
| **pdf.js** (Mozilla) | `src/core/xref.js`: `XRef.parse()`, `readXRef()`, `indexObjects()` | Tries the normal path. On any `XRefParseException` it reruns in **recovery mode**, which calls `indexObjects()` to scan for `obj`, `trailer`, `xref` and `endstream` tokens, picks a trailer whose `/Root` resolves to a valid catalog, and indexes object streams. Detects the xref "off-by-one" first subsection. Reparses a stream's length by scanning when `/Length` is wrong |
| **MuPDF** (Artifex) | `source/pdf/pdf-repair.c`: `pdf_repair_xref()`, `pdf_repair_obj_stms()` | Repairs automatically when loading the xref fails, or when an object lookup lands on garbage. Scans objects, rebuilds trailer candidates, and finds the catalog by `/Type`. `mutool clean` writes a repaired file |
| **PDFium** (Chrome) | `core/fpdfapi/parser/cpdf_parser.cpp`: `RebuildCrossRef()` | Same strategy. Also falls back when the xref "looks wrong" (an object found at the offset has the wrong number). Very large fuzzing investment (OSS-Fuzz) |
| **poppler** | `XRef::constructXRef()` | Reconstructs silently. In our test it opened a file with a deliberately corrupted xref offset and wrong `startxref` and extracted the text with no error |
| **qpdf** | `QPDF::reconstruct_xref()` | Warns ("file is damaged… attempting to reconstruct cross-reference table") and continues. `qpdf --check` reports problems. `qpdf --qdf` gives a normalized, text-editable form, which is invaluable for debugging writer output |
| **pypdf** | `strict=False` (default) vs `strict=True` | In our test, `strict=True` raised `PdfReadError: Broken xref table` on the corrupted file. Lenient mode recovered it ("incorrect startxref pointer") |

**A catalog of common malformations** that lamina's parser must survive. Each one has been seen in real-world files and in the bug trackers above:

1. Wrong `startxref` offset. Offsets are shifted by N bytes (junk prepended, or EOL conversion).
2. Xref entries of the wrong length (19 or 21 bytes), extra spaces, or a missing subsection header.
3. First xref subsection numbered from 1 instead of 0.
4. Xref entry points at the wrong object (the number at the offset doesn't match), or into the middle of an object.
5. `/Prev` cycles, or `/Prev` pointing outside the file.
6. Truncated file: no `%%EOF`, no xref, or a partial final object.
7. Wrong `/Length` (too short or long), or `/Length` as an unresolvable reference.
8. `stream` followed by a lone CR. `endstream` directly glued to data. `endobj` missing.
9. Objects defined twice with the same number. Updates appended without a new xref.
10. Junk between objects, or garbage after `%%EOF`, or multiple `%%EOF`s.
11. Unbalanced parentheses in literal strings. Invalid hex digits. Names with raw spaces or non-ASCII bytes.
12. Numbers like `--1`, `1.2.3`, `+-5`, `1e5`.
13. Corrupt Flate data (truncated, missing zlib header, bad Adler-32).
14. Page tree: wrong `/Count`, cycles, missing `/Type`, pages referenced twice, `/Kids` containing non-dictionaries.
15. Trailer missing `/Root`. Catalog missing `/Type`. `/Size` smaller than the highest object number.
16. Encryption dictionary present, but parameters inconsistent, or `/ID` missing.

**Design implication:** the parser should be a layered pipeline: tokenizer → object parser → xref/loader → document model. Each layer reports **diagnostics** (warning or error, plus byte offset) instead of throwing, and there are explicit modes:

- `strict`: fail on any violation, for validation and test suites
- `lenient`: the default, Acrobat-like recovery

The diagnostics stream is useful in its own right: it is the basis for a "PDF lint" tool.

## 12. A minimal, verified "Hello, World" PDF

### 12.1 The file

The file below is exactly **599 bytes**. It was generated by a small Python script that computes offsets (§12.3) and verified with **poppler 24.x `pdfinfo`/`pdftotext`** and **pypdf 6.17.0 in strict mode**. Lines end in LF (`\n`), except the cross-reference entries, which end in CR LF (shown as `␍␊`). Line 2 contains the four raw bytes `E2 E3 CF D3`, which display as `âãÏÓ` in Latin-1.

```
%PDF-1.7
%âãÏÓ
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]
   /Resources << /Font << /F1 4 0 R >> >>
   /Contents 5 0 R >>
endobj
4 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
5 0 obj
<< /Length 43 >>
stream
BT
/F1 24 Tf
72 720 Td
(Hello, World) Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f␍␊
0000000015 00000 n␍␊
0000000064 00000 n␍␊
0000000121 00000 n␍␊
0000000253 00000 n␍␊
0000000323 00000 n␍␊
trailer
<< /Size 6 /Root 1 0 R >>
startxref
416
%%EOF
```

First 16 bytes (`od -t x1`):

```
0000000 25 50 44 46 2d 31 2e 37 0a 25 e2 e3 cf d3 0a 31  >%PDF-1.7.%.....1<
```

SHA-256 of the verified file: `f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d`.

Verification output:

```
$ pdfinfo hello.pdf
...
Pages:           1
Encrypted:       no
Page size:       612 x 792 pts (letter)
Page rot:        0
File size:       599 bytes
Optimized:       no
PDF version:     1.7

$ pdftotext hello.pdf -
Hello, World

$ python3 -c "import pypdf; r=pypdf.PdfReader('hello.pdf', strict=True); print(r.pages[0].extract_text())"
Hello, World
```

### 12.2 Annotated, line by line

| Offset | Bytes | Explanation |
|---|---|---|
| 0 | `%PDF-1.7⏎` | Header. 9 bytes. Version 1.7 (ISO 32000-1). A PDF 2.0 version of this file would also need an `/ID` in the trailer (§5.4) |
| 9 | `%âãÏÓ⏎` | Binary marker comment: `%` + 4 bytes ≥ 0x80 + LF = 6 bytes. Optional here (the file is pure ASCII otherwise), but always good practice |
| **15** | `1 0 obj` | Object 1, gen 0. **Offset 15** = 9 + 6. This is the value in xref entry 1 |
| | `<< /Type /Catalog /Pages 2 0 R >>` | The document catalog. `/Pages` points to the page tree root |
| | `endobj` | |
| **64** | `2 0 obj` | Page tree root |
| | `<< /Type /Pages /Kids [3 0 R] /Count 1 >>` | One kid (a page), one leaf page in total. No `/Parent`, because it is the root |
| **121** | `3 0 obj` | The page |
| | `/Parent 2 0 R` | Required back-link |
| | `/MediaBox [0 0 612 792]` | US Letter, 8.5×11 in at 72 units/in. Could be inherited from the Pages node instead |
| | `/Resources << /Font << /F1 4 0 R >> >>` | Maps the content-stream name `/F1` to font object 4 |
| | `/Contents 5 0 R` | The page's content stream |
| **253** | `4 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>` | One of the **standard 14 fonts**, so no font program or widths are needed and the viewer substitutes its own Helvetica. Fine for a minimal demo. PDF 2.0 deprecates relying on non-embedded standard fonts, and PDF/A forbids it |
| **323** | `5 0 obj` | Content stream |
| | `<< /Length 43 >>` | 43 = 3 (`BT⏎`) + 10 (`/F1 24 Tf⏎`) + 10 (`72 720 Td⏎`) + 18 (`(Hello, World) Tj⏎`) + 2 (`ET`). The LF after `ET` is the EOL before `endstream` and is **not** counted |
| | `stream⏎` | Keyword + LF. Data starts on the next byte |
| | `BT` | Begin text object |
| | `/F1 24 Tf` | Select font resource F1 at 24 pt |
| | `72 720 Td` | Move the text position to (72, 720): one inch from the left, 72 pt below the top edge |
| | `(Hello, World) Tj` | Show the string (single-byte codes through Helvetica's StandardEncoding) |
| | `ET` | End text object |
| | `⏎endstream⏎endobj` | |
| **416** | `xref` | Start of the cross-reference table. **Offset 416**, which is the value after `startxref` |
| | `0 6` | One subsection: object numbers 0 through 5 |
| | `0000000000 65535 f␍␊` | Object 0: free-list head, generation 65535, next free = 0 (end of list) |
| | `0000000015 00000 n␍␊` … | Objects 1–5 with their byte offsets. Each line is exactly 20 bytes |
| | `trailer` `<< /Size 6 /Root 1 0 R >>` | `/Size` = highest object number (5) + 1. `/Root` → catalog |
| | `startxref` `416` | Where to find the xref |
| | `%%EOF` | End marker (followed by a final LF; total 599 bytes) |

### 12.3 Generator script (as used for verification)

```python
import sys
out = sys.argv[1]
content = b"BT\n/F1 24 Tf\n72 720 Td\n(Hello, World) Tj\nET"
objs = [
 b"<< /Type /Catalog /Pages 2 0 R >>",
 b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
 b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\n"
 b"   /Resources << /Font << /F1 4 0 R >> >>\n   /Contents 5 0 R >>",
 b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
 b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
]
buf = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offs = []
for i, o in enumerate(objs, 1):
    offs.append(len(buf))                      # byte offset of "i 0 obj"
    buf += b"%d 0 obj\n" % i + o + b"\nendobj\n"
xref = len(buf)
buf += b"xref\n0 %d\n" % (len(objs) + 1)
buf += b"0000000000 65535 f\r\n"
for off in offs:
    buf += b"%010d 00000 n\r\n" % off          # exactly 20 bytes
buf += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
open(out, "wb").write(buf)
```

This is the core algorithm of every PDF writer: **serialize objects while recording the offset of each, then emit the index.** Everything else is object-graph management.

### 12.4 The same document with object and cross-reference streams (PDF 1.5)

To exercise §6, we also generated a 521-byte variant. Objects 1–4 sit in an object stream (object 6, Flate-compressed), the content stream is object 5, and object 7 is a Flate-compressed cross-reference stream with `/W [1 2 1]`. There is no `xref` keyword and no `trailer` keyword. Its uncompressed parts:

```
%PDF-1.5
%âãÏÓ
5 0 obj
<</Length 43>>
stream
BT /F1 24 Tf 72 720 Td (Hello, World) Tj ET
endstream
endobj
6 0 obj
<</Type/ObjStm/N 4/First 20/Filter/FlateDecode/Length 163>>
stream
…163 bytes zlib…   (decodes to "1 0 2 30 3 66 4 163 <</Type/Catalog/Pages 2 0 R>> …", see §6.1)
endstream
endobj
7 0 obj
<</Type/XRef/Size 8/W[1 2 1]/Root 1 0 R/Filter/FlateDecode/Length 35>>
stream
…35 bytes zlib…    (decodes to the 8 rows listed in §6.2)
endstream
endobj
startxref
362
%%EOF
```

`pdfinfo` reports "PDF version: 1.5, Pages: 1", and `pdftotext` and pypdf (strict) both extract "Hello, World".

## 13. Security-relevant parsing pitfalls

This section is brief. Full treatment, with CVEs and mitigations, is in **[chapter 04](04-interactive-security.md)** (§11, PDF as an attack surface).

- **Polyglots and parser differentials.** Readers accept a header anywhere in the first 1024 bytes, rebuild xrefs by scanning, and ignore trailing garbage. As a result, one file can be a valid PDF *and* a valid ZIP, JPEG, HTML or PE file (Ange Albertini's corkami/PoC||GTFO work). One PDF can also render **different content in different viewers**, by exploiting duplicate keys, duplicate object numbers, xref versus scan disagreements, or hybrid-file lookup order. Content-sniffing gateways and malware scanners are often fooled. Defence: in lamina, be deterministic and document which choice we make in each ambiguous case. Offer a strict mode that reports every ambiguity.
- **Incremental-update attacks on signatures.** Researchers at Ruhr-University Bochum published a series of attacks:
  - **Incremental Saving Attack, Signature Wrapping and Universal Signature Forgery** ("1 Trillion Dollar Refund", ACM CCS 2019)
  - **Shadow Attacks** (NDSS 2021): hide, replace, or hide-and-replace content using *well-formed* incremental updates. 16 of 29 tested viewers were vulnerable, including Adobe Acrobat and Foxit
  - **Certification attacks** (Evil Annotation and Sneaky Signature, IEEE S&P 2021)

  The core lesson for a validator: after verifying the signature over `/ByteRange`, you **must** diff the object graph between the signed revision and the final revision, and judge every change against the DocMDP/FieldMDP permissions. "The signed bytes verify" is not enough.
- **Decompression bombs.** Deflate reaches about 1032:1, so 1 MB inflates to about 1 GB. Filter chains multiply this (`[/FlateDecode /FlateDecode]`). Other vectors: JBIG2 and CCITT with huge `/Width`/`/Height`, object streams with huge `/N`, xref streams with huge `/Size` or `/Index` ranges, and `/W` widths up to 8+ bytes producing absurd offsets. Mitigation: per-stream and per-document decoded-size budgets, checked *while* decoding. Lazy decoding. Dimension caps before allocating image buffers.
- **Recursion and cycles.** All of these occur in malicious and in merely broken files:
  - `/Prev` loops
  - page tree `/Kids` cycles
  - outline `/Next` cycles
  - name-tree `/Kids` cycles
  - a stream's indirect `/Length` that points into an object stream which needs that same stream to decode
  - an object stream containing a reference to itself
  - form XObjects that `Do` themselves
  - deeply nested arrays or dictionaries (`[[[[…` 100k deep) that overflow a recursive-descent parser's stack

  Mitigation: visited sets, explicit depth limits, and iterative parsing where possible.
- **Complex decoders are attack surface.** JBIG2 is the canonical example: the FORCEDENTRY exploit (CVE-2021-30860, NSO Group) built a Turing-complete machine out of JBIG2 refinement operations in Apple's CoreGraphics. It was delivered as a PDF disguised as a GIF. Treat image codecs (JBIG2, JPX, CCITT) as untrusted code paths: memory-safe implementations, fuzzing, and sandboxing.
- **Active content and external references** (`/OpenAction`, JavaScript, `/Launch`, `/URI`, `/GoToR`, `/F` external stream files, `/SubmitForm`) are not file-structure issues, but a parser must expose them for auditing and must never act on them implicitly. Researchers have shown data exfiltration through forms and URIs ("Processing Dangerous Paths", NDSS 2021).

## 14. Implementation guidance for lamina (summary)

**Writer defaults:**

- Header `%PDF-1.7`, or `%PDF-2.0` when 2.0 features are used, plus the binary marker.
- LF line endings. Xref entries `nnnnnnnnnn ggggg n\r\n`.
- Always write `/ID`, generated from a content hash so output is deterministic, with an option for a random ID.
- Default to **object streams + cross-reference streams** (smaller files, universally supported since about 2005). Offer a "classic" mode for maximal compatibility and debugging, and a "QDF-like" readable mode (uncompressed, one object per line, comments with object labels).
- Deterministic output: stable object numbering, sorted dictionary keys only where semantics allow it, no timestamps unless requested. This makes golden-file tests possible.
- Streams: FlateDecode with a configurable level. PNG Up predictor for xref streams. Never LZW. Never `BrotliDecode` by default.
- `/Length` written directly when known, indirectly when streaming.
- A balanced page tree (e.g., ≤ 32 kids per node) for large documents.
- Write both `/Info` and XMP when a compatibility profile requires it. XMP only for PDF 2.0 or PDF/A-4.
- Incremental "append mode" is a first-class save mode, required for signatures and form filling on signed documents.

**Reader architecture:**

- A byte-level tokenizer that never decodes text. Lookahead for `n g R` / `n g obj`.
- Xref loader for classic tables, streams, hybrid files and `/Prev` chains, plus a reconstruction fallback. Every repair is logged as a diagnostic with a byte offset.
- Lazy, cached object resolution. Decryption in the loader. Object stream cache.
- Limits for depth, decoded bytes, object count, and image dimensions, all configurable.
- `strict` and `lenient` modes. A conformance test corpus that includes deliberately broken files. The files from §12, plus the corrupted-xref variant we produced, are a starting point.
- Use the PDF Association's **Arlington PDF Model** (a machine-readable TSV/JSON definition of every PDF object, key, type, version and requirement, for PDF 1.0–2.0) to generate typed accessors and validation rules, instead of hand-transcribing tables from the spec.

---

## 15. References

**Specifications**

- ISO 32000-2:2020, *Document management — Portable document format — Part 2: PDF 2.0* — https://www.iso.org/standard/75839.html
- PDF Association, sponsored no-cost access to ISO 32000-2 and the ISO/TS 32001/32002/32003 extensions — https://pdfa.org/sponsored-standards/
- PDF Association, "Announcing no-cost access to ISO 32000-2 (PDF 2.0)" (April 2023) — https://pdfa.org/announcing-no-cost-access-to-iso-32000-2-pdf-2-0/
- PDF 2.0 public issue tracker and corrections — https://pdf-issues.pdfa.org/32000-2-2020/
- ISO 32000-1:2008 (PDF 1.7), copy hosted by Adobe — https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf
- Adobe PDF Reference archive (1.0–1.7) — https://opensource.adobe.com/dc-acrobat-sdk-docs/
- PDF Association, *Brotli compression extension for PDF* — https://pdfa.org/download-area/publications/pdf-extension-brotli.pdf
- ISO 14289-2:2024 (PDF/UA-2) — https://www.iso.org/standard/82278.html
- PDF Association, "ISO 14289-2 (PDF/UA-2) … has arrived" — https://pdfa.org/iso-14289-2-pdf-ua-2-the-gold-standard-for-accessibility-in-pdf-2-0-has-arrived/
- RFC 1950 (zlib) — https://www.rfc-editor.org/rfc/rfc1950 ; RFC 1951 (DEFLATE) — https://www.rfc-editor.org/rfc/rfc1951
- PNG filter types (W3C PNG spec, §9) — https://www.w3.org/TR/png-3/#9Filters

**Tools and implementations**

- PDF Association, Arlington PDF Model — https://github.com/pdf-association/arlington-pdf-model
- veraPDF (PDF/A and PDF/UA validator) — https://verapdf.org/
- pdf.js xref parsing and recovery — https://github.com/mozilla/pdf.js/blob/master/src/core/xref.js
- MuPDF repair code — https://github.com/ArtifexSoftware/mupdf/blob/master/source/pdf/pdf-repair.c
- PDFium parser — https://pdfium.googlesource.com/pdfium/+/refs/heads/main/core/fpdfapi/parser/cpdf_parser.cpp
- qpdf documentation (QDF mode, `--check`, linearization) — https://qpdf.readthedocs.io/
- pypdf (strict vs lenient parsing) — https://pypdf.readthedocs.io/
- HexaPDF BrotliDecode filter — https://hexapdf.gettalong.org/documentation/api/HexaPDF/Filter/BrotliDecode.html

**Security research**

- Mainka, Mladenov, Rohlmann, "Shadow Attacks: Hiding and Replacing Content in Signed PDFs", NDSS 2021 — https://www.ndss-symposium.org/wp-content/uploads/ndss2021_1B-4_24117_paper.pdf
- PDF Insecurity project (signature spoofing, certification attacks, encryption attacks) — https://pdf-insecurity.org/
- Google Project Zero, "A deep dive into an NSO zero-click iMessage exploit" (FORCEDENTRY / JBIG2) — https://googleprojectzero.blogspot.com/2021/12/a-deep-dive-into-nso-zero-click.html
- Ange Albertini, PDF format notes and polyglots (corkami) — https://github.com/corkami/docs/blob/master/PDF/PDF.md

---

## Verification log

**Date:** 2026-10-28

**Sources used:** the text of ISO 32000-1:2008 (PDF 1.7, Adobe-hosted copy), checked clause by clause for §7.2–7.9, §12.3.3, §12.4.2, §14.2, §14.4, §14.11.2 and Annexes C, E, F and I; the Library of Congress format description of PDF 2.0 (fdd000474) for the PDF 2.0 deprecations; the PDF Association's ISO 32000-2 errata pages (clause 12); IDRsolutions, pypdf and Mapsoft explainer pages as secondary context. The ISO 32000-2 text itself was **not** available. The example files in `examples/01-file-structure/` were checked by size, SHA-256 and byte offsets only. No scripts were run.

**Corrections made**

- §1, PDF 2.0 deprecations: "PostScript XObjects" were listed as deprecated → now "removed outright in 2.0 (discouraged in 1.7)", per LoC.
- §3.1/§3.2 clause numbers: §7.2.3 / §7.2.4 → added the 32000-1 numbers §7.2.2 (Character Set) / §7.2.3 (Comments).
- §3.3 binary marker: "should be followed" → "**shall**" (32000-1 §7.5.2).
- §3.3 1024-byte header tolerance: now says plainly that this is an old Adobe implementation note, not in ISO 32000-1 §7.5.2.
- §4.2 Annex C limits: "Acrobat's 32-bit limits" → "minimum limits for a typical 32-bit conforming reader" (Table C.1). The string limit was **inverted**: "32,767 bytes, not respected for strings in content streams" → "applies **only** to strings in content streams; no effective limit elsewhere". "5 significant digits" → "in the fractional part". Added CID ≤ 65,535 and the 10¹⁰-byte file-size limit for classic xref tables (C.2). Marked the page-size range as a C.2 recommendation.
- §4.3 `\ddd`: the `\0053` example was described as "high-bit overflow". Now it's described correctly as the three-digit maximum (§7.3.4.2).
- §4.3 dates: "PDF 1.7 had a trailing `'`; 2.0 dropped it" → the trailing `'` was in the Adobe PDF Reference. **ISO 32000-1 §7.9.4 already omits it.** Also added which fields are required and their defaults.
- §4.3 encryption exceptions: added content streams to "strings inside streams", and cited §7.6.1.
- §4.4 names: "`#` should be escaped" → `#`, delimiters and white-space **must** be escaped. Only regular characters outside 0x21–0x7E "should" be (§7.3.5).
- §4.5 duplicate keys: "behavior undefined" → "shall not occur; the spec doesn't say what readers should do" (§7.3.7).
- §4.5 unknown keys: citation "§7.3.7" was wrong → 32000-1 Annex I.3. Added the 32000-1 Annex E name (PDF Name Registry).
- §4.6 `/F` external streams: "`/Length` is then ignored and should be 0" → the bytes between `stream`/`endstream` are ignored, and `/Length` should still count them, usually 0 (Table 5). Marked PDF 1.2.
- §4.7 null references: the spec only states the rule for undefined objects (§7.3.9–7.3.10). Applying it to a generation mismatch or a free entry is now labelled as reader practice.
- §4.7 must-be-direct examples: "the `/Encrypt` dictionary's entries when used with xref streams" (not found in the spec) → stream dictionaries, xref-stream Table 17 entries, linearization dictionary values, and `/ID` when encrypted.
- §5.2: each update's trailer has its own `%%EOF` (§7.5.6). The "last 1024 bytes" claim is now labelled as an Adobe implementation note.
- §5.4 Table 15: `/Prev` is "present only when there is more than one section" (this includes linearized files). Noted the 32000-1 "shall be an indirect reference" typo. `/Info` must be indirect. With `/Encrypt`, `/ID` must be direct and unencrypted. The `/ID` derivation now follows §14.4 (time, path, size, `/Info` values; need not be reproducible).
- §5.4 update trailers: "repeats all keys except `/Prev` and any key whose value changes" → repeats **all** keys except `/Prev`, changed or not (§7.5.6).
- §6.1 object streams: added "must not consist solely of a reference" and the object-number reuse rule (§7.5.7).
- §6.2 xref streams: "values in its dictionary must be direct" → only the Table 17 entries plus `/Filter` and `/DecodeParms`. `/Root` must be indirect. Added "no Crypt filter". "Should contain an entry for itself" → it needs an entry, usually in itself, or in the classic table of a hybrid file (§7.5.8.2–7.5.8.3).
- §6.3 hybrid files: added the §7.5.8.4 wording that `/XRefStm` belongs in an update section's trailer, and that hidden objects appear as free entries with generation 65535.
- §8.1 `/T`: the comment now gives the 32000-1 Table F.1 definition (the white-space before the first entry; or the offset of the main xref stream).
- §8.1 hint tables: the optional-table list now follows Table F.2 + F.4.6. Added article threads and page labels, and dropped "renditions". The "deviations that Annex F acknowledges" are now attributed to the old Adobe Reference; ISO Annex F does not carry them.
- §8.2: added that linearized files must not use inherited page attributes (§7.7.3.4).
- §9 filter table: `RunLengthDecode` "since 1.2" → 1.0 (Table 6 gives it no version tag).
- §9.2: noted that filter abbreviations are spec-legal only in inline images (§8.9.7).
- §10.3 page objects: `/Contents` was listed with the required keys → it is optional, and an absent value means an empty page. Writers must not write an empty `/Contents` array (Table 30).
- §10.5 `/ProcSet`: "may omit on write" → 32000-1 §14.2 still says writers *should* emit it. Inherited form resources: "PDF 1.1 and earlier" → "earlier versions; obsolete per §7.8.3".
- §10.5 name trees: added the shorter-key-first ordering and byte-wise equality (§7.9.6).
- §10.8 page labels: clarified the letter-style sequence (AA = letter doubled). Added that index 0 is required and `/St` must be ≥ 1 (§12.4.2).

**Confirmed against 32000-1 (no change needed):** white-space and delimiter tables; EOL rules after `stream`; literal/hex string rules; null-valued entries; number syntax and no exponents; 20-byte xref entries with the SP CR / SP LF / CR LF EOLs; free-list rules and generation 65535; `/Size` semantics; object-stream contents and restrictions; xref stream `/W`, `/Index`, entry types and the "other type → null" rule; hybrid lookup order; linearization dictionary keys and the first-1024-bytes rule; filter parameters (LZW `EarlyChange` default 1, CCITT defaults, PNG predictor rule, Crypt filter first); page-tree keys and inheritable attributes (Resources, MediaBox, CropBox, Rotate); page boundary defaults and clipping; name-dictionary trees; outline `/C`/`/F` (PDF 1.4); `/UserUnit` (PDF 1.6); and the clause numbers §7.3.x, §7.4, §7.5.x, §7.7.x, §7.8.3, §7.9.x, §12.3.3, §12.4.2 and §14.11.2, which match.

**Still unverified**

- All PDF 2.0-only statements:
  - required `/ID`
  - UTF-8 text strings (corroborated by LoC)
  - the `/Info` deprecation (corroborated by LoC)
  - the `/ISO_` extension prefix and arrays
  - `/AF`, `/DPartRoot`, `/DSS`, page-level output intents
  - the "limits became advice" change in Annex C
  - the 32000-2 numbering of §7.2.x and the 32000-2 Annex E
- History table (§1): which version introduced each feature, Acrobat versions, dates. Only the items that 32000-1 tags with a version were checked.
- ISO family table (§2), governance and free-access history, BrotliDecode status, and all tool, implementation and security-research claims (§11, §13). These are outside the spec.
- Acrobat behaviors: header within 1024 bytes, `%%EOF` within the last 1024 bytes, and accepting abbreviated filter names in stream dictionaries.
