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
  /** What the file covers, as a CSS `unicode-range`, which `fonts.lock.json` records too. */
  readonly coverage: string;
}

/**
 * The fonts for PDFs, regular style: Lamina Sans (Source Sans 3 under its served name, since its
 * licence reserves "Source") for Latin and Latin Extended, and Noto Sans Arabic for Arabic. They
 * are fetched when needed and never preloaded.
 */
export const PDF_FONTS: readonly PdfFont[] = Object.freeze([
  Object.freeze({
    id: "lamina-sans",
    family: "Lamina Sans",
    file: "pdf/lamina-sans-regular.ttf",
    href: new URL("../pdf/lamina-sans-regular.ttf", import.meta.url).href,
    coverage:
      "U+0020-007E,U+00A0-0181,U+0186,U+018A,U+018D-0190,U+0192-0194,U+0196-0197,U+019A-019B,U+019E,U+01A0-01A1,U+01AF-01B1,U+01B7,U+01BF-01C3,U+01CD-01DD,U+01E2-01E7,U+01EA-01EB,U+01EE-01F0,U+01F4-01F5,U+01F7-0201,U+0204-0205,U+0208-0209,U+020C-020D,U+0218-0220,U+0222-0223,U+0226-0227,U+0237,U+023A-023E,U+0241-0245,U+0248-0249,U+0250-027B,U+027D-0285,U+0287-0292,U+0294-0295,U+0298-0299,U+029B-029F,U+02A1-02A8,U+02B0-02B4,U+02B7-02B9,U+02BB-02C1,U+02C6-02CC,U+02D0-02D7,U+02DA,U+02DC-02DE,U+02E0-02E4,U+02ED,U+0304,U+0308,U+0329,U+1D25,U+1D2C,U+1D2E,U+1D30-1D31,U+1D33-1D3A,U+1D3C,U+1D3E-1D43,U+1D47-1D4B,U+1D4D,U+1D4F-1D52,U+1D56-1D58,U+1D5B,U+1D6F-1D70,U+1D79,U+1D7B,U+1D7F,U+1D9C,U+1DA0,U+1DAC,U+1DAE,U+1DB0,U+1DBB,U+1DBF,U+1E02-1E03,U+1E06-1E07,U+1E0A-1E11,U+1E16-1E17,U+1E1E-1E2B,U+1E32-1E3B,U+1E3E-1E49,U+1E52-1E53,U+1E56-1E63,U+1E66-1E67,U+1E6A-1E6F,U+1E72-1E73,U+1E7E-1E85,U+1E8E-1E97,U+1E9E-1E9F,U+1EF2-1EF9,U+2007,U+2009-200B,U+2010-201E,U+2020-2023,U+2026,U+202F-2033,U+2035,U+2039-203A,U+203C-203F,U+2044,U+2047-204A,U+20A1,U+20A3-20A4,U+20A6-20A7,U+20A9,U+20AB-20AC,U+20AE,U+20B1-20B2,U+20B4-20B5,U+20B8-20BA,U+20BD,U+2113,U+2122,U+2190-2193,U+21E7,U+2212,U+2215,U+2318,U+2325,U+232B,U+2713,U+2C60-2C62,U+2C65-2C66,U+2C71,U+2C7D,U+A722-A725,U+A74E-A74F,U+A75A-A75B,U+A764-A765,U+A779-A787,U+A789,U+A7AD-A7AE,U+A7B3,U+A7B5,U+A7F2-A7F4,U+FEFF",
  }),
  Object.freeze({
    id: "noto-sans-arabic",
    family: "Noto Sans Arabic",
    file: "pdf/noto-sans-arabic-regular.ttf",
    href: new URL("../pdf/noto-sans-arabic-regular.ttf", import.meta.url).href,
    coverage:
      "U+0600-06FF,U+0750-077F,U+0870-088E,U+0890-0891,U+0898-08E1,U+08E3-08FF,U+200C-200E,U+2010-2011,U+204F,U+2E41,U+FB50-FBC2,U+FBD3-FD8F,U+FD92-FDC7,U+FDCF,U+FDF0-FDFF,U+FE70-FE74,U+FE76-FEFC",
  }),
] as const);
