---
title: "A Cloudflare Worker"
description: "A Cloudflare Worker that returns a one-page letter per request, in a standard 14 font or a bundled one, and a 400 for a name the font can't draw."
version: "0.3.0"
url: "https://docs.laminapdf.com/recipes/cloudflare-worker/"
---

# A Cloudflare Worker

A Cloudflare Worker that returns a one-page letter per request, in a standard 14 font or a bundled one, and a 400 for a name the font can't draw.

You'll make a Cloudflare Worker that returns a one-page letter to the name in its URL. You need this to make PDFs on request, such as tickets or receipts, without running a server of your own.

The Cloudflare Worker draws the letter in Helvetica at `/?name=` and in a bundled Noto Sans at `/embedded?name=`, then returns the PDF:

`packages/sdk/examples/worker/src/index.ts`:

```ts
// A Worker that generates a one-page letter per request: at `/?name=` in a standard font, which
// needs no font file, and at `/embedded?name=` in a font bundled with the Worker, which the PDF
// embeds. A name the font can't draw gets a 400 with the error as JSON. The bundled font is a file
// in `src/fonts/` that Wrangler bundles as bytes (wrangler.toml's `rules`); put the fonts your
// documents need there.
import { Builder, Font, init, isLaminaPdfError, type LaminaPdfError } from "@laminapdf/sdk";
import notoSans from "./fonts/NotoSans-Regular.ttf";

/** The longest `?name=` the letter takes. */
const MAX_NAME = 100;

/** The letter's font for each path: Helvetica, a standard 14 font, or the bundled Noto Sans. */
const FONTS = new Map([
  ["/", Font.standard("Helvetica")],
  ["/embedded", Font.fromBytes(notoSans)],
]);

/** A one-page letter to `name` in `font`. */
function letter(name: string, font: Font): Uint8Array<ArrayBuffer> {
  const b = new Builder({ producer: "lamina Worker example" });
  try {
    const page = b.page("A4");
    page.surface().text(`Hello, ${name}!`, { font, size: 24, at: [72, 96] });
    page.finish();
    return saveAndFree(b);
  } finally {
    // A no-op after finish(); when drawing threw, it frees the builder's WebAssembly side now
    // rather than whenever the garbage collector gets to it (an isolate serves many requests).
    b.free();
  }
}

/** Finishes `b`, saves the document and frees it (the bytes are a copy in JS memory). */
function saveAndFree(b: Builder): Uint8Array<ArrayBuffer> {
  const doc = b.finish();
  try {
    return doc.save();
  } finally {
    doc.free();
  }
}

const pdf = (bytes: Uint8Array<ArrayBuffer>): Response =>
  new Response(bytes, { headers: { "content-type": "application/pdf" } });

/**
 * Whether `e` says the font can't draw the name: the caller's input, not a server fault. Other
 * errors, such as `invalid-font` for a broken font file, stay a 500.
 */
function isNameError(e: unknown): e is LaminaPdfError {
  return isLaminaPdfError(e, "missing-glyph") || isLaminaPdfError(e, "encoding-full");
}

/** A 400 with the error's ADR 0026 object as JSON. */
const badRequest = (body: unknown): Response => Response.json(body, { status: 400 });

// A default export because that is how a Worker module exposes its handlers (the named-exports
// rule is for package code).
export default {
  async fetch(request: Request): Promise<Response> {
    // Loads both modules once per isolate; later requests return at once.
    await init({ text: true });
    const url = new URL(request.url);
    const font = FONTS.get(url.pathname);
    if (font === undefined) return new Response("not found", { status: 404 });
    const name = url.searchParams.get("name") ?? "world";
    if (name.length > MAX_NAME) return badRequest({ message: `name is longer than ${MAX_NAME} characters` });
    try {
      return pdf(letter(name, font));
    } catch (e) {
      // A script the font lacks (`missing-glyph`: Chinese in either font) or, in Helvetica, more
      // distinct non-WinAnsi Latin letters than its 37 spare codes (`encoding-full`).
      if (isNameError(e)) return badRequest(e.toJSON());
      throw e;
    }
  },
};
```

Open `/?name=Ada` and your browser shows one A4 page that says "Hello, Ada!" in 24-point type. To configure Wrangler and run it, follow the [Cloudflare Workers quick start](../../pages/quick-start/workers.md).

## How it works

- `init({ text: true })` loads the WebAssembly modules the first time an isolate runs `fetch`, and returns at once after that. An isolate is one running copy of the Cloudflare Worker, which serves many requests.
- `FONTS` maps each path to its font, made once per isolate. Helvetica is one of the [standard 14 fonts](../../pages/reference/glossary.md#standard-14-fonts), so that PDF embeds no font. Noto Sans arrives as bytes from `src/fonts/`, and that PDF embeds only the glyphs the letter uses.
- An unknown path gets a 404, and a name longer than `MAX_NAME` characters gets a 400 before anything is drawn.
- `isNameError` turns two codes into a 400 with the error's JSON. `missing-glyph` means a character the font lacks, and `encoding-full` means more than 37 different letters outside Helvetica's Western European set. Any other error, such as a broken font file, becomes a 500.
- `letter` frees its builder in `finally`, and `saveAndFree` frees the document once its bytes are copied out, because the isolate goes on to serve other requests.

## What to change

- **The document:** draw your own pages in `letter`, and keep both `finally` blocks, so a request that fails still frees what it made.
- **Your fonts:** put the font files your documents need in `src/fonts/`, import each one and add it to `FONTS`. The quick start's Wrangler configuration bundles every `.ttf` and `.otf` file as bytes.
- **Errors the caller caused:** add a code to `isNameError` when the request can cause it, such as `text-overflow` for a text box drawn with `overflow: "error"`. See [Errors and warnings](../../pages/concepts/errors-and-warnings.md).
- **A download name:** add `content-disposition: attachment; filename="letter.pdf"` to the headers in `pdf`, so browsers save the file instead of showing it.

Next: [Troubleshooting JavaScript and TypeScript](../../pages/troubleshooting/javascript.md), for the problems people hit most and how to fix them.
