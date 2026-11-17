# The inspection differential failed on its first nightly

[Resolution](resolution.md).

## What happened

The nightly run 44173843495, on `1c228ebf0` (17 November 2026), failed in its new "Inspection differential" step, which #714 (the read-only inspectors) added. The step compares the objects and fonts the inspector reads with qpdf's and pdffonts' over the external corpus and the committed PDFs.

- **Objects.** 1,860 of 1,898 files matched qpdf; every file's newest cross-reference section matched.
- **Fonts.** 1,899 of 2,130 files matched pdffonts.
- **Excuses.** None of the 269 that differed was excused, because `fixtures/differential/inspect-exceptions.toml` didn't exist yet, and its `[[known]]` entries could only excuse a whole fact in a file.

## How to reproduce it

```sh
cargo xtask corpus fetch
cargo xtask differential --live --inspect   # qpdf 12.4.2 and pdffonts 26.10.0, as doctor.rs pins
```

The report is in `target/nightly/differential-inspect/`: `report.json` names each file and fact, `lamina.json` and `tools.json` hold each side's facts. Pairing the fonts by object and grouping the fields that differ sorts them by cause, below.

## Cause

Three causes overlap. The inspector misread four things in the font dictionaries, the differential's pdffonts reader misread long rows, and the rest are differences in what the tools report.

### The inspector's bugs

`laminapdf-model`'s font walk (`src/fonts.rs`) read the font dictionary more narrowly than ISO 32000-2 and Lamina PDF's own text extraction do.

- **An indirect `/BaseFont`.** It read the name with `Dict::get_name`, which doesn't follow a reference, though §7.3.10 lets any value be indirect. Fonts such as `pdfjs/test/pdfs/issue2462.pdf`'s object 31 (`/BaseFont 28 0 R`) had no name and no subset tag.
- **A Type 3 font's name.** A Type 3 font has no `/BaseFont` (§9.6.4, Table 110); its name is in its font descriptor's `/FontName` (§9.8.1) or, in older files, its `/Name`. All 54 Type 3 fonts that pdffonts named had none in the inspector, and 14 of them lost their subset tag too (`issue19634.pdf`: `AAAAAA+Toy Liberation Sans`).
- **A `/ToUnicode` name.** Any `/ToUnicode` entry counted as a CMap, but only a stream is one (§9.10.3). Producers write `/ToUnicode /Identity-H`, which maps nothing, and the text extraction ignores it (`laminapdf-text`'s `ExtractFont::load`). That accounted for 16 fonts, such as `sample_fonts_solidconvertor.pdf`'s.
- **The encoding dictionary.** Any dictionary read as `Custom`, but a simple font's encoding dictionary without `/Differences` is its `/BaseEncoding` (§9.6.5.1), and a stream is no encoding for a simple font (only a Type 0 font's CMap is a stream, §9.7.5). The text extraction already reads them so. `issue5599.pdf`'s `<< /BaseEncoding /WinAnsiEncoding >>` showed as `Custom`.

### The differential's pdffonts reader

`parse_pdffonts` cut each row at the columns of the header's dashes. Yet pdffonts pads its columns without cutting values (`printf("%-36s %-17s …")` in poppler's `pdffonts.cc`), so a name over 36 characters, or a name whose bytes were decoded lossily, moved every column after it. `bug900822.pdf`'s `ABCDEF+LucidaSansUnicode,Bold-WinCharSetFFFF` read with its name cut short, `yes yes 32 0` as its object and no type. That misreading caused every Type 0 font's `embedded` difference, three of its `subset` differences and the names cut short, and about half the fonts only one side listed (`SimFang-variant.pdf`, `XiaoBiaoSong.pdf`, `issue3521.pdf`).

### What the tools report differently

pdffonts prints what poppler draws with, not what the file stores (poppler's `GfxFont.cc`, `FontInfo.cc`, `Annot.cc` at 26.10.0); qpdf repairs as it reads.

- **The encoding column** is poppler's base encoding after its fallbacks: WinAnsi for a TrueType font without `/Encoding`, Standard for one whose name maps to a standard 14 font. Of the 294 TrueType fonts the inspector reported as `Builtin`, 293 have no `/Encoding` at all (`GlyphLayoutDIN91379Form.pdf`'s object 16). The other, `issue14821.pdf`'s object 41, has a stream there, which a simple font can't take (§9.6.5.1), so it declares nothing. The inspector rightly reports no encoding for any of them: such a font maps codes through its own `cmap` (§9.6.5.4). An unknown name (`/Winng`, `/NULL`) shows in the inspector as stored, while poppler ignores it.
- **The type column** comes from the font program poppler finds (a CFF program under `/FontFile2` is `Type 1C (OT)`), the inspector's from `/Subtype`.
- **The font set.** pdffonts scans a page's resources, forms and patterns, and only the appearance an annotation's `/AS` selects (`Annot::getAppearanceResDict`). The inspector also lists every other appearance state's fonts (a check box's ZapfDingbats), soft masks' and Type 3 fonts' resources. And poppler draws its own appearance, with a direct Helvetica, for a free text annotation without `/AP`.
- **`uni` for a direct font** is always `no`: pdffonts looks `/ToUnicode` up through the font's object, which a direct font hasn't.
- **Objects.** qpdf's JSON lists the cross-reference stream it read even when no section lists it, lists a referenced number that no section defines as `null` (§7.3.10), and sets `/Type /Catalog` on a root that lacks it (`MissingCatalog.pdf`).

Six files differ because Lamina PDF's reader gives up on a damaged object that qpdf or poppler recovers: a `/Length` past `endstream` (`bug_972518.pdf`, `bad24.pdf`), a bare `"` in a dictionary (`embedded_attachments_with_desc.pdf`), and dictionaries without `>>` or `<<` (`bug_1893.pdf`, `bug_481363.pdf`, `PDFBOX-3148-2-fuzzed.pdf`). Those are the reader's, not the inspector's.
