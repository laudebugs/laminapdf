// @laminapdf/fonts: the UI's typefaces, served from the app's own origin. `@laminapdf/fonts/fonts.css`
// declares every face; this module lists the one file worth preloading.

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
