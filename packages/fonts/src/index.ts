// @laminapdf/fonts: the UI's typefaces, served from the app's own origin. `@laminapdf/fonts/fonts.css`
// declares every face; this module lists the one file worth preloading and the TrueType files the
// reader embeds in PDFs.

/** A font file to preload, as `<link rel="preload" as="font" type="font/woff2" crossorigin href>`. */
export interface FontPreload {
  /**
   * The file's path in this package, such as `files/….woff2`. A page rendered on a server (an
   * Astro or other static build) imports it through its bundler instead of using `href`:
   * `import url from "@laminapdf/fonts/files/….woff2?url"`.
   */
  readonly file: string;
  /**
   * The file's URL, resolved against this module. A browser build (Vite and others) emits the
   * file as an asset and rewrites this to its address; on a server it is a `file:` URL.
   */
  readonly href: string;
  /** The file's media type. */
  readonly type: "font/woff2";
}

/**
 * The files to preload: Lamina Sans' Latin file only, the face of the first paint. Every other
 * face loads when text first uses it, and Latin Extended only for characters outside Latin.
 */
export const FONT_PRELOADS: readonly FontPreload[] = Object.freeze([
  Object.freeze({
    file: "files/lamina-sans-latin-wght-normal.woff2",
    href: new URL("../files/lamina-sans-latin-wght-normal.woff2", import.meta.url).href,
    type: "font/woff2",
  } as const),
]);

/** A TrueType font file the reader embeds in PDFs: a subset of it goes into each page that uses it. */
export interface PdfFont {
  /** A short name the reader's font list and the ops name the font by, such as `lamina-sans`. */
  readonly id: string;
  /** The family the font picker shows. */
  readonly family: string;
  /** The file's path in this package, such as `pdf/….ttf`. */
  readonly file: string;
  /**
   * The file's URL, resolved against this module: a browser build emits the file as an asset and
   * rewrites this to its address on the app's own origin, so the font is fetched from there, once
   * the font picker opens, and the service worker keeps it.
   */
  readonly href: string;
  /** The file's size in bytes, which `fonts.lock.json` records: what the picker names before it fetches the font. */
  readonly bytes: number;
  /**
   * Whether the font is a first choice to fill in for a character the font a text is set in lacks.
   * A serif or a monospace face among sans text looks out of place, so Lamina Sans, the symbols
   * and the scripts are tried first, and Newsreader and JetBrains Mono only for what none of those has.
   */
  readonly preferred: boolean;
  /** What the file covers, as a CSS `unicode-range`, which `fonts.lock.json` records too. */
  readonly coverage: string;
}

/**
 * The fonts for PDFs, regular style, in the order the font picker lists them: Lamina Sans (Source
 * Sans 3 under its served name, since its licence reserves "Source") for Latin and Latin Extended;
 * Newsreader, a serif; JetBrains Mono, a monospace; Noto Sans Symbols 2, for the arrows, maths,
 * dingbats and other signs the others lack (the largest, 660 KB); and Noto Sans Arabic for Arabic.
 * The list is marked pure, so a page that imports only `FONT_PRELOADS` (the reader's entry) leaves
 * it, its coverage strings and the TrueType files out of its startup bundle.
 * Newsreader, JetBrains Mono and Noto Sans Symbols 2 are whole, every character the font maps. They
 * are fetched when needed and never preloaded.
 */
export const PDF_FONTS: readonly PdfFont[] = /* @__PURE__ */ Object.freeze([
  Object.freeze({
    id: "lamina-sans",
    family: "Lamina Sans",
    file: "pdf/lamina-sans-regular.ttf",
    href: new URL("../pdf/lamina-sans-regular.ttf", import.meta.url).href,
    bytes: 102728,
    preferred: true,
    coverage:
      "U+0020-007E,U+00A0-0181,U+0186,U+018A,U+018D-0190,U+0192-0194,U+0196-0197,U+019A-019B,U+019E,U+01A0-01A1,U+01AF-01B1,U+01B7,U+01BF-01C3,U+01CD-01DD,U+01E2-01E7,U+01EA-01EB,U+01EE-01F0,U+01F4-01F5,U+01F7-0201,U+0204-0205,U+0208-0209,U+020C-020D,U+0218-0220,U+0222-0223,U+0226-0227,U+0237,U+023A-023E,U+0241-0245,U+0248-0249,U+0250-027B,U+027D-0285,U+0287-0292,U+0294-0295,U+0298-0299,U+029B-029F,U+02A1-02A8,U+02B0-02B4,U+02B7-02B9,U+02BB-02C1,U+02C6-02CC,U+02D0-02D7,U+02DA,U+02DC-02DE,U+02E0-02E4,U+02ED,U+0304,U+0308,U+0329,U+1D25,U+1D2C,U+1D2E,U+1D30-1D31,U+1D33-1D3A,U+1D3C,U+1D3E-1D43,U+1D47-1D4B,U+1D4D,U+1D4F-1D52,U+1D56-1D58,U+1D5B,U+1D6F-1D70,U+1D79,U+1D7B,U+1D7F,U+1D9C,U+1DA0,U+1DAC,U+1DAE,U+1DB0,U+1DBB,U+1DBF,U+1E02-1E03,U+1E06-1E07,U+1E0A-1E11,U+1E16-1E17,U+1E1E-1E2B,U+1E32-1E3B,U+1E3E-1E49,U+1E52-1E53,U+1E56-1E63,U+1E66-1E67,U+1E6A-1E6F,U+1E72-1E73,U+1E7E-1E85,U+1E8E-1E97,U+1E9E-1E9F,U+1EF2-1EF9,U+2007,U+2009-200B,U+2010-201E,U+2020-2023,U+2026,U+202F-2033,U+2035,U+2039-203A,U+203C-203F,U+2044,U+2047-204A,U+20A1,U+20A3-20A4,U+20A6-20A7,U+20A9,U+20AB-20AC,U+20AE,U+20B1-20B2,U+20B4-20B5,U+20B8-20BA,U+20BD,U+2113,U+2122,U+2190-2193,U+21E7,U+2212,U+2215,U+2318,U+2325,U+232B,U+2713,U+2C60-2C62,U+2C65-2C66,U+2C71,U+2C7D,U+A722-A725,U+A74E-A74F,U+A75A-A75B,U+A764-A765,U+A779-A787,U+A789,U+A7AD-A7AE,U+A7B3,U+A7B5,U+A7F2-A7F4,U+FEFF",
  }),
  Object.freeze({
    id: "newsreader",
    family: "Newsreader",
    file: "pdf/newsreader-regular.ttf",
    href: new URL("../pdf/newsreader-regular.ttf", import.meta.url).href,
    bytes: 109932,
    preferred: false,
    coverage:
      "U+0020-007E,U+00A0-0131,U+0134-0148,U+014A-017E,U+018F,U+0192,U+01A0-01A1,U+01AF-01B0,U+01C4-01CC,U+01E6-01E7,U+01EA-01EB,U+01FA-021B,U+022A-022D,U+0230-0233,U+0237,U+0259,U+02B9-02BA,U+02BC,U+02C6-02C7,U+02D8-02DD,U+0300-0304,U+0306-030C,U+030F,U+0311-0312,U+031B,U+0323-0324,U+0326-0328,U+032E,U+0331,U+0335,U+1E80-1E85,U+1E9E,U+1EA0-1EF9,U+2010,U+2013-2014,U+2018-201A,U+201C-201E,U+2020-2022,U+2026,U+2030,U+2032-2033,U+2039-203A,U+2044,U+2052,U+2070,U+2074,U+20A1,U+20A3-20A4,U+20A6-20A7,U+20A9,U+20AB-20AD,U+20B1-20B2,U+20B5,U+20B9-20BA,U+20BC-20BD,U+2116,U+2122,U+2212,U+2215,U+2219,U+2248,U+2260,U+2264-2265,U+27E8-27E9,U+FB00-FB04",
  }),
  Object.freeze({
    id: "jetbrains-mono",
    family: "JetBrains Mono",
    file: "pdf/jetbrains-mono-regular.ttf",
    href: new URL("../pdf/jetbrains-mono-regular.ttf", import.meta.url).href,
    bytes: 100440,
    preferred: false,
    coverage:
      "U+000D,U+0020-007E,U+00A0-0131,U+0134-017F,U+018F,U+0192,U+01A0-01A1,U+01AF-01B0,U+01CD-01CE,U+01E6-01E7,U+01EA-01EB,U+01F4-01F5,U+01FC-01FF,U+0218-021B,U+0232-0233,U+0237,U+0259,U+02B9-02BA,U+02BC,U+02C6-02C7,U+02C9,U+02D8-02DD,U+02F3,U+02F7,U+0300-0304,U+0306-030C,U+030F,U+0312,U+031B,U+0323,U+0325-0328,U+0336,U+0338,U+0374-0375,U+037E,U+0384-038A,U+038C,U+038E-03A1,U+03A3-03CF,U+03D7,U+0401-040C,U+040E-044F,U+0451-045C,U+045E-045F,U+0490-0491,U+04AE-04AF,U+04E8-04E9,U+1E80-1E85,U+1E9E,U+1EA0-1EF9,U+2010,U+2013-2014,U+2018-201A,U+201C-201E,U+2020-2022,U+2024,U+2026,U+2030,U+2032-2033,U+2039-203A,U+2044,U+2070,U+2074-207A,U+2080-2089,U+20AB-20AC,U+20AE,U+20BD,U+20BF,U+2113,U+2115-2116,U+2122,U+212E,U+2190-2199,U+21E7,U+2200,U+2202-2203,U+2208-2209,U+220B-220C,U+220F-2212,U+2215,U+2218-221A,U+221E,U+2223,U+2225,U+2227-2228,U+222B,U+2237,U+223C,U+2248-2249,U+2260-2261,U+2264-2265,U+2282-2287,U+2302-2305,U+2318,U+2325,U+2374,U+23FB-23FE,U+2500-25A1,U+25AA-25AB,U+25B2-25C7,U+25CA,U+25CC,U+25CE-25CF,U+25D4-25D5,U+25E7-25EB,U+25EF,U+26A0-26A1,U+2715,U+276E-2771,U+27E8-27E9,U+27F5-27F7,U+2B58,U+E0A0-E0A2,U+E0B0-E0B3,U+FEFF",
  }),
  Object.freeze({
    id: "noto-sans-symbols-2",
    family: "Noto Sans Symbols 2",
    file: "pdf/noto-sans-symbols-2-regular.ttf",
    href: new URL("../pdf/noto-sans-symbols-2-regular.ttf", import.meta.url).href,
    bytes: 676692,
    preferred: true,
    coverage:
      "U+0000,U+000D,U+0020-007F,U+00A0-00A3,U+00A5,U+00A7-00AB,U+00AE-00B0,U+00B4,U+00B6-00B8,U+00BA-00BB,U+00BF-0107,U+010A-0113,U+0116-011B,U+011E-0123,U+0126-0127,U+012A-012B,U+012E-0131,U+0136-0137,U+0139-013E,U+0141-0148,U+014A-014D,U+0150-015B,U+015E-0161,U+0164-0165,U+016A-017E,U+0218-021B,U+0237,U+02C6-02C7,U+02C9,U+02D8-02DD,U+0300-0304,U+0306-0308,U+030A-030C,U+0312,U+0326-0328,U+1E80-1E85,U+1E9E,U+1EF2-1EF3,U+2013-2014,U+2018-201A,U+201C-201E,U+2022,U+2026,U+2039-203A,U+20AC,U+20E2-20E3,U+2122,U+21AF,U+21E6-21F0,U+21F3,U+2212,U+2218-2219,U+2299,U+22C4-22C6,U+2316,U+2318,U+231A-231B,U+2324-2328,U+232B,U+237B,U+237D-237F,U+2394,U+23CE-23CF,U+23E9-23EA,U+23ED-23EF,U+23F1-2426,U+2440-244A,U+25A0-2609,U+260E-2612,U+2614-2623,U+2630-2637,U+263C,U+2654-2668,U+267F-268F,U+269E-26A1,U+26AA-26AC,U+26BD-26CD,U+26CF-26E1,U+2700-2704,U+2706-2709,U+270B-271C,U+2722-2727,U+2729-274B,U+274D,U+274F-2753,U+2756-2775,U+2794,U+2798-27AF,U+27B1-27BE,U+2800-28FF,U+2981,U+29BF,U+29EB,U+2B00-2B0D,U+2B12-2B2F,U+2B4D-2B73,U+2B76-2B95,U+2B97-2BFD,U+2BFF,U+4DC0-4DFF,U+FFF9-FFFB,U+10140-1018E,U+10190-1019C,U+101A0,U+101D0-101FD,U+102E0-102FB,U+10E60-10E7E,U+1D2C0-1D2D3,U+1D2E0-1D2F3,U+1D300-1D356,U+1D360-1D378,U+1F000-1F02B,U+1F030-1F093,U+1F0A0-1F0AE,U+1F0B1-1F0BF,U+1F0C1-1F0CF,U+1F0D1-1F0F5,U+1F10D-1F10F,U+1F16D-1F16F,U+1F1AD,U+1F30D-1F30F,U+1F315,U+1F31C,U+1F321-1F32C,U+1F336,U+1F378,U+1F37D,U+1F393-1F39F,U+1F3A7,U+1F3AC-1F3AE,U+1F3C2,U+1F3C4,U+1F3C6,U+1F3CA-1F3CE,U+1F3D4-1F3E0,U+1F3ED,U+1F3F1-1F3F3,U+1F3F5-1F3F7,U+1F408,U+1F415,U+1F41F,U+1F426,U+1F43F,U+1F441-1F442,U+1F446-1F449,U+1F44C-1F44E,U+1F453,U+1F46A,U+1F47D,U+1F4A3,U+1F4B0,U+1F4B3,U+1F4B9,U+1F4BB,U+1F4BF,U+1F4C8-1F4CB,U+1F4DA,U+1F4DF,U+1F4E4-1F4E6,U+1F4EA-1F4ED,U+1F4F7,U+1F4F9-1F4FB,U+1F4FD-1F4FE,U+1F503,U+1F507-1F50A,U+1F50D,U+1F512-1F513,U+1F53E-1F545,U+1F54A,U+1F550-1F579,U+1F57B-1F594,U+1F597-1F5A3,U+1F5A5-1F5FA,U+1F650-1F67F,U+1F687,U+1F68D,U+1F691,U+1F694,U+1F698,U+1F6AD,U+1F6B2,U+1F6B9-1F6BA,U+1F6BC,U+1F6C6-1F6CB,U+1F6CD-1F6CF,U+1F6D3-1F6D7,U+1F6E0-1F6EA,U+1F6F0-1F6F3,U+1F6F7-1F6FC,U+1F774-1F776,U+1F77B-1F7D9,U+1F7E0-1F7EB,U+1F800-1F80B,U+1F810-1F847,U+1F850-1F859,U+1F860-1F887,U+1F890-1F8AD,U+1F8B0-1F8B1,U+1F900-1F90B,U+1F93B,U+1F946,U+1FA00-1FA53,U+1FA60-1FA6D,U+1FA70-1FA74,U+1FA78-1FA7A,U+1FA80-1FA86,U+1FA90-1FAA8,U+1FAB0-1FAB6,U+1FAC0-1FAC2,U+1FAD0-1FAD6,U+1FB00-1FB92,U+1FB94-1FBCA,U+1FBF0-1FBF9",
  }),
  Object.freeze({
    id: "noto-sans-arabic",
    family: "Noto Sans Arabic",
    file: "pdf/noto-sans-arabic-regular.ttf",
    href: new URL("../pdf/noto-sans-arabic-regular.ttf", import.meta.url).href,
    bytes: 127860,
    preferred: true,
    coverage:
      "U+0600-06FF,U+0750-077F,U+0870-088E,U+0890-0891,U+0898-08E1,U+08E3-08FF,U+200C-200E,U+2010-2011,U+204F,U+2E41,U+FB50-FBC2,U+FBD3-FD8F,U+FD92-FDC7,U+FDCF,U+FDF0-FDFF,U+FE70-FE74,U+FE76-FEFC",
  }),
] as const);
