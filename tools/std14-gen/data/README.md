# std14-gen data

Inputs of [`tools/std14-gen`](../src/main.rs), which writes `crates/laminapdf-text/src/std14/generated/*.rs` (todo [B4c](../../../docs/todos/done/B4c-standard-14-and-simple-fonts.md)). The generator checks every file against the SHA-256 below before reading it; its `committed_tables_are_current` test fails when the committed tables differ from a regeneration.

```sh
cargo run -p std14-gen             # rewrite the generated tables
cargo run -p std14-gen -- --check  # verify them
```

## `afm/`: Adobe Core 14 AFM files

The 14 AFM files and `MustRead.html` of Adobe's `Core14_AFMs.zip` (AFM format 4.1, 1997; files dated 1999-11-10), unmodified. Adobe's original URL, `http://partners.adobe.com/public/developer/en/pdf/Core14_AFMs.zip`, is gone; the archive was fetched from the Internet Archive's copy, <https://web.archive.org/web/2010id_/http://partners.adobe.com/public/developer/en/pdf/Core14_AFMs.zip> (SHA-256 `8c892c3c49553cfd2d2a27c4495b4bb12e2875115be7fd127ed3876df19d8654`). The `.tar` of the same page holds byte-identical AFM files. The files keep their CRLF line endings (`.gitattributes`: `-text`), so the hashes match the archive.

**Licence** (`MustRead.html`, verbatim): "This file and the 14 PostScript(R) AFM files it accompanies may be used, copied, and distributed for any purpose and without charge, with or without modification, provided that all copyright notices are retained; that the AFM files are not distributed without this file; that all modifications to this file or any of the AFM files are prominently noted in the modified file(s); and that this paragraph is not modified. Adobe Systems has no responsibility or obligation to support the use of the AFM files."

How lamina meets the conditions: the AFM files are committed unmodified next to `MustRead.html`; the generated `metrics.rs` (a modified, derived form) repeats every copyright notice of the files and the paragraph above unchanged, and states what was changed (only header metrics, widths and encodings kept, converted to Rust tables). The fonts' trademark notices (Helvetica and Times: Linotype-Hell AG; ITC Zapf Dingbats: International Typeface Corporation) stay in the AFM files; lamina uses the names only as the PDF standard's font names (ISO 32000-2 §9.6.2.2).

| File | SHA-256 |
| --- | --- |
| `Courier.afm` | `521e0d7c7521efd4be78a5a9c5398e4c67d0771e396115b0346bc4ef74ada53d` |
| `Courier-Bold.afm` | `ad0150d4bedcc8877742bf94251fcec13e348dd599d4603f679d92027d1e6e99` |
| `Courier-Oblique.afm` | `b27103b2a2ef6030c110626597e2ab47bb8279075a039ab9170facb0aa1f70e1` |
| `Courier-BoldOblique.afm` | `cb82e69ef5f6d421e8f404fe00bb0d993425aae79725331d0ebf847e94e97e92` |
| `Helvetica.afm` | `da33f1870474c8e68bfe3e2353ff107ab6c6eea1f9836ce2aaf1e1a07b17982f` |
| `Helvetica-Bold.afm` | `b880d96baf56d0cc059f258f60b4d764ef49b555ab9db294b959c0016dee41f2` |
| `Helvetica-Oblique.afm` | `b4609b71b660a392ac09df35060271a876c2ce66617dd83bf826f742bb9d9721` |
| `Helvetica-BoldOblique.afm` | `69984a35ca26973a39f261cf83e0d367ea2e6517c590b0b030d4d4a219d9c269` |
| `Times-Roman.afm` | `768e1cabea085d489a63da3e80b96bc5abf0ec98d3073c9b4d6ba76e7bccba64` |
| `Times-Bold.afm` | `b4a000ed85cb22c6cdd985aa0fd3f6f78ed5079b7c6860dc4f0234e0d0e3c522` |
| `Times-Italic.afm` | `ed37fa2e6a67b5b17dfd47f36fc7e90df32891a4408860dbc8d4cbbe9959e242` |
| `Times-BoldItalic.afm` | `93c4744ba955215de02c4aae0b777133442ada2f7ab5a2af30a040b792b3c55d` |
| `Symbol.afm` | `3d2128a820375a10de9bc8bf6cfb15ded482c01ca0f95cc0b3277f37ec8bde66` |
| `ZapfDingbats.afm` | `a32565c90afd1b57a7008fc567b78d95cf1c22adff5e086094d666d88b039859` |
| `MustRead.html` | `be71e055bb2551a6fc8b60dcf094bf40357858980abde86f76cc9a3c144d5403` |

## `agl/`: Adobe Glyph List

`glyphlist.txt` (Adobe Glyph List 2.0) and `zapfdingbats.txt` (ITC Zapf Dingbats Glyph List 2.0) from <https://github.com/adobe-type-tools/agl-aglfn> at commit `4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6`, unmodified. **Licence:** BSD-3-Clause, © 2002-2019 Adobe, stated in each file's header; the generated `agl.rs` and `chars.rs` repeat it.

| File | SHA-256 |
| --- | --- |
| `glyphlist.txt` | `a3b2f61ced9f3644cc0d4ecde5c59df34ca286c689d9484a43a710a81c466789` |
| `zapfdingbats.txt` | `f6394e3cb8a447e84a1dad75d4baaf2aa7f45dc104faf369f4720e1a774ef2dc` |

## `latin-encodings.txt`

The standard Latin character set with its `StandardEncoding`, `MacRomanEncoding` and `WinAnsiEncoding` codes, as ISO 32000-2:2020 Annex D, Table D.2 lists them, written for lamina (`MIT OR Apache-2.0`, like the rest of lamina; an encoding table is a fact of the standard). It was drafted from the Windows-1252 and Mac OS Roman code pages (Python's `cp1252` and `mac_roman` codecs) named through the Adobe Glyph List, with Annex D's differences applied by hand: `space` also at 240 (WinAnsi) and 312 (MacRoman), `hyphen` also at 255 (WinAnsi), `currency` at 333 (MacRoman, where Mac OS Roman now has the Euro), and Mac OS Roman's glyphs outside the Latin set (`apple`, `Omega`, `pi`, `notequal`, `infinity`, …) left out. The generator checks the STD column against the AFM files' codes and the glyph set against theirs; `crates/laminapdf-text/tests/std14.rs` checks the code counts against Table D.2 and WinAnsi's upper half against Windows-1252. SHA-256 `c91fb386c1d8d31a56d94db73e9345d43943c78763c2651b53e1b55a26dc31dc` (the generator checks it like the other inputs; update both when the file changes); a code given to two glyphs in one column fails the generator. The PDFDocEncoding column is not repeated: `laminapdf_syntax::text` holds the workspace's one copy.
