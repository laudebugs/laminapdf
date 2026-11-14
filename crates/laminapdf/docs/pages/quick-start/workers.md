---
title: "Cloudflare Workers"
description: "Generate PDFs per request in a Cloudflare Worker."
version: "0.3.0"
url: "https://docs.laminapdf.com/quick-start/workers/"
---

# Cloudflare Workers

Generate PDFs per request in a Cloudflare Worker.

Cloudflare Workers can't compile WebAssembly at run time. So Wrangler bundles the package's entry for Cloudflare Workers, which imports both `.wasm` files as `WebAssembly.Module`s. `init` instantiates them once per isolate.

This Cloudflare Worker returns a one-page letter at `/?name=…` in a standard font, and the same letter at `/embedded?name=…` in a font bundled with the Cloudflare Worker. Lamina PDF's tests run it in workerd, Cloudflare's runtime:

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

Its Wrangler configuration bundles font files as `Data` modules, so importing a font gives its bytes:

`packages/sdk/examples/worker/wrangler.toml`:

```toml
# A Cloudflare Worker that generates a PDF per request with @laminapdf/sdk. Run `npx wrangler dev`
# in this folder. Wrangler bundles the package's `workerd` export condition, which imports both
# `.wasm` files as `WebAssembly.Module`s (Workers can't compile WebAssembly at run time).
name = "lamina-pdf-example"
main = "src/index.ts"
compatibility_date = "2026-10-01"

# Font files are bundled with the Worker as ArrayBuffers.
[[rules]]
type = "Data"
globs = ["**/*.ttf", "**/*.otf"]
fallthrough = true
```

TypeScript learns what a font import is from a declaration file next to the code:

`packages/sdk/examples/worker/src/data.d.ts`:

```ts
// Font files import as their bytes: Wrangler bundles them as `Data` modules (wrangler.toml's
// `rules`), and this file tells TypeScript what such an import is.
declare module "*.ttf" {
  const data: ArrayBuffer;
  export default data;
}
declare module "*.otf" {
  const data: ArrayBuffer;
  export default data;
}
```

To run it, install `@laminapdf/sdk` and `wrangler` in the Cloudflare Worker's folder (`npm install @laminapdf/sdk@0.3.0 wrangler`), put a copy of [Noto Sans](https://fonts.google.com/noto/specimen/Noto+Sans) at `src/fonts/NotoSans-Regular.ttf`, and run `npx wrangler dev`.

- **Errors the caller caused** become a `400` with the error's JSON: a name the font can't draw (`missing-glyph`, `encoding-full`). Anything else, such as a broken font file, stays a `500`. See [Errors and warnings](../../pages/concepts/errors-and-warnings.md).
- **Memory:** the letter frees its builder and document as soon as the bytes are saved, because one isolate serves many requests.

Next: the [Cloudflare Worker recipe](../../pages/recipes/cloudflare-worker.md) says what to change in it, and [Troubleshooting](../../pages/troubleshooting/javascript.md#init-fails-in-a-cloudflare-worker) covers a Cloudflare Worker that can't load the WebAssembly.
