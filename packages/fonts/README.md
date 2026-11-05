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

## How the files are made

`tools/fonts/subset.py` writes `files/`, `fonts.css` and the `outputs` of `fonts.lock.json` from three npm packages. It checks each source file against the SHA-256 in the lock and limits the variable fonts to the weights above. Each file keeps Latin or Latin Extended (the same ranges as Google Fonts' subsets) and the OpenType features text needs: kerning, marks, ligatures, contextual alternates, case forms and figure styles. Hinting is dropped, Source Sans 3 is renamed, and the result is compressed to WOFF2. Running it twice gives the same bytes.

| Source | npm package | Upstream |
| --- | --- | --- |
| Source Sans 3 3.052 | `source-sans@3.52.0` (Adobe's own release) | [adobe-fonts/source-sans](https://github.com/adobe-fonts/source-sans) |
| Newsreader 1.003 | `@fontsource-variable/newsreader@5.3.0` | [productiontype/Newsreader](https://github.com/productiontype/Newsreader), as built by Google Fonts |
| Caveat 2.000 | `@fontsource/caveat@5.3.0` | [googlefonts/caveat](https://github.com/googlefonts/caveat), as built by Google Fonts |

Newsreader and Caveat come from Fontsource's packages, which repackage Google Fonts' builds of the upstream fonts, because their projects publish no npm package of their own.

To regenerate, for example after updating a source: change its version and the SHA-256 of the files it provides in the lock's `sources`, then run, from the repository root:

```sh
python3 -m venv .venv && .venv/bin/pip install -r tools/fonts/requirements.txt
mkdir -p target/fonts && (cd target/fonts && npm pack source-sans@3.52.0 @fontsource-variable/newsreader@5.3.0 @fontsource/caveat@5.3.0)
.venv/bin/python tools/fonts/subset.py target/fonts
```

`--check` instead fails unless the committed files are already what the script writes.

## Licences

The fonts are under the SIL Open Font License 1.1, and each licence text ships in `LICENSES/`. The package's code (`src/`) is under the project's `MIT OR Apache-2.0`.

| Font | Version | Copyright | Licence |
| --- | --- | --- | --- |
| Lamina Sans (Source Sans 3) | 3.052 | © 2010–2024 Adobe, with Reserved Font Name "Source" | [OFL 1.1](LICENSES/SourceSans3-OFL.txt) |
| Newsreader | 1.003 | © 2020 The Newsreader Project Authors | [OFL 1.1](LICENSES/Newsreader-OFL.txt) |
| Caveat | 2.000 | © 2014 The Caveat Project Authors | [OFL 1.1](LICENSES/Caveat-OFL.txt) |
