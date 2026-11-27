# @laminapdf/fonts

This package serves the typefaces of Lamina PDF's interface from the app's own origin, so no page ever fetches a font from another host. It holds the font files, one style sheet that declares them, and the list of files worth preloading. The web reader, the desktop app, the docs site and the website all load their fonts from here.

For now it is a workspace package, not published to npm.

## Use it

Load the style sheet once, before your own styles. A bundler such as Vite or Astro copies the font files it names into the build with hashed names:

```css
@import "@laminapdf/fonts/fonts.css";
```

Then set `font-family` from the design tokens (`var(--paper-font-sans)`, `var(--paper-font-serif)`, `var(--paper-font-signature)`). A face downloads only when text on the page uses it.

To show the first paint in the right font, preload the one file that paint needs. In code that runs in the browser:

```ts
import { FONT_PRELOADS } from "@laminapdf/fonts";

for (const { href, type } of FONT_PRELOADS) {
  const link = Object.assign(document.createElement("link"), { rel: "preload", as: "font", type, href });
  link.crossOrigin = "anonymous";
  document.head.append(link);
}
```

A page rendered on a server, such as an Astro page, has no browser build to resolve `href`. It imports each `file` through its bundler instead, for example `import href from "@laminapdf/fonts/files/lamina-sans-latin-wght-normal.woff2?url"`.

## The faces

| Family | Style | Weights | Latin | Latin Extended | Used for |
| --- | --- | --- | --- | --- | --- |
| Lamina Sans | upright | 400–700, variable | 30.6 KB (preloaded) | 62.4 KB | every control, label and message |
| Lamina Sans | italic | 400–700, variable | 30.6 KB | 62.6 KB | emphasis in running text |
| Newsreader | upright | 400–600, variable | 36.9 KB | 21.2 KB | titles, headings, outline rows |
| Newsreader | italic | 400 | 21.9 KB | 12.8 KB | quotes |
| Caveat | upright | 500 | 49.0 KB | none | the typed-signature style |

Each face comes as two files. The Latin file covers English and most Western European text, typographic punctuation and the shortcut symbols (⌘ ⌥ ⇧ ⌫ and the arrows). The Latin Extended file covers the rest of the Latin alphabet, and the browser downloads it only when a page shows one of those characters. The Latin files together come to 169 KB, under the 200 KB budget, and the preloaded file stays under 30 KB; `test/budget.test.ts` keeps both.

Greek, Cyrillic, CJK, Arabic, Hebrew, Indic and every other script fall back to the system's fonts, character by character: the token stacks end in `system-ui` and `sans-serif` (or `Georgia, serif`). The return symbol ↵ isn't in the fonts either, and draws in the system font too.

**Lamina Sans is Source Sans 3**, Adobe's typeface, under another name. Its licence reserves the name "Source" for the font as Adobe ships it. These files are subsets, which the licence counts as modified copies, so they take another name. The design is unchanged; Adobe's copyright notice stays in every file.

**The fallback faces** are Arial in its regular, bold, italic and bold italic styles (or Liberation Sans or Arimo, which share its widths). The script scales each so that its text takes the space Lamina Sans' text will take at the same weight and style: 94.23% for regular text. The stack names them right after Lamina Sans, so the swap when the font arrives moves no line. Text at weight 500 uses the regular faces, and text at 600 the bold ones.

**Newsreader's optical size** is fixed at its default, 18. The variable optical-size axis would more than double its Latin file (90 KB instead of 37 KB), and the interface's titles, from 15 to 40 px, read well at one size.

Source Serif 4, which the designs use for page mockups, isn't shipped: nothing in the interface needs it yet.

**For PDFs**, `pdf/lamina-sans-regular.ttf` is Lamina Sans Regular as TrueType, Latin and Latin Extended in one file (100 KB). The reader embeds it in the pages it builds, such as a contents page: PDF readers can't read WOFF2, and the PDF keeps only the glyphs it uses. The reader downloads it only when it builds a page. A title in another script can't be set in it, and the reader says so.

**For the font picker**, three more TrueType files in `pdf/` are whole: every character the font maps, not a subset by script (the owner's choice in [issue #532](https://github.com/laudebugs/laminapdf/issues/532)). Variable fonts are limited to their Regular instance, and hinting is dropped.

| File | Size | Used for |
| --- | --- | --- |
| `newsreader-regular.ttf` | 110 KB | a serif for added text |
| `jetbrains-mono-regular.ttf` | 100 KB | a monospace for added text |
| `noto-sans-symbols-2-regular.ttf` | 661 KB | dingbats, shapes, some arrows and currency signs |

None of the three reserves a font name: each licence names no Reserved Font Name, so the files keep their names and need no renaming, as Lamina Sans does.

JetBrains Mono is the monospace because it is OFL with no reserved name, covers Latin, Greek and Cyrillic in one file, and has the mathematical operators Noto Sans Symbols 2 lacks.

So `∑ √ ≤` and the like come from JetBrains Mono, the last resort among the fonts that fill in for another. Source Code Pro, the example in the issue, reserves "Source" like Source Sans 3 and would need renaming too.

The reader fetches each file from the app's own origin the first time a text needs it, never at startup. Its service worker keeps the file once fetched, and leaves the TrueType files out of its install, so a reader who never adds text downloads none. `PDF_FONTS`, in `@laminapdf/fonts/pdf` (a module of its own, so the reader's entry carries none of it), lists each file with its `bytes`, so the picker can name Noto Sans Symbols 2 and its size before it fetches it.

`pdf/noto-sans-arabic-regular.ttf` is Noto Sans Arabic Regular as TrueType (125 KB). It keeps the Arabic block, the Arabic presentation forms, and the OpenType features that join letters and place marks. Text boxes the reader draws in Helvetica, Times or Courier take it as their fallback font for Arabic, which those fonts lack. The reader downloads it the first time a box holds Arabic, and the PDF keeps only the glyphs it uses.

## How the files are made

`tools/fonts/subset.py` writes `files/`, `pdf/`, `fonts.css` and the `outputs` and `pdf` of `fonts.lock.json` from four npm packages and three files of the Google Fonts repository. It checks each source file against the SHA-256 in the lock and limits the variable fonts to the weights above. Each file keeps Latin or Latin Extended (the same ranges as Google Fonts' subsets) and the OpenType features text needs: kerning, marks, ligatures, contextual alternates, case forms and figure styles. Hinting is dropped, Source Sans 3 is renamed, and the result is compressed to WOFF2 (the PDF faces stay TrueType).

Running it twice gives the same bytes.

| Source | npm package | Upstream |
| --- | --- | --- |
| Source Sans 3 3.052 | `source-sans@3.52.0` (Adobe's own release) | [adobe-fonts/source-sans](https://github.com/adobe-fonts/source-sans) |
| Newsreader 1.003 | `@fontsource-variable/newsreader@5.3.0` | [productiontype/Newsreader](https://github.com/productiontype/Newsreader), as built by Google Fonts |
| Caveat 2.000 | `@fontsource/caveat@5.3.0` | [googlefonts/caveat](https://github.com/googlefonts/caveat), as built by Google Fonts |
| Noto Sans Arabic 2.012 | `@fontsource/noto-sans-arabic@5.3.0` | [notofonts/arabic](https://github.com/notofonts/arabic), as built by Google Fonts |

Newsreader, Caveat and Noto Sans Arabic come from Fontsource's packages, which repackage Google Fonts' builds of the upstream fonts, because their projects publish no npm package of their own.

The three fonts of the reader's font picker (below) come from [google/fonts](https://github.com/google/fonts) at one commit, `5026800`, because Fontsource's packages hold only WOFF2 subsets by script (Noto Sans Symbols 2 in six pieces), and the picker's fonts are whole TrueType files:

| Source | File in `google/fonts` | Upstream |
| --- | --- | --- |
| Newsreader 1.003 | `ofl/newsreader/Newsreader[opsz,wght].ttf` | [productiontype/Newsreader](https://github.com/productiontype/Newsreader) |
| JetBrains Mono 2.211 | `ofl/jetbrainsmono/JetBrainsMono[wght].ttf` | [JetBrains/JetBrainsMono](https://github.com/JetBrains/JetBrainsMono) |
| Noto Sans Symbols 2 2.008 | `ofl/notosanssymbols2/NotoSansSymbols2-Regular.ttf` | [notofonts/symbols](https://github.com/notofonts/symbols) |

The lock's `urls` give each file's address at that commit and `files` its SHA-256. Fetch them first: the script never touches the network. Save each under the name `urls` lists, next to the tarballs.

To regenerate, for example after updating a source: change its version and the SHA-256 of the files it provides in the lock's `sources`, then run, from the repository root:

```sh
python3 -m venv .venv && .venv/bin/pip install -r tools/fonts/requirements.txt
mkdir -p target/fonts && (cd target/fonts && npm pack source-sans@3.52.0 @fontsource-variable/newsreader@5.3.0 @fontsource/caveat@5.3.0 @fontsource/noto-sans-arabic@5.3.0)
python3 - <<'PY'   # the lock's "urls": each file under its own name, checked against the lock's SHA-256 by the script
import json, urllib.request
for source in json.load(open("packages/fonts/fonts.lock.json"))["sources"]:
    for name, url in source.get("urls", {}).items():
        urllib.request.urlretrieve(url, f"target/fonts/{name}")
PY
.venv/bin/python tools/fonts/subset.py target/fonts
```

`--check` instead fails unless the committed files are already what the script writes.

`tools/fonts/stamp_glyphs.py` reads Lamina Sans from `files/` at weight 700 and writes the outlines of the capitals into `crates/laminapdf-build/src/annot/appearance/stamp/generated/glyphs.rs`. Lamina PDF draws a standard stamp's name with them, as shapes, so the stamp needs no font. Run it after changing Lamina Sans; `--check` fails when the outlines are out of date.

## Licences

The fonts are under the SIL Open Font License 1.1, and each licence text ships in `LICENSES/`. The package's code (`src/`) is under the project's `MIT OR Apache-2.0`.

| Font | Version | Copyright | Licence |
| --- | --- | --- | --- |
| Lamina Sans (Source Sans 3) | 3.052 | © 2010–2024 Adobe, with Reserved Font Name "Source" | [OFL 1.1](LICENSES/SourceSans3-OFL.txt) |
| Newsreader | 1.003 | © 2020 The Newsreader Project Authors (the web and TrueType files share the licence) | [OFL 1.1](LICENSES/Newsreader-OFL.txt) |
| Caveat | 2.000 | © 2014 The Caveat Project Authors | [OFL 1.1](LICENSES/Caveat-OFL.txt) |
| Noto Sans Arabic | 2.012 | © 2022 The Noto Project Authors | [OFL 1.1](LICENSES/NotoSansArabic-OFL.txt) |
| JetBrains Mono | 2.211 | © 2020 The JetBrains Mono Project Authors | [OFL 1.1](LICENSES/JetBrainsMono-OFL.txt) |
| Noto Sans Symbols 2 | 2.008 | © 2022 The Noto Project Authors | [OFL 1.1](LICENSES/NotoSansSymbols2-OFL.txt) |
