// The fonts a reader session can set added text in (owner's decision, issue #532): the standard 14
// families, which need no bytes; the fonts the app bundles, fetched from its own origin when the
// font picker opens and kept by the service worker; and font files the user adds. Fonts on the
// computer are the desktop's (E6h). The core embeds whatever bytes it is given, so this only names
// the fonts, holds the bytes, and refuses a file the core would refuse, early and in words.

import { PDF_FONTS, type PdfFont } from "@laminapdf/fonts";
import { type CodePoints, parseRange } from "./ranges.ts";
import { readFont } from "./sfnt.ts";

/** Where a font comes from. */
type FontSource = "standard" | "bundled" | "added";

/** A font the picker offers. */
export interface FontChoice {
  /** What ops name it by: `helvetica`, `times` or `courier` for the standard fonts. */
  readonly id: string;
  /** The name the picker shows. */
  readonly family: string;
  readonly source: FontSource;
  /** The characters it has; the standard fonts have what `WinAnsiEncoding` encodes. */
  readonly coverage: CodePoints;
}

/** What `WinAnsiEncoding` encodes, which Helvetica, Times and Courier draw (ISO 32000-2 Annex D). */
const WIN_ANSI = parseRange(
  "U+0020-007E,U+00A0-00FF,U+0152-0153,U+0160-0161,U+0178,U+017D-017E,U+0192,U+02C6,U+02DC,U+2013-2014,U+2018-201A,U+201C-201E,U+2020-2022,U+2026,U+2030,U+2039-203A,U+20AC,U+2122",
);

/** The standard fonts: no bytes, in every module that lays out text boxes. */
export const STANDARD_FONTS: readonly FontChoice[] = [
  { id: "helvetica", family: "Helvetica", source: "standard", coverage: WIN_ANSI },
  { id: "times", family: "Times", source: "standard", coverage: WIN_ANSI },
  { id: "courier", family: "Courier", source: "standard", coverage: WIN_ANSI },
];

/** Why a font file was refused. */
export type FontFileProblem =
  | "empty"
  | "damaged"
  | "web-font"
  | "unknown-format"
  /** Its licence forbids embedding it (`OS/2` `fsType`); the core refuses it too, with `content-font-not-embeddable`. */
  | "not-embeddable"
  | "too-big";

/** The largest font file taken, in bytes: a font with every script is about 25 MB. */
const MAX_FONT_BYTES = 32 * 1024 * 1024;

/** A refused font file. */
export class FontFileError extends Error {
  readonly problem: FontFileProblem;

  constructor(problem: FontFileProblem, file: string) {
    super(`${file} can't be used as a font: ${problem}`);
    this.name = "FontFileError";
    this.problem = problem;
  }
}

/** Fetches a bundled font's file: `fetch` unless a test says otherwise. */
export type FontFetch = (href: string) => Promise<Uint8Array>;

async function fetchFont(href: string): Promise<Uint8Array> {
  const answer = await fetch(href);
  if (!answer.ok) throw new Error(`the bundled font ${href} answered ${answer.status}`);
  return new Uint8Array(await answer.arrayBuffer());
}

/** What a session knows of its fonts. */
export class FontRegistry {
  readonly #bundled: readonly (FontChoice & { readonly href: string })[];
  readonly #fetch: FontFetch;
  readonly #added = new Map<string, { readonly choice: FontChoice; readonly bytes: Uint8Array }>();
  readonly #fetched = new Map<string, Promise<Uint8Array>>();
  #sequence = 0;

  constructor(options: { readonly bundled?: readonly PdfFont[]; readonly fetch?: FontFetch } = {}) {
    this.#bundled = (options.bundled ?? PDF_FONTS).map((font) => ({
      id: font.id,
      family: font.family,
      source: "bundled",
      coverage: parseRange(font.coverage),
      href: font.href,
    }));
    this.#fetch = options.fetch ?? fetchFont;
  }

  /** The fonts the picker offers, in order: standard, bundled, then the user's, oldest first. */
  list(): readonly FontChoice[] {
    return [...STANDARD_FONTS, ...this.#bundled, ...[...this.#added.values()].map((a) => a.choice)];
  }

  /** The font named `id`. */
  get(id: string): FontChoice | undefined {
    return this.list().find((font) => font.id === id);
  }

  /**
   * The bytes of font `id`: fetched from the app's own origin the first time for a bundled font
   * (a failed fetch is tried again next time), as added for the user's, and none for a standard
   * font, which needs none.
   *
   * @throws if the font is unknown or the fetch fails.
   */
  async bytes(id: string): Promise<Uint8Array | undefined> {
    if (STANDARD_FONTS.some((font) => font.id === id)) return undefined;
    const added = this.#added.get(id);
    if (added) return added.bytes;
    const bundled = this.#bundled.find((font) => font.id === id);
    if (!bundled) throw new Error(`no font ${id}`);
    let pending = this.#fetched.get(id);
    if (!pending) {
      pending = this.#fetch(bundled.href);
      this.#fetched.set(id, pending);
      pending.catch(() => this.#fetched.delete(id));
    }
    return pending;
  }

  /**
   * Adds a font file the user chose. A TrueType or OpenType file is taken; a web font (WOFF,
   * WOFF2), which the core can't read, and a font whose licence forbids embedding are refused.
   *
   * @throws {FontFileError}
   */
  add(file: string, bytes: Uint8Array): FontChoice {
    if (bytes.length > MAX_FONT_BYTES) throw new FontFileError("too-big", file);
    const facts = readFont(bytes);
    if (typeof facts === "string") throw new FontFileError(facts, file);
    if (facts.restricted) throw new FontFileError("not-embeddable", file);
    this.#sequence += 1;
    const id = `added-${this.#sequence}`;
    const stem = file.replace(/\.[^.]*$/, "");
    const choice: FontChoice = {
      id,
      family: facts.family || stem || id,
      source: "added",
      coverage: facts.coverage,
    };
    this.#added.set(id, { choice, bytes });
    return choice;
  }

  /** Takes a font the user added off the list. */
  remove(id: string): void {
    this.#added.delete(id);
  }
}
