// The font registry, the fallback chain and the symbol picker's data, over the fixtures' fonts.
import { readFileSync } from "node:fs";
import { PDF_FONTS } from "@laminapdf/fonts/pdf";
import { fromRoot } from "@laminapdf/test-utils/root";
import { describe, expect, it } from "vitest";
import { DocumentFonts } from "../src/fonts/document-fonts.ts";
import { fallbackChain } from "../src/fonts/fallback.ts";
import { covers, parseRange } from "../src/fonts/ranges.ts";
import { FontFileError, FontRegistry, STANDARD_FONTS } from "../src/fonts/registry.ts";
import { readFont } from "../src/fonts/sfnt.ts";
import { findSymbols, SYMBOL_GROUPS } from "../src/fonts/symbols.ts";

const font = (name: string) => new Uint8Array(readFileSync(fromRoot(`fixtures/fonts/${name}`)));

/** `bytes` with `OS/2.fsType` set to `fsType`. */
function withFsType(bytes: Uint8Array, fsType: number): Uint8Array {
  const out = bytes.slice();
  const view = new DataView(out.buffer);
  for (let i = 0; i < view.getUint16(4); i++) {
    const record = 12 + 16 * i;
    if (String.fromCharCode(...out.subarray(record, record + 4)) === "OS/2") {
      view.setUint16(view.getUint32(record + 8) + 8, fsType);
    }
  }
  return out;
}

/** A registry whose bundled fonts come from the repository's files, counting the fetches. */
function registry() {
  const fetched: string[] = [];
  const files = new Map(PDF_FONTS.map((f) => [f.href, readFileSync(fromRoot(`packages/fonts/${f.file}`))]));
  const fetch = async (href: string) => {
    fetched.push(href);
    const bytes = files.get(href);
    if (!bytes) throw new Error("not found");
    return new Uint8Array(bytes);
  };
  return { registry: new FontRegistry({ fetch }), fetched };
}

describe("ranges", () => {
  it("reads a unicode-range and finds characters in it", () => {
    const set = parseRange("U+0020-007E, U+00A0,U+2013-2014,bogus,U+0030");
    expect([0x20, 0x41, 0x7e, 0xa0, 0x2014, 0x7f, 0xa1].map((c) => covers(set, c))).toEqual([
      true,
      true,
      true,
      true,
      true,
      false,
      false,
    ]);
  });
});

/** A font with a format 4 `cmap` and nothing else: A to E through a glyph array (C is glyph 0), b
 * by a delta (a is glyph 0), and the sentinel. */
function format4Font(): Uint8Array {
  const words = [
    4,
    0,
    0,
    6,
    0,
    0,
    0, // format, length (set below), language, segCountX2, search fields
    0x45,
    0x62,
    0xffff, // end codes
    0, // reserved pad
    0x41,
    0x61,
    0xffff, // start codes
    0,
    (0 - 0x61) & 0xffff,
    1, // deltas
    6,
    0,
    0, // range offsets: the first is 6 bytes to the glyph array
    1,
    2,
    0,
    3,
    4, // glyphs of A to E
  ];
  words[1] = words.length * 2;
  const table = new Uint8Array(12 + 16 + 12 + words.length * 2);
  const view = new DataView(table.buffer);
  view.setUint32(0, 0x00010000);
  view.setUint16(4, 1);
  table.set([...new TextEncoder().encode("cmap")], 12);
  view.setUint32(20, 28);
  view.setUint32(24, 12 + words.length * 2);
  // cmap header (version, one table), its record (platform 3, encoding 1, offset 12), then the subtable.
  for (const [i, w] of [0, 1, 3, 1].entries()) view.setUint16(28 + 2 * i, w);
  view.setUint32(36, 12);
  for (const [i, w] of words.entries()) view.setUint16(40 + 2 * i, w);
  return table;
}

describe("a font file", () => {
  it("does not cover a character its cmap maps to glyph 0, whichever way the segment says so", () => {
    const facts = readFont(format4Font());
    if (typeof facts === "string") throw new Error(facts);
    expect([0x41, 0x42, 0x43, 0x44, 0x45, 0x61, 0x62].map((c) => covers(facts.coverage, c))).toEqual([
      true,
      true,
      false,
      true,
      true,
      false,
      true,
    ]);
  });

  it("has a family, a licence verdict and the characters of its cmap", () => {
    const facts = readFont(font("NotoSans-Regular.ttf"));
    if (typeof facts === "string") throw new Error(facts);
    expect(facts.family).toBe("Noto Sans");
    expect(facts.restricted).toBe(false);
    expect([0x41, 0xfc, 0x5e8].map((c) => covers(facts.coverage, c))).toEqual([true, true, false]);
  });

  it("covers a script of its own, in a format 12 or 4 table alike", () => {
    for (const [file, character] of [
      ["NotoSansHebrew-Regular.ttf", "ש"],
      ["NotoSansArabic-Regular.ttf", "م"],
      ["NotoSansSC-Regular.otf", "你"],
    ] as const) {
      const facts = readFont(font(file));
      if (typeof facts === "string") throw new Error(`${file}: ${facts}`);
      expect(covers(facts.coverage, character.codePointAt(0) ?? 0)).toBe(true);
    }
  });

  it("is refused when it is a web font, empty, damaged or not a font", () => {
    expect(readFont(new Uint8Array())).toBe("empty");
    expect(readFont(new TextEncoder().encode("wOF2....rest of a web font"))).toBe("web-font");
    expect(readFont(new TextEncoder().encode("%PDF-1.7 and some more bytes"))).toBe("unknown-format");
    expect(readFont(font("NotoSans-Regular.ttf").subarray(0, 40))).toBe("damaged");
  });

  it("is restricted by a licence that forbids embedding, unless a looser bit is set", () => {
    const noto = font("NotoSans-Regular.ttf");
    const restricted = (fsType: number) => {
      const facts = readFont(withFsType(noto, fsType));
      return typeof facts === "string" ? facts : facts.restricted;
    };
    expect([restricted(0x0002), restricted(0x0004), restricted(0x000a), restricted(0x0200), restricted(0)]).toEqual([
      true,
      false,
      false,
      true,
      false,
    ]);
  });
});

describe("FontRegistry", () => {
  it("lists the standard fonts, the bundled ones, then those the user adds", () => {
    const { registry: fonts } = registry();
    const added = fonts.add("My Face.ttf", font("NotoSansHebrew-Regular.ttf"));
    expect(fonts.list().map((f) => [f.id, f.source])).toEqual([
      ...STANDARD_FONTS.map((f) => [f.id, "standard"]),
      ...PDF_FONTS.map((f) => [f.id, "bundled"]),
      [added.id, "added"],
    ]);
    expect(added.family).toBe("Noto Sans Hebrew");
    fonts.remove(added.id);
    expect(fonts.get(added.id)).toBeUndefined();
  });

  it("fetches a bundled font once, tries again after a failure, and has no bytes for a standard font", async () => {
    const { registry: fonts, fetched } = registry();
    expect(await fonts.bytes("helvetica")).toBeUndefined();
    const [first, second] = await Promise.all([fonts.bytes("lamina-sans"), fonts.bytes("lamina-sans")]);
    expect(first).toBe(second);
    expect(fetched).toHaveLength(1);
    const offline = new FontRegistry({
      fetch: async () => {
        throw new Error("offline");
      },
    });
    await expect(offline.bytes("lamina-sans")).rejects.toThrow("offline");
    await expect(offline.bytes("lamina-sans")).rejects.toThrow("offline");
    await expect(offline.bytes("no-such-font")).rejects.toThrow("no font");
  });

  it("refuses a web font and a font that forbids embedding, with the problem", () => {
    const { registry: fonts } = registry();
    const refusal = (file: string, bytes: Uint8Array) => {
      try {
        fonts.add(file, bytes);
      } catch (error) {
        return error instanceof FontFileError ? error.problem : String(error);
      }
      return "taken";
    };
    expect(refusal("a.woff2", new TextEncoder().encode("wOF2 and more bytes here"))).toBe("web-font");
    expect(refusal("a.ttf", withFsType(font("NotoSans-Regular.ttf"), 0x0002))).toBe("not-embeddable");
    expect(refusal("a.ttf", new Uint8Array(0))).toBe("empty");
    expect(refusal("a.ttf", font("NotoSans-Regular.ttf"))).toBe("taken");
  });
});

describe("fallbackChain", () => {
  it("needs only the primary font for text it has, and spaces and line breaks draw nothing", () => {
    const { registry: fonts } = registry();
    expect(fallbackChain(fonts.list(), "helvetica", "Zoë reviewed it\n twice")).toEqual({
      fonts: ["helvetica"],
      missing: [],
    });
  });

  it("adds each font a character needs, once, in the order it was needed", () => {
    const { registry: fonts } = registry();
    const hebrew = fonts.add("Hebrew.ttf", font("NotoSansHebrew-Regular.ttf"));
    const plan = fallbackChain(fonts.list(), "helvetica", "Ł שלום مرحبا € ש");
    expect(plan).toEqual({ fonts: ["helvetica", "lamina-sans", hebrew.id, "noto-sans-arabic"], missing: [] });
  });

  it("asks no font for what the core ignores, and for a combining mark as it does", () => {
    const { registry: fonts } = registry();
    const joined = "a\u200d\u200c\u00ad\ufe0f\u2060\u200e\u2028b";
    expect(fallbackChain(fonts.list(), "lamina-sans", joined)).toEqual({ fonts: ["lamina-sans"], missing: [] });
    // U+20DD has a glyph of its own to find, so a font without it is reported, as the core does.
    expect(fallbackChain(fonts.list(), "helvetica", "e\u20dd").missing).toEqual(["\u20dd"]);
  });

  it("lists the characters nothing covers, each once", () => {
    const { registry: fonts } = registry();
    expect(fallbackChain(fonts.list(), "lamina-sans", "a 你 b 你 ∑").missing).toEqual(["你"]);
  });

  it("sets text in Newsreader or the monospace alone when they have it all", () => {
    const { registry: fonts } = registry();
    for (const id of ["newsreader", "jetbrains-mono"]) {
      expect(fallbackChain(fonts.list(), id, "Zoë reviewed it, twice: 0xFF € ™")).toEqual({ fonts: [id], missing: [] });
    }
  });

  it("never takes a standard font as a fallback", () => {
    const { registry: fonts } = registry();
    expect(fallbackChain(fonts.list(), "lamina-sans", "Ł").fonts).toEqual(["lamina-sans"]);
  });
});

describe("DocumentFonts", () => {
  it("adds each font's bytes to the document once and names standard fonts as they are", async () => {
    const { registry: fonts } = registry();
    const added: Uint8Array[] = [];
    const doc = new DocumentFonts(fonts, async (bytes) => {
      added.push(bytes);
      return `blob-${added.length}`;
    });
    const first = await doc.forText("helvetica", "Hello Ł مرحبا");
    const second = await doc.forText("helvetica", "Łódź");
    expect(first.fonts).toEqual(["helvetica", "blob-1", "blob-2"]);
    expect(second.fonts).toEqual(["helvetica", "blob-1"]);
    expect(added).toHaveLength(2);
  });
});

describe("the symbol picker's data", () => {
  it("has groups of distinct characters, each named", () => {
    const characters = SYMBOL_GROUPS.flatMap((g) => g.symbols.map((s) => s.character));
    expect(new Set(characters).size).toBe(characters.length);
    expect(SYMBOL_GROUPS.every((g) => g.symbols.every((s) => s.name !== ""))).toBe(true);
    expect(SYMBOL_GROUPS.map((g) => g.id)).toEqual([
      "punctuation",
      "arrows",
      "math",
      "currency",
      "marks",
      "shapes",
      "fractions",
    ]);
  });

  it("finds a symbol by its character or part of its name, ignoring case", () => {
    expect(findSymbols("Check").map((s) => s.character)).toContain("✓");
    expect(findSymbols("→").map((s) => s.name)).toContain("right arrow");
    expect(findSymbols("  ")).toEqual([]);
  });

  it("finds a font for every character of every group, whichever font the text is set in", () => {
    const { registry: fonts } = registry();
    for (const group of SYMBOL_GROUPS) {
      const text = group.symbols.map((symbol) => symbol.character).join("");
      for (const primary of ["helvetica", "times", "courier", ...PDF_FONTS.map((f) => f.id)]) {
        expect({ group: group.id, primary, missing: fallbackChain(fonts.list(), primary, text).missing }).toEqual({
          group: group.id,
          primary,
          missing: [],
        });
      }
    }
  });

  it("draws the dingbats in Noto Sans Symbols 2, the arrows and marks in Lamina Sans, and the maths it lacks in the monospace", () => {
    const { registry: fonts } = registry();
    expect(fallbackChain(fonts.list(), "lamina-sans", "a ★ ✓ → ©").fonts).toEqual([
      "lamina-sans",
      "noto-sans-symbols-2",
    ]);
    // Noto Sans Symbols 2 has no mathematical operators: JetBrains Mono, the last resort, has them.
    expect(fallbackChain(fonts.list(), "lamina-sans", "∑ ★").fonts).toEqual([
      "lamina-sans",
      "jetbrains-mono",
      "noto-sans-symbols-2",
    ]);
    expect(fallbackChain(fonts.list(), "lamina-sans", "→ ✓ © ’").fonts).toEqual(["lamina-sans"]);
    // A shape or dingbat in a font that lacks it takes the symbol font, which has them.
    expect(fallbackChain(fonts.list(), "newsreader", "a★").fonts).toEqual(["newsreader", "noto-sans-symbols-2"]);
  });
});

describe("the bundled fonts' files", () => {
  const lockedCoverage = (file: string) => {
    const facts = readFont(new Uint8Array(readFileSync(fromRoot(`packages/fonts/${file}`))));
    if (typeof facts === "string") throw new Error(`${file}: ${facts}`);
    return facts;
  };

  it("are TrueType with a licence that allows embedding, covering what the list says", () => {
    for (const bundled of PDF_FONTS) {
      const facts = lockedCoverage(bundled.file);
      expect({ id: bundled.id, restricted: facts.restricted }).toEqual({ id: bundled.id, restricted: false });
      expect(facts.family).toBe(bundled.family);
      const listed = parseRange(bundled.coverage);
      // Every character the list claims is in the file's own cmap, and none is missing from it.
      expect(facts.coverage).toEqual(listed);
    }
  });

  it("make the registry name what a fetch downloads, until it has", async () => {
    const { registry: fonts } = registry();
    const symbols = fonts.toFetch(["lamina-sans", "noto-sans-symbols-2", "helvetica"]);
    expect(symbols.map((f) => [f.family, f.bytes])).toEqual([
      ["Lamina Sans", expect.any(Number)],
      ["Noto Sans Symbols 2", expect.any(Number)],
    ]);
    expect(symbols.find((f) => f.id === "noto-sans-symbols-2")?.bytes).toBeGreaterThan(500_000);
    await fonts.bytes("noto-sans-symbols-2");
    expect(fonts.toFetch(["noto-sans-symbols-2"])).toEqual([]);
  });
});
